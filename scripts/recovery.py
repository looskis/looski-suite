#!/usr/bin/env python3
"""Manual encrypted installation snapshots; no rotation, purge or routing changes."""
import argparse
import fcntl
import json
import os
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from dev import ROOT, configuration
import recovery_archive as encryption
import recovery_bundle as bundle
import recovery_images as image_bundle

WRITERS=('gateway','web','api','worker','auth')
PGMQ_TABLES={'pgmq-meta.bin':'pgmq.meta','pgmq-queue.bin':'pgmq.q_looski_jobs','pgmq-archive.bin':'pgmq.a_looski_jobs'}
CORE_SECRETS=('db_password','runtime_password','jwt_secret','database_url','migration_url')

class Installation:
    def __init__(self,values,root=ROOT):
        self.values=values;self.root=Path(root)
        self.state=self.root/'.local'/values['COMPOSE_PROJECT_NAME']
        self.environment={**os.environ,**values,'COMPOSE_PROFILES':'local-mail' if values['LOOSKI_MAIL_MODE']=='local' else ''}
        self.compose=['docker','compose','--project-directory',str(self.root),'-f',str(self.root/'compose.yaml')]
    def run(self,*arguments,source=None,output=None):
        result=subprocess.run([*self.compose,*arguments],cwd=self.root,env=self.environment,
                              stdin=source,stdout=output or subprocess.PIPE,stderr=subprocess.PIPE)
        if result.returncode:raise RuntimeError('Recovery service operation failed: '+arguments[0])
        return result.stdout
    def sql(self,sql,user="postgres"):
        with tempfile.TemporaryFile() as source:
            source.write(sql.encode());source.seek(0)
            return self.run('exec','-T','db','psql','-U',user,'-d','postgres','-At','-v','ON_ERROR_STOP=1',source=source).decode().strip()
    def running(self):return self.run('ps','--services','--status','running').decode().split()
    def image_ids(self):
        ids={}
        for service in (*image_bundle.SERVICES,*image_bundle.OPTIONAL_SERVICES):
            container=self.run('ps','-a','-q',service).decode().strip()
            if not container:continue
            ids[service]=subprocess.check_output(['docker','inspect','--format','{{.Image}}',container],text=True).strip()
        return ids

    def start_services(self,services,require_healthy=False):
        # Compose start traverses dependencies and can start services that were
        # stopped before backup. Address exactly the existing container IDs.
        containers=[]
        for service in services:
            container=self.run('ps','-a','-q',service).decode().strip()
            if not container:raise RuntimeError('Recorded service container no longer exists')
            containers.append(container)
        if not containers:return
        subprocess.run(['docker','start',*containers],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        deadline=time.monotonic()+60
        while True:
            states=[json.loads(line) for line in subprocess.check_output(['docker','inspect','--format','{{json .State}}',*containers],text=True).splitlines()]
            if all(s['Running'] and (not require_healthy or s.get('Health',{}).get('Status','healthy')=='healthy') for s in states):return
            if time.monotonic()>deadline:raise RuntimeError('Restored service did not become healthy')
            time.sleep(1)


def queue_signature(installation):
    return json.loads(installation.sql("SELECT jsonb_agg(jsonb_build_object('table',table_name,'column',column_name,'type',udt_name,'ordinal',ordinal_position) ORDER BY table_name,ordinal_position) FROM information_schema.columns WHERE table_schema='pgmq' AND table_name IN ('meta','q_looski_jobs','a_looski_jobs')"))


def inventory(installation):
    sql="""
CREATE TEMP TABLE recovery_counts(name text, rows bigint);
DO $looski_recovery$ DECLARE entry record; total bigint; BEGIN
 FOR entry IN SELECT n.nspname,c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
   WHERE c.relkind IN ('r','p') AND n.nspname !~ '^pg_' AND n.nspname<>'information_schema'
 LOOP
  EXECUTE format('SELECT count(*) FROM %I.%I',entry.nspname,entry.relname) INTO total;
  INSERT INTO recovery_counts VALUES(entry.nspname||'.'||entry.relname,total);
 END LOOP;
END $looski_recovery$;
SELECT jsonb_object_agg(name,rows) FROM recovery_counts;
"""
    return json.loads(installation.sql(sql).splitlines()[-1])


def verify_migrations(installation,recipe):
    from migration_integrity import source_checksums,validate
    source=source_checksums(Path(recipe)/'supabase/migrations')
    baseline=json.loads((Path(recipe)/'infra/migrator/baseline.json').read_text())
    applied=json.loads(installation.sql("SELECT coalesce(json_agg(version ORDER BY version),'[]') FROM supabase_migrations.schema_migrations"))
    recorded={}
    if installation.sql("SELECT to_regclass('supabase_migrations.looski_checksums') IS NOT NULL")=='t':
        recorded=json.loads(installation.sql("SELECT coalesce(json_object_agg(version,sha256),'{}') FROM supabase_migrations.looski_checksums"))
    validate(applied,recorded,source,baseline)
    return {'source':source,'applied':applied,'recorded':recorded}


def role_memberships(installation):
    return json.loads(installation.sql("SELECT coalesce(jsonb_agg(jsonb_build_object('role',r.rolname,'member',m.rolname,'grantor',g.rolname,'admin',a.admin_option,'inherit',a.inherit_option,'set',a.set_option) ORDER BY r.rolname,m.rolname,g.rolname),'[]') FROM pg_auth_members a JOIN pg_roles r ON r.oid=a.roleid JOIN pg_roles m ON m.oid=a.member JOIN pg_roles g ON g.oid=a.grantor"))


def secrets_snapshot(installation):
    result={'core':{name:encryption.private_regular(installation.state/'secrets'/name).decode() for name in CORE_SECRETS},
            'invitation_key':encryption.private_regular(installation.state/'invitation-key').decode() if (installation.state/'invitation-key').exists() else None}
    smtp=installation.state/'smtp.json'
    if smtp.exists():result['smtp']=json.loads(encryption.private_regular(smtp))
    return result


def status_write(installation,document):
    descriptor,temporary=tempfile.mkstemp(prefix='.backup-status-',dir=installation.state)
    try:
        with os.fdopen(descriptor,'w') as output:
            json.dump(document,output);output.flush();os.fsync(output.fileno())
        os.replace(temporary,installation.state/'backup-status.json')
    finally:
        if os.path.exists(temporary):os.unlink(temporary)


def recover_services(installation):
    with (installation.state/'setup.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        path=installation.state/'backup-status.json'
        if not path.exists():raise ValueError('No backup status exists')
        status=json.loads(path.read_text())
        if status.get('state') not in ('running','service_restoration_failed'):
            return {'restored':False,'state':status.get('state')}
        services=status.get('original_services')
        if not isinstance(services,list) or any(service not in WRITERS for service in services):
            raise ValueError('Invalid recorded service state')
        if services:installation.start_services(list(reversed(services)),status.get('require_healthy',False))
        status['state']='interrupted_services_restored';status_write(installation,status)
        return {'restored':True,'services':services}


def create(installation,destination,recipient):
    encryption.validate_recipient(recipient)
    destination=Path(destination).absolute()
    if destination.exists() or destination.is_symlink():raise FileExistsError('Backup destination exists')
    if not installation.state.is_dir():raise ValueError('Configure the installation before backing it up')
    # Same lock as setup/update: never capture half an applied migration.
    with (installation.state/'setup.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        prior=installation.state/'backup-status.json'
        if prior.exists() and json.loads(prior.read_text()).get('state') in ('running','service_restoration_failed'):
            raise ValueError('Interrupted backup requires recover-services before another snapshot')
        running=installation.running()
        if 'db' not in running:raise ValueError('Database must be running for a coordinated snapshot')
        images=installation.image_ids();image_bundle.validate_ids(images);stopped=[s for s in WRITERS if s in running]
        started=time.monotonic();maintenance=None;result=None;failure=None;status_failure=None
        status={'state':'running','phase':'preparing','started_at':datetime.now(timezone.utc).isoformat(),'original_services':stopped,'require_healthy':set(WRITERS).issubset(running),'destination':str(destination)}
        status_write(installation,status)
        try:
            with tempfile.TemporaryDirectory(prefix='looski-backup-',dir=installation.state) as temporary:
                directory=Path(temporary);files=bundle.freeze_recipe(installation.root,directory)
                migration_identity=verify_migrations(installation,directory/'recipe')
                # Image export is immutable and can happen before maintenance.
                image_path=directory/'images.tar.gz';images=image_bundle.capture(images,image_path);files['images.tar.gz']=image_path
                status['phase']='stopping_writers';status_write(installation,status)
                installation.run('stop','-t','30',*stopped) if stopped else None
                maintenance=time.monotonic()
                status['phase']='capturing';status_write(installation,status)
                schema=int(installation.sql('SELECT version FROM suite.schema_version WHERE singleton'))
                if schema not in (23,28):raise ValueError('This recovery runner supports schemas 23 and 28')
                postgres=installation.sql('SHOW server_version_num')
                if not postgres.startswith('17'):raise ValueError('This recovery runner requires PostgreSQL 17')
                if installation.sql("SELECT count(*) FROM pg_tablespace WHERE spcname NOT IN ('pg_default','pg_global')")!='0':raise ValueError('Custom tablespaces require a separate recovery procedure')
                counts=inventory(installation)
                queue_names=json.loads(installation.sql("SELECT coalesce(jsonb_agg(queue_name ORDER BY queue_name),'[]') FROM pgmq.meta"))
                if queue_names!=['looski_jobs']:raise ValueError('Unsupported queue inventory for this recovery version')
                queue_metadata={'columns':queue_signature(installation),'sequence':json.loads(installation.sql("SELECT jsonb_build_object('last_value',last_value,'is_called',is_called) FROM pgmq.q_looski_jobs_msg_id_seq"))}
                for name,table in PGMQ_TABLES.items():
                    path=directory/name;descriptor=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
                    with os.fdopen(descriptor,'wb') as output:installation.run('exec','-T','db','psql','-U','postgres','-d','postgres','-q','-v','ON_ERROR_STOP=1','-c','COPY (SELECT * FROM '+table+' ORDER BY '+('queue_name' if table=='pgmq.meta' else 'msg_id')+') TO STDOUT WITH (FORMAT binary)',output=output)
                    files[name]=path
                secret_path=directory/'secrets.json'
                descriptor=os.open(secret_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
                with os.fdopen(descriptor,'w') as output:json.dump(secrets_snapshot(installation),output)
                files['secrets.json']=secret_path
                for name,command in [('database.dump',['pg_dump','-U','postgres','-d','postgres','--format=custom']),('roles.sql',['pg_dumpall','-U','postgres','--roles-only'])]:
                    path=directory/name;descriptor=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
                    with os.fdopen(descriptor,'wb') as output:installation.run('exec','-T','db',*command,output=output)
                    files[name]=path
                metadata={'captured_at':datetime.now(timezone.utc).isoformat(),'configuration':installation.values,'schema_version':schema,'postgres_version_num':postgres,'images':images,'table_counts':counts,'pgmq':queue_metadata,'role_memberships':role_memberships(installation),'migration_integrity':migration_identity,'diagnostics':'excluded; PostgreSQL audit retained','external_jobs':'quarantine required before resume'}
                archive=directory/'snapshot.tar';bundle.package(archive,files,metadata);bundle.inspect(archive)
                status['phase']='encrypting';status_write(installation,status)
                encryption.encrypt(archive,destination,recipient)
                result={'archive':str(destination),'sha256':bundle.digest(destination),'encrypted_bytes':destination.stat().st_size,'captured_at':metadata['captured_at']}
        except BaseException as error:
            failure=error
        finally:
            status['phase']='restoring_services'
            try:status_write(installation,status)
            except OSError as error:status_failure=error
            # Start the existing containers; never rebuild or migrate while
            # unwinding a backup, and never start previously stopped services.
            if stopped:
                try:installation.start_services(list(reversed(stopped)),status['require_healthy'])
                except Exception as error:
                    status['state']='service_restoration_failed'
                    try:status_write(installation,status)
                    except OSError:pass
                    raise RuntimeError('Backup service restoration failed; use recover-services before retrying') from error
        if failure is not None:
            status['state']='failed_services_restored'
            try:status_write(installation,status)
            except OSError:pass
            raise failure
        if status_failure is not None:raise RuntimeError('Services restored but backup status could not be persisted; inspect available disk space') from status_failure
        result['elapsed_seconds']=round(time.monotonic()-started,3)
        result['maintenance_seconds']=round(time.monotonic()-(maintenance or started),3)
        status.update(state='complete',phase='complete',result=result);status_write(installation,status)
        return result


def verify(archive,identity):
    with tempfile.TemporaryDirectory(prefix='looski-verify-') as directory:
        plaintext=Path(directory)/'snapshot.tar'
        encryption.decrypt(archive,plaintext,identity)
        return bundle.inspect(plaintext)


def main():
    parser=argparse.ArgumentParser(description=__doc__);commands=parser.add_subparsers(dest='command',required=True)
    commands.add_parser('build-tools');commands.add_parser('status');commands.add_parser('recover-services')
    keygen=commands.add_parser('keygen');keygen.add_argument('--output',required=True,type=Path)
    backup=commands.add_parser('create');backup.add_argument('--recipient',required=True);backup.add_argument('--output',required=True,type=Path)
    check=commands.add_parser('verify');check.add_argument('--archive',required=True,type=Path);check.add_argument('--identity',required=True,type=Path)
    preview=commands.add_parser('preview');preview.add_argument('--target',required=True,type=Path)
    restore=commands.add_parser('restore');restore.add_argument('--archive',required=True,type=Path);restore.add_argument('--identity',required=True,type=Path);restore.add_argument('--project',required=True);restore.add_argument('--port',required=True,type=int)
    args=parser.parse_args()
    if args.command=='build-tools':encryption.build_tools()
    elif args.command=='status':
        path=Installation(configuration()).state/'backup-status.json'
        print(path.read_text() if path.exists() else json.dumps({'state':'never_run'}))
    elif args.command=='recover-services':print(json.dumps(recover_services(Installation(configuration()))))
    elif args.command=='keygen':print(json.dumps({'recipient':encryption.generate_identity(args.output),'identity_file':str(args.output)}))
    elif args.command=='create':print(json.dumps(create(Installation(configuration()),args.output,args.recipient)))
    elif args.command=='preview':
        from recovery_restore import preview
        print(json.dumps(preview(args.target)))
    elif args.command=='restore':
        from recovery_restore import restore
        print(json.dumps(restore(args.archive,args.identity,args.project,args.port)))
    elif args.command=='verify':
        manifest=verify(args.archive,args.identity)
        print(json.dumps({'verified':True,'metadata':manifest['metadata'],'member_count':len(manifest['entries'])}))
if __name__=='__main__':
    import signal
    def interrupted(signum,frame):raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM,interrupted)
    try:main()
    except KeyboardInterrupt:raise SystemExit('Recovery operation interrupted; inspect its status before retrying')
    except (OSError,ValueError,RuntimeError,subprocess.CalledProcessError) as error:
        raise SystemExit(str(error))
