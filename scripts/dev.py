#!/usr/bin/env python3
"""Local development stack. Never resets volumes or rotates existing secrets."""
import argparse
import fcntl
import json
import hmac
import hashlib
import stat
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
KEYS = ('COMPOSE_PROJECT_NAME', 'LOOSKI_HTTP_PORT', 'LOOSKI_PUBLIC_URL', 'LOOSKI_MAIL_MODE', 'LOOSKI_DIAGNOSTICS_MODE',
        'LOOSKI_TUNNEL_MODE')
TUNNEL_CREDENTIAL_KEYS = {'AccountTag', 'TunnelSecret', 'TunnelID'}


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
    values.setdefault('LOOSKI_MAIL_MODE', 'disabled')
    if values['LOOSKI_MAIL_MODE'] not in ('disabled', 'local', 'smtp'):
        raise SystemExit('LOOSKI_MAIL_MODE must be disabled, local or smtp')
    origin = urlsplit(values['LOOSKI_PUBLIC_URL'])
    if (origin.scheme not in ('http', 'https') or not origin.hostname or origin.path
            or origin.query or origin.fragment or origin.username or origin.password
            or re.search(r'[\s$#"\'\\]', values['LOOSKI_PUBLIC_URL'])):
        raise SystemExit('LOOSKI_PUBLIC_URL must be an HTTP(S) origin without credentials, path, or trailing slash')
    if origin.scheme == 'http' and origin.hostname not in ('localhost', '127.0.0.1'):
        raise SystemExit('Use HTTPS for a public origin')
    if origin.scheme == 'http' and origin.port != int(values['LOOSKI_HTTP_PORT']):
        raise SystemExit('Local LOOSKI_PUBLIC_URL port must match LOOSKI_HTTP_PORT')
    if values['LOOSKI_MAIL_MODE'] == 'local' and origin.hostname not in ('localhost', '127.0.0.1'):
        raise SystemExit('Local mail capture is restricted to loopback development origins')
    values.setdefault('LOOSKI_DIAGNOSTICS_MODE', 'local')
    if values['LOOSKI_DIAGNOSTICS_MODE'] not in ('disabled','local'):
        raise SystemExit('LOOSKI_DIAGNOSTICS_MODE must be disabled or local')
    values.setdefault('LOOSKI_TUNNEL_MODE', 'disabled')
    if values['LOOSKI_TUNNEL_MODE'] not in ('disabled', 'cloudflare'):
        raise SystemExit('LOOSKI_TUNNEL_MODE must be disabled or cloudflare')
    if values['LOOSKI_TUNNEL_MODE'] == 'cloudflare' and (origin.scheme != 'https' or origin.port):
        raise SystemExit('A Cloudflare tunnel requires an HTTPS LOOSKI_PUBLIC_URL on the default port')
    values['LOOSKI_AUTH_MAIL_ENABLED'] = str(values['LOOSKI_MAIL_MODE'] != 'disabled').lower()
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
    # Domain-separated invitation signing material is reproducible from the
    # backed-up installation secret; it is never the Supabase JWT signing key.
    invitation_key = hmac.new((directory / 'jwt_secret').read_bytes().strip(), b'looski-suite:staff-invitation:v1', hashlib.sha256).hexdigest()
    invitation_file = directory.parent / 'invitation-key'
    if invitation_file.exists():
        if invitation_file.read_text().strip() != invitation_key:
            raise SystemExit('Invitation key does not match this installation; restore consistent secrets.')
    else:
        with invitation_file.open('x') as stream:
            os.chmod(stream.name, 0o600)
            stream.write(invitation_key + '\n')
    diagnostics_file=directory.parent/'diagnostics-key'
    diagnostics_key=hmac.new((directory/'jwt_secret').read_bytes().strip(),b'looski-suite:diagnostics:v1',hashlib.sha256).hexdigest()
    if diagnostics_file.exists():
        if diagnostics_file.read_text().strip()!=diagnostics_key:raise SystemExit('Diagnostics key does not match this installation')
    else:
        descriptor=os.open(diagnostics_file,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
        with os.fdopen(descriptor,'w') as stream:stream.write(diagnostics_key+'\n')
    configure_mail(values)
    configure_tunnel(values)
    config = ROOT / '.env'
    if not config.exists():
        with config.open('x') as stream:
            os.chmod(stream.name, 0o600)
            stream.write(''.join(f'{key}={values[key]}\n' for key in KEYS))
    print(f"Installation: {values['COMPOSE_PROJECT_NAME']} | {values['LOOSKI_PUBLIC_URL']}", flush=True)


def configure_mail(values):
    state = ROOT / '.local' / values['COMPOSE_PROJECT_NAME']
    mode = values['LOOSKI_MAIL_MODE']
    mail = {'host': '127.0.0.1', 'port': 1, 'user': '', 'password': '',
            'from': 'no-reply@example.invalid', 'sender_name': 'Looski Suite'}
    if mode == 'local':
        mail.update(host='mailpit', port=1025)
    elif mode == 'smtp':
        path = state / 'smtp.json'
        if not path.is_file() or path.is_symlink() or stat.S_IMODE(path.stat().st_mode) != 0o600:
            raise SystemExit('SMTP requires a regular private .local/<installation>/smtp.json file with mode 0600; see docs/auth-email.md')
        try:
            mail = json.loads(path.read_text())
            if set(mail) != {'host', 'port', 'user', 'password', 'from', 'sender_name'}:
                raise ValueError()
            if not isinstance(mail['port'], int) or isinstance(mail['port'], bool) or mail['port'] != 465:
                raise ValueError()
            if any(not isinstance(mail[k], str) or not mail[k] or len(mail[k]) > 1024 or any(c in mail[k] for c in '\r\n\0') for k in ('host', 'user', 'password', 'from', 'sender_name')):
                raise ValueError()
            if not re.fullmatch(r'[A-Za-z0-9.-]+', mail['host']) or not re.fullmatch(r'[^<>\s@]+@[^<>\s@]+', mail['from']):
                raise ValueError()
        except (ValueError, TypeError, OSError):
            raise SystemExit('Invalid SMTP configuration; expected host, port (465, implicit TLS), user, password, from and sender_name. Values are not logged.')
    # Separate from the immutable core secret set. Atomic replacement never rotates identity/DB secrets.
    target = state / 'auth-mail.json'
    temporary = state / 'auth-mail.json.tmp'
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(mail, stream)
    temporary.replace(target)


def write_private(target, content):
    temporary = target.with_name(target.name + '.tmp')
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as stream:
        os.fchmod(stream.fileno(), 0o600)
        stream.write(content)
    temporary.replace(target)


def configure_tunnel(values):
    """Render the connector configuration from the one public origin.

    The tunnel credential is created once by `cloudflared tunnel create` and kept
    privately with the installation; setup never creates or rotates it.
    """
    state = ROOT / '.local' / values['COMPOSE_PROJECT_NAME']
    tunnel_id = '00000000-0000-0000-0000-000000000000'
    if values['LOOSKI_TUNNEL_MODE'] == 'cloudflare':
        path = state / 'cloudflare-tunnel.json'
        if not path.is_file() or path.is_symlink() or stat.S_IMODE(path.stat().st_mode) != 0o600:
            raise SystemExit('Cloudflare tunnel requires a regular private .local/<installation>/cloudflare-tunnel.json file with mode 0600; see docs/setup-contract.md')
        try:
            credentials = json.loads(path.read_text())
            if not TUNNEL_CREDENTIAL_KEYS <= set(credentials) or any(not isinstance(credentials[k], str) or not credentials[k] for k in TUNNEL_CREDENTIAL_KEYS):
                raise ValueError()
            if not re.fullmatch(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', credentials['TunnelID']):
                raise ValueError()
        except (ValueError, TypeError, OSError):
            raise SystemExit('Invalid Cloudflare tunnel credentials; expected the JSON file written by cloudflared tunnel create. Values are not logged.')
        tunnel_id = credentials['TunnelID']
    else:
        # Compose resolves every declared secret file; the disabled connector never starts.
        path = state / 'cloudflare-tunnel.json'
        if not path.exists():
            write_private(path, '{}')
    hostname = urlsplit(values['LOOSKI_PUBLIC_URL']).hostname
    write_private(state / 'cloudflared.yml', ''.join([
        f'tunnel: {tunnel_id}\n',
        'credentials-file: /run/secrets/cloudflare_tunnel\n',
        'no-autoupdate: true\n',
        'metrics: 127.0.0.1:2000\n',
        'ingress:\n',
        f'  - hostname: {hostname}\n',
        '    service: http://gateway:8080\n',
        '  - service: http_status:404\n',
    ]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['configure', 'up', 'stop', 'status', 'logs'])
    args = parser.parse_args()
    values = configuration()
    environment = {**os.environ, **values}
    profiles = set(filter(None, environment.get('COMPOSE_PROFILES', '').split(',')))
    if values['LOOSKI_MAIL_MODE'] == 'local': profiles.add('local-mail')
    else: profiles.discard('local-mail')
    if values['LOOSKI_DIAGNOSTICS_MODE']=='local':profiles.add('observability')
    else:profiles.discard('observability')
    if values['LOOSKI_TUNNEL_MODE'] == 'cloudflare': profiles.add('public-tunnel')
    else: profiles.discard('public-tunnel')
    environment['COMPOSE_PROFILES'] = ','.join(sorted(profiles))
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
            if values['LOOSKI_MAIL_MODE'] == 'local':
                run('up', '-d', '--wait', 'mailpit')
            else:
                run('stop', 'mailpit')
            run('up', '-d', '--wait', 'db', 'auth-templates')
            # Compose does not reliably detect changed file-secret contents.
            run('up', '-d', '--no-deps', '--force-recreate', '--wait', 'auth')
            run('up', '-d', '--no-deps', '--wait', 'supabase-gateway')
            # Always run a new migration container, even if the old one exited successfully.
            run('up', '--no-deps', '--force-recreate', '--exit-code-from', 'migrate', 'migrate')
            # Dependencies have already passed health/migration gates. Avoid
            # Compose restarting the completed one-shot migration job a second time.
            if values['LOOSKI_DIAGNOSTICS_MODE']=='local':
                try:run('up','-d','--no-deps','--wait','--wait-timeout','45','clickhouse')
                except subprocess.CalledProcessError:print('Diagnostics unavailable; application startup continues without log storage',flush=True)
            else:run('stop','clickhouse')
            run('up', '-d', '--no-deps', '--wait', 'api', 'web', 'gateway', 'worker')
            if values['LOOSKI_TUNNEL_MODE'] == 'cloudflare':
                # Recreate so a changed hostname or credential takes effect.
                run('up', '-d', '--no-deps', '--force-recreate', '--wait', 'tunnel')
                print(f"Ready: {values['LOOSKI_PUBLIC_URL']} (local gateway http://localhost:{values['LOOSKI_HTTP_PORT']})")
            else:
                run('stop', 'tunnel')
                print(f"Ready: http://localhost:{values['LOOSKI_HTTP_PORT']}")
        elif args.command == 'stop':
            run('stop')
        elif args.command == 'status':
            run('ps', '-a')
        elif args.command == 'logs':
            run('logs', '--tail', '80', 'api', 'web', 'gateway', *(['tunnel'] if values['LOOSKI_TUNNEL_MODE'] == 'cloudflare' else []))


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'Command failed (exit {error.returncode}); data volumes were not reset.')
