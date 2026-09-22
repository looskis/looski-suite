import json
import os
from pathlib import Path
import tempfile
import unittest
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

    def test_rejects_mismatched_callback_port(self):
        (self.root / '.env').write_text('LOOSKI_HTTP_PORT=58889\nLOOSKI_PUBLIC_URL=http://localhost:58888\n')
        with self.assertRaises(SystemExit):
            dev.configuration()

    def test_rejects_credentials_and_invalid_ports(self):
        for values in [{'LOOSKI_HTTP_PORT': '88888'}, {'LOOSKI_PUBLIC_URL': 'https://secret@example.com'},
                       {'COMPOSE_PROJECT_NAME': '../another-installation'}]:
            with self.subTest(values=values), patch.dict(os.environ, values), self.assertRaises(SystemExit):
                dev.configuration()

    def test_partial_secrets_are_not_rotated(self):
        directory = self.root / '.local/looski-suite/secrets'
        directory.mkdir(parents=True)
        (directory / 'db_password').write_text('existing')
        with self.assertRaises(SystemExit):
            dev.configure(dev.configuration())
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

    def test_mail_modes_replace_only_the_mail_secret(self):
        state=self.root/'.local/looski-suite';state.mkdir(parents=True)
        config=dev.configuration()
        dev.configure_mail({**config,'LOOSKI_MAIL_MODE':'local'})
        self.assertEqual(json.loads((state/'auth-mail.json').read_text())['host'],'mailpit')
        dev.configure_mail(config)
        self.assertEqual(json.loads((state/'auth-mail.json').read_text())['port'],1)


if __name__ == '__main__':
    unittest.main()
