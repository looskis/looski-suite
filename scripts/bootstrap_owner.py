#!/usr/bin/env python3
"""Provision this installation's first owner via its local operator boundary."""
import getpass
import json
import os
import subprocess

from dev import ROOT, configuration

if __name__ == '__main__':
    config = configuration()
    print(f"First owner for {config['COMPOSE_PROJECT_NAME']}")
    print('Use your own email. This local operator command confirms it without sending mail.')
    data = {
        'email': input('Email: ').strip(),
        'display_name': input('Your name: ').strip(),
        'workspace_name': input('Business / workspace name: ').strip(),
        'password': getpass.getpass('Password (at least 12 characters): '),
    }
    if data['password'] != getpass.getpass('Repeat password: '):
        raise SystemExit('Passwords differ; nothing was changed.')
    result = subprocess.run(
        ['docker', 'compose', 'exec', '-T', 'api', '/bin/sh',
         '/usr/local/bin/looski-entrypoint', 'bootstrap-owner'],
        cwd=ROOT, env={**os.environ, **config}, input=json.dumps(data), text=True,
    )
    raise SystemExit(result.returncode)
