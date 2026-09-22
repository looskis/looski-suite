#!/usr/bin/env python3
"""Local development stack. Never resets volumes or rotates existing secrets."""
import argparse
import fcntl
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
KEYS = ('COMPOSE_PROJECT_NAME', 'LOOSKI_HTTP_PORT', 'LOOSKI_PUBLIC_URL')


def configuration():
    values = {}
    config = ROOT / '.env'
    if config.exists():
        for line in config.read_text().splitlines():
            if not line.strip() or line.lstrip().startswith('#'):
                continue
            key, separator, value = line.partition('=')
            if not separator or key not in KEYS:
                raise SystemExit('Use only the documented non-secret settings in .env')
            values[key] = value
    values.update({k: os.environ[k] for k in KEYS if k in os.environ})
    values.setdefault('COMPOSE_PROJECT_NAME', 'looski-suite')
    values.setdefault('LOOSKI_HTTP_PORT', '58888')
    values.setdefault('LOOSKI_PUBLIC_URL', f"http://localhost:{values['LOOSKI_HTTP_PORT']}")
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]{0,62}', values['COMPOSE_PROJECT_NAME']):
        raise SystemExit('COMPOSE_PROJECT_NAME must contain lowercase letters, digits, hyphens, or underscores')
    if not values['LOOSKI_HTTP_PORT'].isdigit() or not 1024 <= int(values['LOOSKI_HTTP_PORT']) <= 65535:
        raise SystemExit('LOOSKI_HTTP_PORT must be between 1024 and 65535')
    origin = urlsplit(values['LOOSKI_PUBLIC_URL'])
    if (origin.scheme not in ('http', 'https') or not origin.hostname or origin.path
            or origin.query or origin.fragment or origin.username or origin.password
            or re.search(r'[\s$#"\'\\]', values['LOOSKI_PUBLIC_URL'])):
        raise SystemExit('LOOSKI_PUBLIC_URL must be an HTTP(S) origin without credentials, path, or trailing slash')
    if origin.scheme == 'http' and origin.hostname not in ('localhost', '127.0.0.1'):
        raise SystemExit('Use HTTPS for a public origin')
    if origin.scheme == 'http' and origin.port != int(values['LOOSKI_HTTP_PORT']):
        raise SystemExit('Local LOOSKI_PUBLIC_URL port must match LOOSKI_HTTP_PORT')
    return values


def configure(values):
    directory = ROOT / '.local' / values['COMPOSE_PROJECT_NAME'] / 'secrets'
    expected = {'db_password', 'runtime_password', 'jwt_secret', 'database_url', 'migration_url'}
    if directory.exists():
        if {p.name for p in directory.iterdir()} != expected or any(not (directory / n).stat().st_size for n in expected):
            raise SystemExit('Incomplete secret set. Restore this installation’s secrets; setup will not rotate them.')
    else:
        # Existing named volumes must never be paired with newly generated passwords.
        volumes = subprocess.check_output(['docker', 'volume', 'ls', '-q', '--filter',
            f"label=com.docker.compose.project={values['COMPOSE_PROJECT_NAME']}"], text=True)
        if volumes.strip():
            raise SystemExit('Existing installation volumes found without secrets. Restore secrets before continuing.')
        directory.mkdir(parents=True, mode=0o700)
        db_password, runtime_password = secrets.token_hex(32), secrets.token_hex(32)
        files = {
            'db_password': db_password,
            'runtime_password': runtime_password,
            'jwt_secret': secrets.token_hex(32),
            'database_url': f'postgres://looski_runtime:{runtime_password}@db:5432/postgres',
            'migration_url': f'postgres://postgres:{db_password}@db:5432/postgres?sslmode=disable',
        }
        for name, value in files.items():
            with (directory / name).open('x') as stream:
                os.chmod(stream.name, 0o600)
                stream.write(value + '\n')
    config = ROOT / '.env'
    if not config.exists():
        with config.open('x') as stream:
            os.chmod(stream.name, 0o600)
            stream.write(''.join(f'{key}={values[key]}\n' for key in KEYS))
    print(f"Installation: {values['COMPOSE_PROJECT_NAME']} | {values['LOOSKI_PUBLIC_URL']}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['configure', 'up', 'stop', 'status', 'logs'])
    args = parser.parse_args()
    values = configuration()
    environment = {**os.environ, **values}
    compose = ['docker', 'compose', '--project-directory', str(ROOT), '-f', str(ROOT / 'compose.yaml')]

    def run(*arguments, capture=False):
        return subprocess.run([*compose, *arguments], cwd=ROOT, env=environment,
                              check=True, text=True, capture_output=capture)

    # One setup/update operation per installation, including configuration creation.
    state = ROOT / '.local' / values['COMPOSE_PROJECT_NAME']
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (state / 'setup.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if args.command in ('configure', 'up'):
            configure(values)
        if args.command == 'configure':
            run('config', '--quiet')
        elif args.command == 'up':
            running_gateway = run('ps', '--status', 'running', '-q', 'gateway', capture=True).stdout.strip()
            if not running_gateway:
                for family, address in [(socket.AF_INET, '127.0.0.1'), (socket.AF_INET6, '::1')]:
                    with socket.socket(family, socket.SOCK_STREAM) as sock:
                        try:
                            sock.bind((address, int(values['LOOSKI_HTTP_PORT'])))
                        except OSError:
                            raise SystemExit(f'Cannot bind {address}:LOOSKI_HTTP_PORT. Both loopback families must be available; choose a free port and matching LOOSKI_PUBLIC_URL in .env.')
            run('build')
            # Quiesce callers before applying schema changes. Failure leaves them stopped.
            run('stop', 'gateway', 'web', 'api', 'worker')
            run('up', '-d', '--wait', 'db', 'auth', 'supabase-gateway')
            # Always run a new migration container, even if the old one exited successfully.
            run('up', '--no-deps', '--force-recreate', '--exit-code-from', 'migrate', 'migrate')
            # Dependencies have already passed health/migration gates. Avoid
            # Compose restarting the completed one-shot migration job a second time.
            run('up', '-d', '--no-deps', '--wait', 'api', 'web', 'gateway', 'worker')
            print(f"Ready: http://localhost:{values['LOOSKI_HTTP_PORT']}")
        elif args.command == 'stop':
            run('stop')
        elif args.command == 'status':
            run('ps', '-a')
        elif args.command == 'logs':
            run('logs', '--tail', '80', 'api', 'web', 'gateway')


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'Command failed (exit {error.returncode}); data volumes were not reset.')
