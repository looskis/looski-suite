"""Portable, pinned image payloads; never depend on mutable registry tags."""
import gzip
import hashlib
import json
import re
import secrets
import subprocess
import tarfile

OPTIONAL_SERVICES=('clickhouse',)
SERVICES=('db','auth','api','web','gateway','auth-templates','supabase-gateway','worker','migrate')
DIGEST=r'sha256:[0-9a-f]{64}'
REFERENCE=r'looski-recovery-bundle/[0-9a-f]{32}-[0-9a-f]{64}:snapshot'


def validate_ids(images):
    if not isinstance(images,dict) or not set(SERVICES).issubset(images) or set(images)-set((*SERVICES,*OPTIONAL_SERVICES)) or any(not isinstance(value,str) or not re.fullmatch(DIGEST,value) for value in images.values()):
        raise ValueError('Missing pinned recovery service image; create every required service before backup')


def validate(images):
    if not isinstance(images,dict):raise ValueError('Invalid recovery image manifest')
    validate_ids({service:value.get('source_id') if isinstance(value,dict) else None for service,value in images.items()})
    for value in images.values():
        if not isinstance(value.get('reference'),str) or not re.fullmatch(REFERENCE,value['reference']) or not isinstance(value.get('config_id'),str) or not re.fullmatch(DIGEST,value['config_id']):
            raise ValueError('Invalid portable recovery image identity')


def inspect_id(reference,command=('docker',)):
    return subprocess.check_output([*command,'image','inspect','--format','{{.Id}}',reference],text=True,stderr=subprocess.PIPE).strip()


def capture(images,destination):
    validate_ids(images)
    # Desktop's containerd store reports manifest/index IDs; a classic engine
    # reports config IDs. Unique archive tags preserve their association across
    # engines without replacing any installation's own mutable image tags.
    nonce=secrets.token_hex(16)
    references={ident:'looski-recovery-bundle/'+nonce+'-'+ident[7:]+':snapshot' for ident in set(images.values())}
    created=[]
    try:
        for ident,reference in references.items():
            if not subprocess.run(['docker','image','inspect',reference],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
                raise ValueError('Recovery image tag already exists')
            subprocess.run(['docker','image','tag',ident,reference],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            created.append((ident,reference))
        with destination.open('xb') as target:
            destination.chmod(0o600)
            process=subprocess.Popen(['docker','image','save',*sorted(references.values())],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            try:
                with gzip.GzipFile(fileobj=target,mode='wb',compresslevel=1,mtime=0) as compressed:
                    for chunk in iter(lambda:process.stdout.read(1024*1024),b''):compressed.write(chunk)
                process.stdout.close()
                if process.wait():raise RuntimeError('Could not package pinned recovery images')
            finally:
                if process.poll() is None:process.kill();process.wait()
                process.stderr.close()
        configs={}
        with tarfile.open(destination,'r:gz') as archive:
            manifest=archive.getmember('manifest.json')
            if manifest.size>1024*1024:raise ValueError('Oversized image manifest')
            for entry in json.load(archive.extractfile(manifest)):
                member=archive.getmember(entry['Config'])
                if not member.isfile() or member.size>10*1024*1024:raise ValueError('Invalid image config')
                config_id='sha256:'+hashlib.sha256(archive.extractfile(member).read()).hexdigest()
                for reference in entry.get('RepoTags') or []:
                    if reference in configs:raise ValueError('Ambiguous recovery platform image')
                    configs[reference]=config_id
        result={service:{'source_id':ident,'reference':references[ident],'config_id':configs[references[ident]]} for service,ident in images.items()}
        validate(result)
        return result
    finally:
        for ident,reference in created:
            # Remove only the temporary tag we created, and only if it still
            # names our image. The original tags and image are retained.
            if inspect_id(reference)==ident:
                subprocess.run(['docker','image','rm',reference],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)


def loaded_ids(images,command=('docker',)):
    validate(images)
    result={}
    for service,value in images.items():
        ident=inspect_id(value['reference'],command)
        if ident not in (value['source_id'],value['config_id']):raise ValueError('Recovery image identity does not match its authenticated snapshot')
        result[service]=ident
    return result


def load(images,source):
    validate(images)
    # Always exercise the portable payload, even if the host already has images.
    with source.open('rb') as archive:
        result=subprocess.run(['docker','image','load','--quiet'],stdin=archive,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if result.returncode:raise RuntimeError('Pinned recovery image import failed')
    return loaded_ids(images)
