#!/usr/bin/env python3
"""Import a downloaded Google web OAuth client without printing its secret."""
import argparse
import json
from pathlib import Path

import dev


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dev', action='store_true')
    parser.add_argument('--client-file', required=True, type=Path)
    args = parser.parse_args()
    values = dev.configuration(args.dev)
    try:
        client = json.loads(args.client_file.read_text())['web']
        if values['LOOSKI_PUBLIC_URL'] + '/auth/v1/callback' not in client['redirect_uris']:
            raise ValueError()
        credentials = {key: client[key] for key in ('client_id', 'client_secret')}
    except (ValueError, KeyError, TypeError, OSError):
        raise SystemExit('Expected a Google web-client JSON download with this installation’s /auth/v1/callback redirect URI. Values are not logged.')
    state = dev.ROOT / '.local' / values['COMPOSE_PROJECT_NAME']
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    # Validate before replacing any usable credential or enabling the provider.
    import tempfile
    with tempfile.TemporaryDirectory(dir=state) as temporary:
        candidate = Path(temporary) / 'google-oauth.json'
        dev.write_private(candidate, json.dumps(credentials))
        dev.google_credentials(candidate)
        dev.write_private(state / 'google-oauth.json', json.dumps(credentials))
    dev.configure_google({**values, 'LOOSKI_GOOGLE_AUTH_ENABLED': 'true'})
    config = dev.config_file(args.dev)
    lines = config.read_text().splitlines() if config.exists() else [f'{key}={values[key]}' for key in dev.KEYS]
    lines = [line for line in lines if not line.startswith('LOOSKI_GOOGLE_AUTH_ENABLED=')]
    dev.write_private(config, '\n'.join([*lines, 'LOOSKI_GOOGLE_AUTH_ENABLED=true']) + '\n')
    print('Google OAuth credential saved privately for ' + values['COMPOSE_PROJECT_NAME'] + '.')
    print('Run `make dev`.' if args.dev else 'Run `make up`.')


if __name__ == '__main__':
    main()
