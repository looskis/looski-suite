#!/usr/bin/env python3
"""Agent-friendly installation operations. stdout is one JSON result; progress uses stderr."""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import platform
import re
import secrets
import shutil
import subprocess
import sys
import time
from pathlib import Path
from urllib.request import Request, build_opener
from urllib.parse import urlsplit
from urllib.error import URLError

import dev
from cloudflare import Cloudflare, NoRedirect, discover, provision
from recovery_archive import private_regular


def run(command, *, capture=False, environment=None):
    result = subprocess.run(command, cwd=dev.ROOT, env=environment,
                            stdout=subprocess.PIPE if capture else sys.stderr,
                            stderr=subprocess.PIPE if capture else sys.stderr, text=True)
    if result.returncode:
        raise RuntimeError('Operation failed: ' + command[0] + ' (exit ' + str(result.returncode) + '); retry after resolving the prerequisite')
    return result.stdout if capture else None


def doctor():
    checks = {'platform': platform.system() in ('Darwin', 'Linux'),
              'architecture': platform.machine() in ('arm64', 'aarch64', 'x86_64', 'AMD64'),
              'python': sys.version_info >= (3, 9), 'docker': bool(shutil.which('docker'))}
    if checks['docker']:
        try:
            checks['linux_engine'] = run(['docker', 'info', '--format', '{{.OSType}}'], capture=True).strip() == 'linux'
            version = run(['docker', 'compose', 'version', '--short'], capture=True).strip()
            match = re.match(r'v?(\d+)\.(\d+)', version)
            checks['compose'] = bool(match and tuple(map(int, match.groups())) >= (2, 20))
        except RuntimeError:
            checks['linux_engine'] = False
    guidance = {'platform': 'Use macOS or Linux; on Windows use a Linux host or WSL2.',
                'architecture': 'Use an ARM64 or x86-64 host.',
                'python': 'Install Python 3.9 or newer.',
                'docker': 'Install Docker with Compose following the official instructions for this OS.',
                'linux_engine': 'Start the Docker Linux engine and ensure this user can access it.',
                'compose': 'Install or update the Docker Compose v2 plugin to version 2.20 or newer.'}
    return {'ready': all(checks.values()), 'checks': checks,
            'next_actions': [guidance[key] for key, passed in checks.items() if not passed],
            'free_disk_bytes': shutil.disk_usage(dev.ROOT).free,
            'release': json.loads((dev.ROOT / 'release.json').read_text()) if (dev.ROOT / 'release.json').exists() else 'source-checkout'}


def values():
    # Installation commands use persisted identity, not ambient shell overrides.
    with contextlib.ExitStack() as stack:
        old = {key: os.environ.pop(key) for key in dev.KEYS if key in os.environ}
        stack.callback(os.environ.update, old)
        return dev.configuration()


def compose(config, *args, capture=False):
    command = ['docker', 'compose', '--project-directory', str(dev.ROOT), '-f', str(dev.ROOT / 'compose.yaml')]
    if config['LOOSKI_TUNNEL_MODE'] == 'cloudflare-managed':
        command += ['-f', str(dev.ROOT / 'compose.managed.yaml')]
    return run([*command, *args], capture=capture, environment={**os.environ, **config})


def save_config(config):
    dev.write_private(dev.ROOT / '.env', ''.join(f'{key}={config[key]}\n' for key in dev.KEYS))


