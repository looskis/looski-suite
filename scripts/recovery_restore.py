"""Restore an authenticated snapshot into fresh, isolated Docker resources only."""
import hashlib
import json
import os
import re
import secrets
import subprocess
import tempfile
import time
from pathlib import Path
from recovery import Installation, inventory, queue_signature, CORE_SECRETS, PGMQ_TABLES, verify_migrations, role_memberships
from dev import ROOT
import recovery_archive as encryption
import recovery_bundle as bundle
import recovery_images as image_bundle


def literal(value):
    return "'"+value.replace("'","''")+"'"


def role_recipe(sql):
    """Keep pg_dumpall attributes/grants; make its role creation idempotent.

    PostgreSQL emits one CREATE ROLE per simple identifier. Unusual multi-line
    role names are rejected, not interpreted using an ad-hoc SQL parser.
    """
    names=[];remaining=[]
    for line in sql.splitlines():
        if line.startswith('CREATE ROLE '):
            match=re.fullmatch(r'CREATE ROLE ([a-z_][a-z0-9_]*|"(?:[^"\r\n]|"")+");',line)
            if not match:raise ValueError('Unsupported role name in recovery recipe')
            name=match.group(1)
            names.append(name[1:-1].replace('""','"') if name.startswith('"') else name)
        else:remaining.append(line)
    if not names:raise ValueError('Empty role recovery recipe')
    prefix="SET standard_conforming_strings=on;\nSET log_statement='none';\nSET log_min_error_statement='panic';\n"
    prefix+="SELECT format('CREATE ROLE %I',name) FROM jsonb_array_elements_text("+literal(json.dumps(names))+"::jsonb) AS names(name) WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname=name)\n\\gexec\n"
    return prefix+'\n'.join(remaining)+'\n'


def restore_memberships(installation,memberships):
    def identifier(value):
        if not isinstance(value,str) or not value or len(value)>63:raise ValueError('Invalid recovery database role')
        return '"'+value.replace('"','""')+'"'
    # GRANTED BY must already hold ADMIN authority. Catalog name order can
    # put a downstream grant before its grantor, so replay grant chains from
    # superusers outward before executing the transactional reconciliation.
    superusers=set(json.loads(installation.sql("SELECT json_agg(rolname) FROM pg_roles WHERE rolsuper")))
    remaining=list(memberships);ordered=[];administrators=set()
    while remaining:
        ready=[m for m in remaining if m['grantor'] in superusers or (m['role'],m['grantor']) in administrators]
        if not ready:raise ValueError('Unsupported cyclic or incomplete role grant authority')
        for membership in ready:
            ordered.append(membership);remaining.remove(membership)
            if membership['admin']:administrators.add((membership['role'],membership['member']))
    statements=[]
    for membership in ordered:
        options=[]
        for option in ('admin','inherit','set'):
            value=membership[option]
            if type(value) is not bool:raise ValueError('Invalid role grant option')
            options.append(option.upper()+' '+str(value).upper())
        statements.append('GRANT '+identifier(membership['role'])+' TO '+identifier(membership['member'])+' WITH '+', '.join(options)+' GRANTED BY '+identifier(membership['grantor'])+';')
    installation.sql("""BEGIN;
DO $roles$ DECLARE membership record; BEGIN
 FOR membership IN SELECT r.rolname AS role,m.rolname AS member,g.rolname AS grantor
 FROM pg_auth_members a JOIN pg_roles r ON r.oid=a.roleid JOIN pg_roles m ON m.oid=a.member JOIN pg_roles g ON g.oid=a.grantor
 LOOP EXECUTE format('REVOKE %I FROM %I GRANTED BY %I CASCADE',membership.role,membership.member,membership.grantor); END LOOP;
END $roles$;
"""+'\n'.join(statements)+'\nCOMMIT;',user='supabase_admin')


def write_private(path,data):
    path=Path(path);path.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
    descriptor=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
    with os.fdopen(descriptor,'wb') as stream:stream.write(data if isinstance(data,bytes) else data.encode())


