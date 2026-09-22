"""Private archive publication and pinned age encryption for local recovery.

No decryption identity is included in an archive. Decrypted data is published
only after age verifies the complete authenticated stream.
"""
import os
import re
import stat
import subprocess
import tempfile
from pathlib import Path
from dev import ROOT

IMAGE = 'looski-suite-recovery-tools:1.3.2'


def build_tools():
    subprocess.run(['docker', 'build', '-f', str(ROOT/'infra/recovery/Dockerfile'),
                    '-t', IMAGE, str(ROOT)], check=True)


def private_regular(path):
    path = Path(path).absolute()
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, 'rb') as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600:
            raise ValueError('Recovery identity must be a regular mode-0600 file')
        if info.st_size > 16384:
            raise ValueError('Recovery identity is too large')
        return stream.read()


def run_age(arguments, *, source=None, output=None, identity=None, keygen=False):
    command = ['docker', 'run', '--rm', '-i', '--network', 'none', '--read-only',
               '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges']
    with tempfile.TemporaryDirectory(prefix='looski-age-') as directory:
        if identity is not None:
            # Copy the checked bytes to a fresh private file before the mount.
            # The container sees only this key, not the caller's whole directory.
            key = Path(directory)/'identity'
            descriptor = os.open(key, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(descriptor, 'wb') as stream:
                stream.write(private_regular(identity))
            if ',' in str(key):
                raise ValueError('Temporary directory cannot contain a comma')
            command += ['--mount', 'type=bind,src='+str(key)+',dst=/identity,readonly']
        if keygen:
            command += ['--entrypoint', 'age-keygen']
        command += [IMAGE, *arguments]
        result = subprocess.run(command, stdin=source or subprocess.DEVNULL,
                                stdout=output or subprocess.PIPE, stderr=subprocess.PIPE)
        if result.returncode:
            # Tool errors may contain paths or untrusted archive metadata.
            raise RuntimeError('Recovery encryption tool failed; output was not published')
        return result.stdout


def publish(destination, writer):
    """Writer must complete verification before this no-overwrite publication."""
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise FileExistsError('Recovery destination already exists')
    descriptor, temporary = tempfile.mkstemp(prefix='.looski-recovery-', dir=destination.parent)
    try:
        with os.fdopen(descriptor, 'wb') as output:
            writer(output)
            output.flush()
            os.fsync(output.fileno())
        os.link(temporary, destination)
        parent = os.open(destination.parent, os.O_RDONLY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        os.unlink(temporary)
    return destination


def generate_identity(destination):
    publish(destination, lambda output: run_age([], output=output, keygen=True))
    return recipient(destination)


def recipient(identity):
    value = run_age(['-y', '/identity'], identity=identity, keygen=True).decode().strip()
    validate_recipient(value)
    return value


def validate_recipient(value):
    if not isinstance(value, str) or not re.fullmatch(r'age1[0-9a-z]{58}', value):
        raise ValueError('Expected a standard age X25519 recipient')


def encrypt(source, destination, public_key):
    validate_recipient(public_key)
    with Path(source).open('rb') as stream:
        return publish(destination, lambda output: run_age(['--encrypt', '-r', public_key], source=stream, output=output))


def decrypt(source, destination, identity):
    # age authenticates the complete input before publication; never pipe its
    # incremental plaintext directly into tar, pg_restore or another consumer.
    with Path(source).open('rb') as stream:
        return publish(destination, lambda output: run_age(['--decrypt', '-i', '/identity'], source=stream, output=output, identity=identity))
