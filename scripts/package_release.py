#!/usr/bin/env python3
"""Build a source-inclusive, digest-pinned runtime bundle from a committed revision."""
import argparse
import hashlib
import json
import re
import subprocess
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CUSTOM = {'api': 'api', 'worker': 'api', 'owner-setup': 'api', 'web': 'web', 'auth': 'auth', 'migrate': 'migrate'}
DIGEST_REFERENCE = r'[a-z0-9][a-z0-9./:_-]*@sha256:[0-9a-f]{64}'


def pin_compose(document, images):
    for service, config in document['services'].items():
        reference = images.get(service)
        if not isinstance(reference, str) or not re.fullmatch(DIGEST_REFERENCE, reference):
            raise ValueError('Missing digest-pinned image for ' + service)
        config.pop('build', None)
        config['image'] = reference
    return document


def resolve(reference):
    output = subprocess.check_output(['docker', 'buildx', 'imagetools', 'inspect', reference,
                                     '--format', '{{json .Manifest}}'], text=True)
    manifest = json.loads(output)
    platforms = {(entry.get('platform', {}).get('os'), entry.get('platform', {}).get('architecture'))
                 for entry in manifest.get('manifests', [])}
    if not {('linux', 'amd64'), ('linux', 'arm64')}.issubset(platforms):
        raise ValueError('Release image must support both linux/amd64 and linux/arm64: ' + reference)
    digest = manifest['digest']
    result = reference.split('@')[0] + '@' + digest
    if not re.fullmatch(DIGEST_REFERENCE, result):
        raise ValueError('Invalid registry digest')
    return result


def package(version, registry, output):
    if not re.fullmatch(r'v\d+\.\d+\.\d+(?:-[a-z0-9.-]+)?', version):
        raise ValueError('Use a version such as v0.1.0-preview.1')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    subprocess.run(['git', 'diff', '--exit-code', 'HEAD', '--'], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    output.mkdir(parents=True, exist_ok=True)
    source = output / ('looski-source-' + version + '.tar.gz')
    subprocess.run(['git', 'archive', '--format=tar.gz', '--output', str(source.resolve()), 'HEAD'], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix='looski-release-') as temporary:
        stage = Path(temporary)
        # git archive is generated locally from the checked revision, not user input.
        with tarfile.open(source) as archive:
            for member in archive:
                if not member.isfile() and not member.isdir():
                    raise ValueError('Release source must not contain symlinks')
            archive.extractall(stage, filter='data')
        raw = subprocess.check_output(['docker', 'compose', '--profile', '*', '-f', str(stage / 'compose.yaml'),
                                       'config', '--no-interpolate', '--no-path-resolution', '--format', 'json'], cwd=stage, text=True)
        document = json.loads(raw)
        images = {}
        cache = {}
        for service, config in document['services'].items():
            reference = registry + '-' + CUSTOM[service] + ':' + revision if service in CUSTOM else config['image']
            if reference not in cache:
                cache[reference] = resolve(reference)
            images[service] = cache[reference]
        recovery = resolve(registry + '-recovery:' + revision)
        (stage / 'compose.yaml').write_text(json.dumps(pin_compose(document, images), indent=2) + '\n')
        manifest = {'format': 'looski-release', 'version': version, 'revision': revision,
                    'images': images, 'recovery_image': recovery, 'platforms': ['linux/amd64', 'linux/arm64']}
        (stage / 'release.json').write_text(json.dumps(manifest, indent=2) + '\n')
        bundle = output / ('looski-suite-' + version + '.tar.gz')
        with tarfile.open(bundle, 'w:gz') as archive:
            for path in sorted(stage.rglob('*')):
                if path.is_file():
                    archive.add(path, arcname=path.relative_to(stage), recursive=False)
    checksum_file = output / 'SHA256SUMS'
    checksum_file.write_text(''.join(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + path.name + '\n' for path in (bundle, source)))
    return bundle


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    parser.add_argument('--registry', default='ghcr.io/looskis/looski-suite')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    print(package(args.version, args.registry, args.output))