def restore(archive,identity,project,port):
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]{0,62}',project) or not 1024<=port<=65535:
        raise ValueError('Use a valid new installation name and loopback port')
    target=ROOT/'.local'/'recoveries'/project
    if target.exists():raise FileExistsError('Recovery target already exists; choose a new name')
    # Existing containers/volumes of any state must not be reused or overwritten.
    for kind in ['container','volume']:
        existing=subprocess.check_output(['docker',kind,'ls','-q',*(['-a'] if kind=='container' else []),'--filter','label=com.docker.compose.project='+project],text=True).strip()
        if existing:raise ValueError('Recovery target has existing Docker resources')
    started=time.monotonic()
    with tempfile.TemporaryDirectory(prefix='looski-restore-') as temporary:
        plaintext=Path(temporary)/'snapshot.tar';encryption.decrypt(archive,plaintext,identity)
        manifest=bundle.inspect(plaintext);metadata=manifest['metadata']
        if metadata['schema_version'] not in (23,28,29,30,31,32,33,34,35,36) or not metadata['postgres_version_num'].startswith('17'):
            raise ValueError('Unsupported recovery schema or PostgreSQL version')
        if metadata['configuration']['COMPOSE_PROJECT_NAME']==project:raise ValueError('Recovery must use a different installation name')
        images=metadata['images']
        image_bundle.validate(images)
        target.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
        bundle.extract(plaintext,target)
    images=image_bundle.load(images,target/'images.tar.gz')
    recipe=target/'recipe'
    values={'COMPOSE_PROJECT_NAME':project,'LOOSKI_HTTP_PORT':str(port),'LOOSKI_PUBLIC_URL':'http://localhost:'+str(port),'LOOSKI_MAIL_MODE':'disabled','LOOSKI_AUTH_MAIL_ENABLED':'false','LOOSKI_DIAGNOSTICS_MODE':'disabled'}
    installation=Installation(values,recipe)
    restored_secrets=json.loads((target/'secrets.json').read_text())
    for name in CORE_SECRETS:
        value=secrets.token_hex(32)+'\n' if name=='jwt_secret' else restored_secrets['core'][name]
        write_private(installation.state/'secrets'/name,value)
    # Public callbacks use the rehearsal origin; identity records retain their
    # original issuer. Rotating the signing key prevents snapshot-era sessions
    # from authenticating, without rewriting stable identity links by email.
    issuer=metadata['configuration'].get('LOOSKI_AUTH_ISSUER',metadata['configuration']['LOOSKI_PUBLIC_URL']+'/auth/v1')
    installation.values['LOOSKI_AUTH_ISSUER']=issuer
    installation.environment['LOOSKI_AUTH_ISSUER']=issuer
    override={'networks':{'default':{'internal':True}},'services':{service:{'image':image,'pull_policy':'never'} for service,image in images.items()}}
    override['services'].setdefault('auth',{}).setdefault('environment',{})['GOTRUE_JWT_ISSUER']=issuer
    override['services'].setdefault('api',{}).setdefault('environment',{})['LOOSKI_AUTH_ISSUER']=issuer
    override['services'].setdefault('worker',{})['profiles']=['recovery-manual-worker']
    override['services']['worker'].setdefault('environment',{})['LOOSKI_AUTH_ISSUER']=issuer
    override_path=target/'compose.recovery.json';write_private(override_path,json.dumps(override))
    installation.compose+=['-f',str(override_path)]
    configured=subprocess.run(['python3',str(recipe/'scripts/dev.py'),'configure'],cwd=recipe,env=installation.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if configured.returncode:raise RuntimeError('Recovery secret/configuration bootstrap failed')
    report={'state':'restoring','source_captured_at':metadata['captured_at'],'project':project,'port':port,'source_archive_sha256':bundle.digest(archive)}
    report_path=target/'recovery-report.json';write_private(report_path,json.dumps(report))
    try:
        installation.run('up','-d','--no-deps','--no-build','--wait','db')
        networks=json.loads(subprocess.check_output(['docker','network','inspect',project+'_default'],text=True))
        if not networks[0]['Internal']:raise RuntimeError('Recovery network is not isolated')
        sql=role_recipe((target/'roles.sql').read_text())
        report["phase"]="restore_roles";report_path.write_text(json.dumps(report))
        installation.sql(sql,user="supabase_admin")
        restore_memberships(installation,metadata['role_memberships'])
        # CREATE from the archive restores into a new postgres database. Only
        # the fresh isolated target exists; original volumes are never touched.
        report['phase']='restore_database';report_path.write_text(json.dumps(report))
        # The image's background workers/health probes may reconnect to its
        # empty bootstrap database. Fence connections before removing only this
        # new target database, then let pg_restore recreate its captured settings.
        installation.run('exec','-T','db','psql','-U','supabase_admin','-d','template1','-v','ON_ERROR_STOP=1','-c','ALTER DATABASE postgres ALLOW_CONNECTIONS false','-c','DROP DATABASE postgres WITH (FORCE)')
        with (target/'database.dump').open('rb') as source:
            installation.run('exec','-T','db','pg_restore','-U','supabase_admin','--create','--section=pre-data','--no-acl','--exit-on-error','--dbname=template1',source=source)
        report['phase']='restore_pgmq';report_path.write_text(json.dumps(report))
        installation.sql("SELECT pgmq.create('looski_jobs')")
        if queue_signature(installation)!=metadata['pgmq']['columns']:raise RuntimeError('PGMQ column layout is incompatible with the snapshot')
        installation.sql('TRUNCATE pgmq.meta,pgmq.q_looski_jobs,pgmq.a_looski_jobs',user='supabase_admin')
        for name,table in PGMQ_TABLES.items():
            with (target/name).open('rb') as source:installation.run('exec','-T','db','psql','-U','supabase_admin','-d','postgres','-q','-v','ON_ERROR_STOP=1','-c','COPY '+table+' FROM STDIN WITH (FORMAT binary)',source=source)
        sequence=metadata['pgmq']['sequence']
        if type(sequence['last_value']) is not int or sequence['last_value']<1 or type(sequence['is_called']) is not bool:raise ValueError('Invalid PGMQ sequence state')
        installation.sql("SELECT setval('pgmq.q_looski_jobs_msg_id_seq',"+str(sequence['last_value'])+","+str(sequence['is_called']).lower()+")",user='supabase_admin')
        for section in ['data','post-data']:
            with (target/'database.dump').open('rb') as source:installation.run('exec','-T','db','pg_restore','-U','supabase_admin','--section='+section,'--exit-on-error','--dbname=postgres',source=source)
        # Schema ACL entries belong to pre-data in PostgreSQL's TOC. They
        # could not run before extension-owned queue tables existed. Apply the
        # complete ACL subset now, including Auth schema usage for its definer
        # functions; post-data alone does not restore these privileges.
        with (target/'database.dump').open('rb') as source:
            toc=installation.run('exec','-T','db','pg_restore','--list',source=source).decode()
        acl='\n'.join(line for line in toc.splitlines() if ' ACL ' in line)+'\n'
        with tempfile.TemporaryFile() as source:
            source.write(acl.encode());source.seek(0)
            installation.run('exec','-T','db','sh','-c','umask 077; cat > /tmp/looski-recovery-acl.list',source=source)
        try:
            with (target/'database.dump').open('rb') as source:
                installation.run('exec','-T','db','pg_restore','-U','supabase_admin','--use-list=/tmp/looski-recovery-acl.list','--exit-on-error','--dbname=postgres',source=source)
        finally:installation.run('exec','-T','db','rm','-f','/tmp/looski-recovery-acl.list')
        report['phase']='verify_inventory' ;report_path.write_text(json.dumps(report))
        verify_migrations(installation,recipe)
        if 'role_memberships' in metadata and role_memberships(installation)!=metadata['role_memberships']:raise RuntimeError('Restored database role memberships differ from the snapshot')
        restored_counts=inventory(installation)
        if restored_counts!=metadata['table_counts']:raise RuntimeError('Restored table inventory does not match the authenticated snapshot')
        invalid='0' if metadata['schema_version']==23 else installation.sql("SELECT count(*) FROM files.files f LEFT JOIN files.contents c ON c.workspace_id=f.workspace_id AND c.file_id=f.id WHERE c.file_id IS NULL OR f.byte_length<>octet_length(c.content) OR f.sha256<>encode(pg_catalog.sha256(c.content),'hex')")
        if invalid!='0':raise RuntimeError('Restored file content failed integrity checks')
        for name,table in PGMQ_TABLES.items():
            with tempfile.TemporaryFile() as check:
                installation.run('exec','-T','db','psql','-U','supabase_admin','-d','postgres','-q','-v','ON_ERROR_STOP=1','-c','COPY (SELECT * FROM '+table+' ORDER BY '+('queue_name' if table=='pgmq.meta' else 'msg_id')+') TO STDOUT WITH (FORMAT binary)',output=check)
                check.seek(0);digest=hashlib.sha256()
                for chunk in iter(lambda:check.read(1024*1024),b''):digest.update(chunk)
                if digest.hexdigest()!=manifest['entries'][name]['sha256']:raise RuntimeError('Restored PGMQ data does not match the snapshot')
        report['phase']='quarantine';report_path.write_text(json.dumps(report))
        pending=json.loads(installation.sql("SELECT coalesce(jsonb_agg(jsonb_build_object('id',id,'workspace_id',workspace_id,'kind',kind,'state',state,'attempts',attempts)),'[]') FROM suite.jobs WHERE state IN ('queued','running','blocked','exhausted')"))
        # No pending external effect may execute simply because an old snapshot
        # has forgotten its completion. Existing job-admin resume remains explicit.
        installation.sql("""BEGIN;
DELETE FROM pgmq.q_looski_jobs;
WITH changed AS (UPDATE suite.jobs SET state='blocked',error_code='unavailable',message_id=NULL,lease_token=NULL,lease_until=NULL,version=version+1,updated_at=now() WHERE state IN ('queued','running','blocked','exhausted') RETURNING *)
INSERT INTO suite.job_events(workspace_id,job_id,state,attempt,error_code) SELECT workspace_id,id,state,attempts,error_code FROM changed;
DELETE FROM auth.refresh_tokens; DELETE FROM auth.sessions; DELETE FROM auth.one_time_tokens;
UPDATE auth.users SET confirmation_token='',recovery_token='',email_change_token_current='',email_change_token_new='',phone_change_token='',reauthentication_token='';
COMMIT;""",user="supabase_admin")
        report.update(state='verified_isolated',verified_tables=len(restored_counts),verified_files=restored_counts.get('files.contents',0),quarantined_jobs=pending,elapsed_seconds=round(time.monotonic()-started,3),session_policy='signing key rotated; sessions, refresh tokens and pending one-time links invalidated',workers='disabled',mail='disabled',routing='not switched')
        report_path.write_text(json.dumps(report,indent=2));report_path.chmod(0o600)
        for name in ('database.dump','roles.sql','secrets.json','images.tar.gz',*PGMQ_TABLES):(target/name).unlink()
        return {'target':str(target),'report':str(report_path),'verified_tables':report['verified_tables'],'verified_files':report['verified_files'],'quarantined_jobs':len(pending),'elapsed_seconds':report['elapsed_seconds']}
    except BaseException:
        report['state']='failed_isolated';report_path.write_text(json.dumps(report));report_path.chmod(0o600)
        raise


def preview(target):
    """Expose only a verified rehearsal on loopback; no workers or outbound mail."""
    target=Path(target).resolve()
    if target.parent != (ROOT/'.local'/'recoveries').resolve():raise ValueError('Unknown recovery target')
    report_path=target/'recovery-report.json';report=json.loads(report_path.read_text())
    if report['state'] not in ('verified_isolated','preview_isolated'):raise ValueError('Recovery verification has not succeeded')
    project=report['project'];port=report['port']
    network=json.loads(subprocess.check_output(['docker','network','inspect',project+'_default'],text=True))
    if not network[0]['Internal']:raise ValueError('Recovery network must remain isolated')
    values={'COMPOSE_PROJECT_NAME':project,'LOOSKI_HTTP_PORT':str(port),'LOOSKI_PUBLIC_URL':'http://localhost:'+str(port),'LOOSKI_MAIL_MODE':'disabled','LOOSKI_AUTH_MAIL_ENABLED':'false','LOOSKI_DIAGNOSTICS_MODE':'disabled'}
    # Docker Desktop cannot publish a port from an internal-only network.
    # Only Caddy joins a second ingress bridge; Auth/API/DB remain on the
    # internal network and cannot make outbound provider calls.
    override_path=target/'compose.recovery.json'
    override=json.loads(override_path.read_text())
    override['networks']['recovery-preview']={}
    override['services']['gateway']['networks']=['default','recovery-preview']
    override_path.write_text(json.dumps(override))
    installation=Installation(values,target/'recipe');installation.compose+=['-f',str(target/'compose.recovery.json')]
    if 'worker' in installation.running():raise ValueError('Stop recovery workers before preview')
    installation.run('up','-d','--no-deps','--no-build','--wait','--wait-timeout','120','auth-templates','auth','supabase-gateway','api','web','gateway')
    report['state']='preview_isolated';report_path.write_text(json.dumps(report,indent=2))
    return {'url':values['LOOSKI_PUBLIC_URL'],'mail':'disabled','workers':'disabled','network':'business services internal; gateway loopback ingress'}
