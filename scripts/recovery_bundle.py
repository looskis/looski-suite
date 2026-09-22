"""Versioned recovery bundles with bounded, regular-file-only extraction."""
import hashlib
import json
import os
import stat
import shutil
import tarfile
from pathlib import Path, PurePosixPath

FORMAT='looski-recovery'
VERSION=5
# Deliberately exclude mutable installation data and unrelated research.
RECIPE_ROOTS=('Cargo.toml','Cargo.lock','package.json','pnpm-lock.yaml','pnpm-workspace.yaml',
              'compose.yaml','Makefile','LICENSE','NOTICE','TRADEMARKS.md','README.md','.dockerignore',
              '.env.example','.prettierrc.json','.prettierignore','rust-toolchain.toml',
              'crates','apps','infra','scripts','supabase','contracts','docs')
EXCLUDED={'.git','.local','.agents','.codex','.aws','node_modules','.svelte-kit','build',
          'target','__pycache__','.temp','.branches','research','.DS_Store'}


def digest(path):
    result=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):result.update(chunk)
    return result.hexdigest()


def recipe_files(root):
    root=Path(root);files={}
    for name in RECIPE_ROOTS:
        source=root/name
        if not source.exists():continue
        paths=[source] if source.is_file() else source.rglob('*')
        for path in paths:
            relative=path.relative_to(root)
            if any(part in EXCLUDED for part in relative.parts):continue
            if path.is_symlink():raise ValueError('Recovery recipe cannot include symlinks')
            if path.is_dir():continue
            if not path.is_file() or path.stat().st_size>10*1024*1024:
                raise ValueError('Unsupported recovery recipe file')
            files['recipe/'+relative.as_posix()]=path
    return files


def freeze_recipe(root,destination):
    files={}
    for name,path in recipe_files(root).items():
        target=Path(destination)/name
        target.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
        with path.open('rb') as source,target.open('xb') as output:
            target.chmod(0o600);shutil.copyfileobj(source,output)
        files[name]=target
    return files


def valid_name(name):
    path=PurePosixPath(name)
    if path.is_absolute() or str(path)!=name or any(p in ('..','.') for p in path.parts):return False
    if name in ('database.dump','roles.sql','secrets.json','pgmq-meta.bin','pgmq-queue.bin','pgmq-archive.bin','images.tar.gz'):return True
    if not name.startswith('recipe/'):return False
    parts=path.parts[1:]
    return bool(parts) and parts[0] in RECIPE_ROOTS and not any(p in EXCLUDED for p in parts)


def package(destination,files,metadata):
    if not files or any(not valid_name(name) for name in files):raise ValueError('Unsupported recovery member')
    entries={name:{'size':Path(path).stat().st_size,'sha256':digest(path)} for name,path in sorted(files.items())}
    manifest={'format':FORMAT,'format_version':VERSION,'metadata':metadata,'entries':entries}
    encoded=json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()
    if len(encoded)>4*1024*1024:raise ValueError('Recovery manifest too large')
    import io
    with tarfile.open(destination,'w') as archive:
        info=tarfile.TarInfo('manifest.json');info.size=len(encoded);info.mode=0o600
        archive.addfile(info,io.BytesIO(encoded))
        for name,path in sorted(files.items()):
            info=tarfile.TarInfo(name);info.size=entries[name]['size'];info.mode=0o600
            with Path(path).open('rb') as stream:archive.addfile(info,stream)
    os.chmod(destination,0o600)
    return manifest


def inspect(source):
    """Read every payload byte and verify it before any extraction or restore."""
    total_size=Path(source).stat().st_size
    with tarfile.open(source,'r:') as archive:
        members={}
        for member in archive:
            if len(members)>=10000 or member.name in members or not member.isfile() or member.size<0 or member.size>total_size:
                raise ValueError('Invalid or duplicate recovery member')
            if member.name!='manifest.json' and not valid_name(member.name):raise ValueError('Unsafe recovery path')
            members[member.name]=member
        if 'manifest.json' not in members or members['manifest.json'].size>4*1024*1024:raise ValueError('Missing or oversized recovery manifest')
        manifest=json.load(archive.extractfile(members['manifest.json']))
        if not isinstance(manifest,dict) or manifest.get('format')!=FORMAT or manifest.get('format_version')!=VERSION:raise ValueError('Unsupported recovery format')
        entries=manifest.get('entries')
        if not isinstance(entries,dict) or set(entries)!=set(members)-{'manifest.json'}:raise ValueError('Recovery inventory mismatch')
        if not {'database.dump','roles.sql','secrets.json','pgmq-meta.bin','pgmq-queue.bin','pgmq-archive.bin','images.tar.gz'}.issubset(entries):raise ValueError('Incomplete recovery bundle')
        for name,entry in entries.items():
            if not isinstance(entry,dict) or members[name].size!=entry.get('size'):raise ValueError('Recovery size mismatch')
            checksum=hashlib.sha256()
            with archive.extractfile(members[name]) as stream:
                for chunk in iter(lambda:stream.read(1024*1024),b''):checksum.update(chunk)
            if checksum.hexdigest()!=entry.get('sha256'):raise ValueError('Recovery checksum mismatch')
        return manifest


def extract(source,destination):
    manifest=inspect(source)
    destination=Path(destination)
    destination.mkdir(mode=0o700) # Fresh destination only.
    with tarfile.open(source,'r:') as archive:
        for name in manifest['entries']:
            target=destination/name;target.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
            # Source bind mounts must remain readable to non-root container users.
            # The surrounding recovery root is private; only secrets stay 0600.
            if name.startswith('recipe/'):
                for parent in target.parents:
                    if parent==destination:break
                    parent.chmod(0o755)
            descriptor=os.open(target,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644 if name.startswith('recipe/') else 0o600)
            with os.fdopen(descriptor,'wb') as output,archive.extractfile(name) as source_file:
                for chunk in iter(lambda:source_file.read(1024*1024),b''):output.write(chunk)
    return manifest
