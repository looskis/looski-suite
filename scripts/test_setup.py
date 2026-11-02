import json
import os
from pathlib import Path
import tempfile
import unittest
import contextlib
import io
from unittest.mock import patch

import dev


class SetupConfigurationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.root_patch = patch.object(dev, 'ROOT', self.root)
        self.env_patch = patch.dict(os.environ, {}, clear=True)
        self.root_patch.start()
        self.env_patch.start()

    def tearDown(self):
        self.env_patch.stop()
        self.root_patch.stop()
        self.directory.cleanup()

    def test_port_override_derives_matching_origin(self):
        with patch.dict(os.environ, {'LOOSKI_HTTP_PORT': '58889', 'COMPOSE_PROJECT_NAME': 'looski-suite-demo'}):
            config = dev.configuration()
        self.assertEqual(config['LOOSKI_PUBLIC_URL'], 'http://localhost:58889')

    def test_interrupted_first_secret_publication_can_retry(self):
        with patch('dev.subprocess.check_output', return_value=''), patch('dev.os.rename', side_effect=OSError('interrupted')):
            with self.assertRaises(OSError):
                dev.configure(dev.configuration(), dev.config_file())
        directory = self.root / '.local/looski-suite/secrets'
        self.assertFalse(directory.exists())
        with patch('dev.subprocess.check_output', return_value=''), contextlib.redirect_stdout(io.StringIO()):
            dev.configure(dev.configuration(), dev.config_file())
        self.assertEqual(len(list(directory.iterdir())), 5)
        self.assertTrue(all(path.stat().st_mode & 0o777 == 0o600 for path in directory.iterdir()))

    def test_rejects_mismatched_callback_port(self):
        (self.root / '.env').write_text('LOOSKI_HTTP_PORT=58889\nLOOSKI_PUBLIC_URL=http://localhost:58888\n')
        with self.assertRaises(SystemExit):
            dev.configuration()

    def test_rejects_credentials_and_invalid_ports(self):
        for values in [{'LOOSKI_HTTP_PORT': '88888'}, {'LOOSKI_PUBLIC_URL': 'https://secret@example.com'},
                       {'COMPOSE_PROJECT_NAME': '../another-installation'}]:
            with self.subTest(values=values), patch.dict(os.environ, values), self.assertRaises(SystemExit):
                dev.configuration()

    def test_development_stack_is_a_separate_installation(self):
        (self.root / '.env').write_text('COMPOSE_PROJECT_NAME=looski-suite\nLOOSKI_PUBLIC_URL=http://localhost:58888\n')
        config = dev.configuration(dev=True)
        self.assertEqual((config['COMPOSE_PROJECT_NAME'], config['LOOSKI_PUBLIC_URL']),
                         ('looski-suite-dev', 'http://localhost:58889'))
        (self.root / '.env.dev').write_text('LOOSKI_HTTP_PORT=58890\nLOOSKI_PUBLIC_URL=http://localhost:58890\n')
        self.assertEqual(dev.configuration(dev=True)['LOOSKI_HTTP_PORT'], '58890')
        self.assertEqual(dev.configuration()['LOOSKI_HTTP_PORT'], '58888')

    def test_development_stack_never_targets_the_primary_installation(self):
        for primary, values in [('', {'COMPOSE_PROJECT_NAME': 'looski-suite'}),
                                ('COMPOSE_PROJECT_NAME=business\n', {'COMPOSE_PROJECT_NAME': 'business'})]:
            (self.root / '.env').write_text(primary)
            with self.subTest(values=values), patch.dict(os.environ, values), self.assertRaises(SystemExit):
                dev.configuration(dev=True)

    def test_partial_secrets_are_not_rotated(self):
        directory = self.root / '.local/looski-suite/secrets'
        directory.mkdir(parents=True)
        (directory / 'db_password').write_text('existing')
        with self.assertRaises(SystemExit):
            dev.configure(dev.configuration(), dev.config_file())
        self.assertEqual((directory / 'db_password').read_text(), 'existing')

    def test_compose_configuration_keeps_mail_mode_across_service_restarts(self):
        for mode,enabled in [('disabled','false'),('local','true'),('smtp','true')]:
            with patch.dict(os.environ,{'LOOSKI_MAIL_MODE':mode,'LOOSKI_AUTH_MAIL_ENABLED':'wrong'}):
                config=dev.configuration()
                self.assertEqual(config['LOOSKI_AUTH_MAIL_ENABLED'],enabled)

    def test_local_mail_cannot_use_public_origin(self):
        with patch.dict(os.environ, {'LOOSKI_MAIL_MODE':'local','LOOSKI_PUBLIC_URL':'https://app.example.com'}), self.assertRaises(SystemExit):
            dev.configuration()

    def test_smtp_requires_private_valid_credentials_without_leaking_them(self):
        state=self.root/'.local/looski-suite';state.mkdir(parents=True)
        config={**dev.configuration(),'LOOSKI_MAIL_MODE':'smtp'}
        path=state/'smtp.json'
        secret='private-test-smtp-password'
        values={'host':'smtp.example.com','port':465,'user':'sender','password':secret,'from':'accounts@example.com','sender_name':'Looski Suite'}
        path.write_text(json.dumps(values));path.chmod(0o644)
        with self.assertRaises(SystemExit) as failure: dev.configure_mail(config)
        self.assertNotIn(secret,str(failure.exception))
        path.chmod(0o600)
        dev.configure_mail(config)
        output=state/'auth-mail.json'
        self.assertEqual(output.stat().st_mode & 0o777,0o600)
        self.assertEqual(json.loads(output.read_text()),values)
        for changed in [{'port':25},{'port':587},{'from':'injected\r\nBcc: x@example.com'},{'host':'smtp.example.com/path'},{'password':None}]:
            path.write_text(json.dumps({**values,**changed}))
            with self.assertRaises(SystemExit) as failure: dev.configure_mail(config)
            self.assertNotIn(secret,str(failure.exception))
            self.assertEqual(json.loads(output.read_text()),values)

    def test_tunnel_requires_https_origin_and_private_credentials(self):
        for values in [{'LOOSKI_TUNNEL_MODE':'cloudflare'},{'LOOSKI_TUNNEL_MODE':'cloudflare','LOOSKI_PUBLIC_URL':'https://suite.example.com:8443'},
                       {'LOOSKI_TUNNEL_MODE':'ngrok','LOOSKI_PUBLIC_URL':'https://suite.example.com'}]:
            with self.subTest(values=values), patch.dict(os.environ, values), self.assertRaises(SystemExit):
                dev.configuration()
        state=self.root/'.local/looski-suite';state.mkdir(parents=True)
        with patch.dict(os.environ,{'LOOSKI_TUNNEL_MODE':'cloudflare','LOOSKI_PUBLIC_URL':'https://suite.example.com'}):
            config=dev.configuration()
        path=state/'cloudflare-tunnel.json'
        secret='private-tunnel-secret'
        credentials={'AccountTag':'account','TunnelSecret':secret,'TunnelID':'6f1c2d4e-1111-4222-8333-444455556666'}
        path.write_text(json.dumps(credentials));path.chmod(0o644)
        with self.assertRaises(SystemExit) as failure: dev.configure_tunnel(config)
        self.assertNotIn(secret,str(failure.exception))
        path.chmod(0o600)
        dev.configure_tunnel(config)
        rendered=(state/'cloudflared.yml').read_text()
        self.assertEqual((state/'cloudflared.yml').stat().st_mode & 0o777,0o600)
        self.assertIn('tunnel: 6f1c2d4e-1111-4222-8333-444455556666',rendered)
        self.assertIn('hostname: suite.example.com\n    service: http://gateway:8080',rendered)
        self.assertNotIn(secret,rendered)
        for changed in [{'TunnelID':'../other'},{'TunnelSecret':''}]:
            path.write_text(json.dumps({**credentials,**changed}))
            with self.assertRaises(SystemExit) as failure: dev.configure_tunnel(config)
            self.assertNotIn(secret,str(failure.exception))

    def test_disabled_tunnel_never_replaces_existing_credentials(self):
        state=self.root/'.local/looski-suite';state.mkdir(parents=True)
        path=state/'cloudflare-tunnel.json'
        path.write_text('{"kept":true}')
        dev.configure_tunnel(dev.configuration())
        self.assertEqual(path.read_text(),'{"kept":true}')

    def test_mail_modes_replace_only_the_mail_secret(self):
        state=self.root/'.local/looski-suite';state.mkdir(parents=True)
        config=dev.configuration()
        dev.configure_mail({**config,'LOOSKI_MAIL_MODE':'local'})
        self.assertEqual(json.loads((state/'auth-mail.json').read_text())['host'],'mailpit')
        dev.configure_mail(config)
        self.assertEqual(json.loads((state/'auth-mail.json').read_text())['port'],1)


if __name__ == '__main__':
    unittest.main()
