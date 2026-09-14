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


if __name__ == '__main__':
    unittest.main()