def configure(args):
    config = values()
    overrides = {'COMPOSE_PROJECT_NAME': args.project, 'LOOSKI_HTTP_PORT': str(args.port) if args.port is not None else None,
                 'LOOSKI_PUBLIC_URL': args.url}
    if (dev.ROOT / '.env').exists():
        if args.project is not None and args.project != config['COMPOSE_PROJECT_NAME']:
            raise ValueError('Never rename an existing installation; choose a separate directory for another installation')
        if args.url is not None and args.url != config['LOOSKI_PUBLIC_URL']:
            raise ValueError('Use publish to change an existing installation’s public origin')
        if args.port is not None:
            config['LOOSKI_HTTP_PORT'] = str(args.port)
            origin = urlsplit(config['LOOSKI_PUBLIC_URL'])
            if origin.scheme == 'http':
                config['LOOSKI_PUBLIC_URL'] = f'http://{origin.hostname}:{args.port}'
    else:
        config.update({key: value for key, value in overrides.items() if value is not None})
        if not args.url:
            config['LOOSKI_PUBLIC_URL'] = 'http://localhost:' + config['LOOSKI_HTTP_PORT']
        config['LOOSKI_AUTH_ISSUER'] = config['LOOSKI_PUBLIC_URL'] + '/auth/v1'
    with contextlib.ExitStack() as stack:
        old = os.environ.copy()
        stack.callback(lambda: (os.environ.clear(), os.environ.update(old)))
        os.environ.update(config)
        config = dev.configuration()
    state = dev.ROOT / '.local' / config['COMPOSE_PROJECT_NAME']
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (state / 'setup.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        with contextlib.redirect_stdout(sys.stderr):
            dev.configure(config, dev.ROOT / '.env')
        save_config(config)
    return config


def up(config):
    run([sys.executable, str(dev.ROOT / 'scripts/dev.py'), 'up'], environment={**os.environ, **config})


def owner_exists(config):
    answer = compose(config, 'exec', '-T', 'db', 'psql', '-U', 'postgres', '-d', 'postgres', '-At', '-v', 'ON_ERROR_STOP=1',
                     '-c', 'SELECT bootstrapped_at IS NOT NULL FROM suite.installation WHERE singleton', capture=True).strip()
    if answer not in ('t', 'f'):
        raise RuntimeError('Could not determine owner bootstrap state')
    return answer == 't'


def services(config):
    raw = compose(config, 'ps', '-a', '--format', 'json', capture=True).strip()
    return json.loads(raw) if raw.startswith('[') else [json.loads(line) for line in raw.splitlines()]


def claim_link(config):
    state = dev.ROOT / '.local' / config['COMPOSE_PROJECT_NAME']
    with (state / 'setup.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if owner_exists(config):
            return {'state': 'already_claimed', 'login_url': config['LOOSKI_PUBLIC_URL'] + '/login'}
        token = secrets.token_hex(32)
        expires = int(time.time()) + 3600
        # The service receives only a hash. The bearer link stays in a private file.
        dev.write_private(state / 'owner-claim.json', json.dumps({'token_sha256': hashlib.sha256(token.encode()).hexdigest(), 'expires_at': expires}))
        link_file = state / 'owner-setup-link'
        dev.write_private(link_file, config['LOOSKI_PUBLIC_URL'] + '/setup#' + token + '\n')
        compose(config, 'up', '-d', '--no-deps', '--force-recreate', '--wait', 'owner-setup')
    return {'state': 'awaiting_owner', 'private_link_file': str(link_file), 'expires_at': expires}


def verify(config):
    origin = config['LOOSKI_PUBLIC_URL']
    checks = {}
    required = {'db', 'auth', 'auth-templates', 'supabase-gateway', 'api', 'worker', 'web', 'gateway', 'owner-setup'}
    if config.get('LOOSKI_TUNNEL_MODE', 'disabled') != 'disabled': required.add('tunnel')
    if config['LOOSKI_MAIL_MODE'] == 'local': required.add('mailpit')
    inventory = {service['Service']: service for service in services(config)}
    checks['services'] = all(inventory.get(service, {}).get('Health') == 'healthy'
                             and inventory[service].get('State') == 'running' for service in required)
    migration = inventory.get('migrate', {})
    checks['migration'] = migration.get('State') == 'exited' and migration.get('ExitCode') == 0
    http = build_opener(NoRedirect())
    for path, expected in [('/health/ready', 200), ('/api/v1/system', 200), ('/login', 200), ('/auth/v1/admin/users', 404), ('/rest/v1/', 404)]:
        try:
            with http.open(Request(origin + path), timeout=15) as response:
                checks[path] = response.status == expected
                if path == '/api/v1/system':
                    checks['canonical_origin'] = json.loads(response.read(65536)).get('public_url') == origin
        except URLError as error:
            checks[path] = getattr(error, 'code', None) == expected
        except (TimeoutError, ValueError):
            checks[path] = False
    claimed = owner_exists(config)
    state = dev.ROOT / '.local' / config['COMPOSE_PROJECT_NAME']
    backup_path = state / 'backup-status.json'
    return {'ready': all(checks.values()), 'url': origin, 'checks': checks,
            'owner': 'created' if claimed else 'awaiting_owner',
            'email': 'configured_unverified' if config['LOOSKI_MAIL_MODE'] == 'smtp' else config['LOOSKI_MAIL_MODE'],
            'backup': json.loads(backup_path.read_text()).get('state') if backup_path.exists() else 'not_configured'}


def wait_verify(config, seconds):
    if not 0 <= seconds <= 900:
        raise ValueError('--wait must be between 0 and 900 seconds')
    deadline = time.monotonic() + seconds
    while True:
        result = verify(config)
        remaining = deadline - time.monotonic()
        if result['ready'] or remaining <= 0:
            return result
        print('Waiting for the configured address and services to become ready…', file=sys.stderr, flush=True)
        time.sleep(min(5, remaining))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('doctor')
    install = commands.add_parser('install')
    install.add_argument('--project')
    install.add_argument('--port', type=int)
    install.add_argument('--url')
    commands.add_parser('start')
    commands.add_parser('stop')
    commands.add_parser('status')
    verify_parser = commands.add_parser('verify')
    verify_parser.add_argument('--wait', type=int, default=0, metavar='SECONDS', help='wait for readiness (maximum 900 seconds)')
    commands.add_parser('claim')
    publish = commands.add_parser('publish')
    publish.add_argument('--url', required=True)
    publish.add_argument('--account-id', help='optional account selector; discovered from hostname by default')
    publish.add_argument('--zone-id', help='optional zone selector; discovered from hostname by default')
    publish.add_argument('--token-file', required=True, type=Path)
    smtp = commands.add_parser('smtp')
    smtp.add_argument('--credentials-file', required=True, type=Path)
    backup = commands.add_parser('backup')
    backup.add_argument('--recipient', required=True)
    backup.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if args.command == 'doctor':
        result = doctor()
    elif args.command == 'install':
        result = doctor()
        if not result['ready']:
            print(json.dumps(result)); return 2
        config = configure(args)
        up(config)
        result = verify(config)
        if result['ready']:
            result['setup'] = claim_link(config)
    else:
        if not (dev.ROOT / '.env').is_file():
            raise ValueError('Run install first')
        config = values()
        state = dev.ROOT / '.local' / config['COMPOSE_PROJECT_NAME']
        if args.command == 'publish':
            with (state / 'setup.lock').open('a') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                if config['LOOSKI_TUNNEL_MODE'] == 'cloudflare':
                    raise ValueError('Existing locally managed tunnel requires an explicit migration; use a fresh installation')
                client = Cloudflare(args.token_file)
                account, zone = discover(client, args.url, args.account_id, args.zone_id)
                route = provision(client, state, account, zone, args.url)
                config.update(LOOSKI_PUBLIC_URL='https://' + route['hostname'], LOOSKI_TUNNEL_MODE='cloudflare-managed')
                if config['LOOSKI_MAIL_MODE'] == 'local':
                    config['LOOSKI_MAIL_MODE'] = 'disabled'
                config['LOOSKI_AUTH_MAIL_ENABLED'] = str(config['LOOSKI_MAIL_MODE'] != 'disabled').lower()
                save_config(config)
            up(config)
            result = wait_verify(config, 180)
            if result['ready']:
                result['setup'] = claim_link(config)
        elif args.command == 'smtp':
            with (state / 'setup.lock').open('a') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                target = state / 'smtp.json'
                previous = target.read_bytes() if target.exists() else None
                dev.write_private(target, private_regular(args.credentials_file).decode())
                try:
                    dev.configure_mail({**config, 'LOOSKI_MAIL_MODE': 'smtp'})
                except BaseException:
                    if previous is None: target.unlink()
                    else: dev.write_private(target, previous.decode())
                    raise
                config.update(LOOSKI_MAIL_MODE='smtp', LOOSKI_AUTH_MAIL_ENABLED='true')
                save_config(config)
            up(config)
            result = verify(config)
        elif args.command == 'claim':
            result = claim_link(config)
        elif args.command == 'verify':
            result = wait_verify(config, args.wait)
        elif args.command == 'status':
            result = {'project': config['COMPOSE_PROJECT_NAME'], 'url': config['LOOSKI_PUBLIC_URL'], 'services': services(config)}
        elif args.command == 'stop':
            run([sys.executable, str(dev.ROOT / 'scripts/dev.py'), 'stop'], environment={**os.environ, **config})
            result = {'state': 'stopped', 'data_preserved': True}
        elif args.command == 'start':
            up(config); result = verify(config)
        elif args.command == 'backup':
            import recovery
            run([sys.executable, str(dev.ROOT / 'scripts/recovery.py'), 'build-tools'])
            result = recovery.create(recovery.Installation(config), args.output, args.recipient)
    print(json.dumps(result))
    return 0 if result.get('ready', True) else 2


if __name__ == '__main__':
    try:
        with (dev.ROOT / '.installer.lock').open('a') as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise RuntimeError('Another installer command is running in this directory') from None
            result = main()
        raise SystemExit(result)
    except SystemExit as error:
        if isinstance(error.code, str):
            print(json.dumps({'ready': False, 'error': error.code}))
            raise SystemExit(1)
        raise
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(json.dumps({'ready': False, 'error': str(error)}))
        raise SystemExit(1)
