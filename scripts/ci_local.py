#!/usr/bin/env python3
"""Rehearse the committed CI revision on a fresh, isolated local installation.

Usage: python3 scripts/ci_local.py [--ref HEAD] [--port 58891]
Requires the workflow's Node, pnpm and Rust versions on PATH. This uses the
host OS/architecture, not a GitHub-hosted Ubuntu runner. No push or dispatch.
"""
import argparse
import datetime
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def output(*command, cwd=ROOT, env=None):
    return subprocess.check_output(command, cwd=cwd, env=env, text=True).strip()


def colima_ipv6(port, project, env, log):
    """Forward the VM's real IPv6 listener when Colima only forwards IPv4."""
    if sys.platform != 'darwin':
        return None
    context = output('docker', 'context', 'show', env=env)
    endpoint = output('docker', 'context', 'inspect', context, '--format', '{{.Endpoints.docker.Host}}', env=env)
    config = Path.home() / '.colima/ssh_config'
    if context != 'colima' or not endpoint.endswith('/.colima/default/docker.sock') or not config.is_file() or env.get('DOCKER_HOST'):
        return None
    try:
        with socket.create_connection(('::1', port), timeout=1):
            return None
    except ConnectionRefusedError:
        pass
    # Never hide missing Compose bindings: verify the VM really publishes IPv6.
    bindings = json.loads(output('docker', 'inspect', project + '-gateway-1', '--format',
                                 '{{json .HostConfig.PortBindings}}', env=env))
    if {'HostIp': '::1', 'HostPort': str(port)} not in bindings.get('8080/tcp', []):
        raise RuntimeError('Gateway does not publish IPv6; refusing to compensate for missing Compose configuration')
    ssh = ['ssh', '-F', str(config), '-S', 'none', '-o', 'ControlMaster=no', '-o', 'ExitOnForwardFailure=yes']
    probe = output(*ssh, 'colima', 'curl', '-g', '--noproxy', "'*'", '-fsS', f"'http://[::1]:{port}/health/live'", env=env)
    if json.loads(probe) != {'status': 'ok'}:
        raise RuntimeError('The gateway IPv6 endpoint inside Colima is unhealthy')
    process = subprocess.Popen([*ssh, '-N', '-L', f'[::1]:{port}:[::1]:{port}', 'colima'],
                               env=env, stdout=log, stderr=log)
    for _ in range(50):
        if process.poll() is not None:
            raise RuntimeError('Colima IPv6 forwarding failed; see colima-ipv6.log')
        try:
            with socket.create_connection(('::1', port), timeout=0.1):
                return process
        except ConnectionRefusedError:
            time.sleep(0.1)
    process.terminate()
    process.wait(timeout=5)
    raise RuntimeError('Timed out starting Colima IPv6 forwarding')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref', default='HEAD', help='committed revision; uncommitted edits are excluded')
    parser.add_argument('--port', type=int, default=58891)
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error('port must be between 1024 and 65535')
    for family, address in ((socket.AF_INET, '127.0.0.1'), (socket.AF_INET6, '::1')):
        with socket.socket(family, socket.SOCK_STREAM) as listener:
            try:
                listener.bind((address, args.port))
            except OSError:
                parser.error(f'{address}:{args.port} is unavailable; choose another --port')
    revision = output('git', 'rev-parse', '--verify', args.ref + '^{commit}')
    base = ROOT / '.local' / 'ci-preflight'
    base.mkdir(parents=True, exist_ok=True)
    with (base / 'run.lock').open('w') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SystemExit('Another local preflight is using this checkout’s build cache')
        stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%d-%H%M%S')
        run = base / (stamp + '-' + revision[:8])
        source = run / 'source'
        source.mkdir(parents=True)
        archive = run / 'source.tar'
        subprocess.run(['git', 'archive', '--format=tar', '-o', str(archive), revision], cwd=ROOT, check=True)
        subprocess.run(['tar', '-xf', str(archive), '-C', str(source)], check=True)
        archive.unlink()
        workflow = (source / '.github/workflows/check.yml').read_text()
        # Fail closed if the workflow changes instead of silently skipping new steps.
        commands = re.findall(r'^\s+- run: (.+)$', workflow, re.MULTILINE)
        pnpm = json.loads((source / 'package.json').read_text())['packageManager']
        expected = [f'npm install -g {pnpm}', 'pnpm install --frozen-lockfile',
                    'make check test', 'pnpm build', 'cargo build --locked', 'make up', 'make acceptance']
        if commands != expected:
            raise SystemExit('Workflow commands changed; update ci_local.py before claiming parity')
        node = re.search(r'node-version: ([\d.]+)', workflow).group(1)
        rust = re.search(r'dtolnay/rust-toolchain@([\d.]+)', workflow).group(1)
        # Avoid inheriting local mail, tunnel, Compose, API, or credential settings.
        keep = ('PATH', 'HOME', 'USER', 'TMPDIR', 'DOCKER_HOST', 'DOCKER_CONTEXT',
                'DOCKER_CONFIG', 'RUSTUP_HOME', 'CARGO_HOME')
        env = {key: os.environ[key] for key in keep if key in os.environ}
        architecture = {'arm64': 'arm64', 'aarch64': 'arm64', 'x86_64': 'x64'}.get(platform.machine(), platform.machine())
        tool_paths = [ROOT / '.local/ci-tools' / f'node-v{node}-{sys.platform}-{architecture}' / 'bin',
                      ROOT / '.local/ci-tools/pnpm/node_modules/.bin', Path.home() / '.cargo/bin']
        env['PATH'] = os.pathsep.join([*(str(path) for path in tool_paths if path.is_dir()), env.get('PATH', '')])
        versions = {name: output(name, '--version', cwd=source, env=env)
                    for name in ('node', 'pnpm', 'rustc', 'cargo', 'python3')}
        if versions['node'] != 'v' + node or versions['pnpm'] != pnpm.split('@')[1] or versions['rustc'].split()[1] != rust:
            raise SystemExit(f'Expected Node {node}, {pnpm}, Rust {rust}; found {versions}. Adjust PATH and retry.')
        versions['docker'] = output('docker', 'info', '--format', '{{.ServerVersion}} {{.Architecture}}', env=env)
        versions['compose'] = output('docker', 'compose', 'version', '--short', env=env)
        project = 'looski-local-' + stamp + '-identity-check'
        workflow_env = dict(re.findall(r'^      ([A-Z_]+): ["\']?([^\n"\']+)["\']?$', workflow, re.MULTILINE))
        if set(workflow_env) != {'COMPOSE_PROJECT_NAME', 'LOOSKI_HTTP_PORT', 'LOOSKI_PUBLIC_URL', 'LOOSKI_MAIL_MODE'}:
            raise SystemExit('Workflow environment changed; update ci_local.py before claiming parity')
        env.update(workflow_env)
        env.update(COMPOSE_PROJECT_NAME=project, LOOSKI_HTTP_PORT=str(args.port),
                   LOOSKI_PUBLIC_URL=f'http://localhost:{args.port}',
                   CI='true', PYTHONUNBUFFERED='1')
        cache = base / 'cargo-target'
        cache.mkdir(exist_ok=True)
        (source / 'target').symlink_to(cache, target_is_directory=True)
        report = {'revision': revision, 'platform': platform.platform(), 'versions': versions,
                  'project': project, 'port': args.port, 'steps': [], 'result': 'running',
                  'limits': 'Host OS/architecture, Python, Docker and Compose differ from GitHub; caches are reused.'}

        def save():
            (run / 'result.json').write_text(json.dumps(report, indent=2) + '\n')

        def step(label, command):
            logfile = run / (str(len(report['steps']) + 1).zfill(2) + '-' + label + '.log')
            print(f'Running {label}; log: {logfile}', flush=True)
            started = time.monotonic()
            with logfile.open('w') as log:
                result = subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT)
            report['steps'].append({'name': label, 'exit_code': result.returncode,
                                    'seconds': round(time.monotonic() - started, 1), 'log': str(logfile)})
            save()
            if result.returncode:
                print('\n'.join(logfile.read_text(errors='replace').splitlines()[-35:]), flush=True)
                raise subprocess.CalledProcessError(result.returncode, command)

        print(f'Revision {revision}; fresh installation {project}; results: {run}', flush=True)
        print('Local preflight only: host platform differs from GitHub Ubuntu/x86. Uncommitted edits excluded.', flush=True)
        save()
        stack_started = False
        forwarder = None
        try:
            step('dependencies', ['pnpm', 'install', '--frozen-lockfile'])
            step('checks-and-tests', ['make', 'check', 'test'])
            step('web-build', ['pnpm', 'build'])
            step('rust-build', ['cargo', 'build', '--locked'])
            stack_started = True
            step('fresh-install', ['make', 'up'])
            with (run / 'colima-ipv6.log').open('w') as log:
                forwarder = colima_ipv6(args.port, project, env, log)
            if forwarder:
                report['adaptations'] = ['Temporary SSH forwarding from host ::1 to the verified Colima VM IPv6 gateway; removed after the run.']
                print(report['adaptations'][0], flush=True)
                save()
            step('acceptance', ['make', 'acceptance'])
            report['result'] = 'passed'
        except (subprocess.CalledProcessError, OSError, RuntimeError, KeyboardInterrupt) as error:
            report['result'] = 'failed'
            report['error'] = str(error) or 'Interrupted'
        finally:
            if forwarder:
                forwarder.terminate()
                forwarder.wait(timeout=5)
            if stack_started:
                try:
                    step('stop', ['python3', 'scripts/dev.py', 'stop'])
                except (subprocess.CalledProcessError, OSError, KeyboardInterrupt) as error:
                    report['result'] = 'failed'
                    report['stop_error'] = str(error) or 'Interrupted'
            save()
        print(f"Local preflight {report['result']}: {run / 'result.json'}", flush=True)
        print('Snapshot, private fixture and stopped volumes retained for diagnosis; no shared installation changed.', flush=True)
        return 0 if report['result'] == 'passed' else 1


if __name__ == '__main__':
    sys.exit(main())
