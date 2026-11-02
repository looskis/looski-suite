#!/usr/bin/env python3
"""Download and verify an official preview into a fresh installation directory."""
import argparse
import hashlib
import json
import os
import re
import shutil
import tarfile
import tempfile
from pathlib import Path, PurePosixPath
from urllib.request import urlopen

REPOSITORY = 'https://github.com/looskis/looski-suite/releases/download/'


def unpack(archive_path, destination, checksum):
    if not re.fullmatch(r'[a-f0-9]{64}', checksum):
        raise ValueError('A SHA-256 checksum is required')
    digest = hashlib.sha256()
    with Path(archive_path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    if digest.hexdigest() != checksum:
        raise ValueError('Release checksum mismatch; nothing installed')
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError('Destination must not exist; existing installations are never overwritten')
    with tarfile.open(archive_path, 'r:gz') as archive:
        seen = set()
        total = 0
        for member in archive:
            name = PurePosixPath(member.name)
            total += member.size
            if (not member.isfile() or name.is_absolute() or str(name) != member.name
                    or any(part in ('.', '..', '.local', '.env', '.env.dev') for part in name.parts)
                    or member.name in seen or member.size > 32 * 1024 * 1024
                    or total > 256 * 1024 * 1024 or len(seen) >= 20000):
                raise ValueError('Unsafe or oversized release archive')
            seen.add(member.name)
        if not {'release.json', 'compose.yaml', 'looski-admin', 'scripts/install.py'}.issubset(seen):
            raise ValueError('Incomplete runtime release')
        manifest = json.load(archive.extractfile('release.json'))
        if manifest.get('format') != 'looski-release':
            raise ValueError('Unsupported release format')
        # Extract into a temporary sibling, then publish only a complete directory.
        with tempfile.TemporaryDirectory(prefix='.looski-download-', dir=destination.parent) as temporary:
            stage = Path(temporary) / 'installation'
            stage.mkdir(mode=0o700)
            for member in archive.getmembers():
                target = stage / member.name
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open('xb') as output, archive.extractfile(member) as source:
                    shutil.copyfileobj(source, output)
                target.chmod(0o755 if member.name == 'looski-admin' else 0o644)
            # Atomic no-clobber directory reservation; an incomplete interrupted move
            # is never accepted as an install. Files are then moved into that reservation.
            destination.mkdir(mode=0o700)
            for path in stage.iterdir():
                shutil.move(str(path), destination / path.name)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--sha256', help='Expected checksum from a separately verified release, if available')
    args = parser.parse_args()
    if not re.fullmatch(r'v\d+\.\d+\.\d+-preview\.\d+', args.version):
        raise ValueError('Select an explicit preview version')
    base = REPOSITORY + args.version + '/'
    filename = 'looski-suite-' + args.version + '.tar.gz'
    with urlopen(base + 'SHA256SUMS', timeout=30) as response:
        entries = dict(line.split()[::-1] for line in response.read(16384).decode().splitlines())
    checksum = args.sha256 or entries[filename]
    with tempfile.TemporaryDirectory(prefix='looski-fetch-') as temporary:
        archive = Path(temporary) / filename
        with urlopen(base + filename, timeout=60) as response, archive.open('xb') as output:
            size = 0
            while chunk := response.read(1024 * 1024):
                size += len(chunk)
                if size > 128 * 1024 * 1024:
                    raise ValueError('Release download exceeds size limit')
                output.write(chunk)
        manifest = unpack(archive, args.destination, checksum)
    print(json.dumps({'directory': str(args.destination.absolute()), 'release': manifest['version'],
                      'next': str(args.destination.absolute() / 'looski-admin') + ' install'}))


if __name__ == '__main__':
    main()
