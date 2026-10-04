#!/usr/bin/env python3
"""Record this installation's SMTP account privately and select SMTP mail.

Prompts without echoing the password and writes .local/<installation>/smtp.json
with mode 0600. Run `make up` afterwards; only the Auth container receives it.
"""
import getpass
import json
import re

import dev


def ask(label, default=None):
    suffix = f' [{default}]' if default else ''
    value = input(f'{label}{suffix}: ').strip()
    return value or default or ''


def main():
    values = dev.configuration()
    state = dev.ROOT / '.local' / values['COMPOSE_PROJECT_NAME']
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    path = state / 'smtp.json'
    print(f"Installation: {values['COMPOSE_PROJECT_NAME']} | {values['LOOSKI_PUBLIC_URL']}")
    print('The provider must accept implicit TLS on port 465 and allow the sender address.')
    mail = {
        'host': ask('SMTP host (e.g. smtp.postmarkapp.com)'),
        'port': 465,
        'user': ask('SMTP username'),
        'password': getpass.getpass('SMTP password (not echoed): '),
        'from': ask('From address (e.g. accounts@loo.ski)'),
        'sender_name': ask('Sender name', 'Looski Suite'),
    }
    dev.write_private(path, json.dumps(mail))
    # Validate with the same rules setup applies; never print the values.
    dev.configure_mail({**values, 'LOOSKI_MAIL_MODE': 'smtp'})
    set_mail_mode(dev.ROOT / '.env')
    print(f'Saved {path.relative_to(dev.ROOT)} (mode 0600) and set LOOSKI_MAIL_MODE=smtp. Run `make up` to apply.')


def set_mail_mode(config):
    """Only the documented non-secret setting changes; other lines are kept."""
    lines = config.read_text().splitlines() if config.exists() else []
    lines = [line for line in lines if not re.match(r'LOOSKI_MAIL_MODE=', line)] + ['LOOSKI_MAIL_MODE=smtp']
    dev.write_private(config, '\n'.join(lines) + '\n')


if __name__ == '__main__':
    try:
        main()
    except (KeyboardInterrupt, EOFError):
        raise SystemExit('\nCancelled; nothing was changed.')
