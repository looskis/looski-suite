import hashlib
import io
import json
import os
import tarfile
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

import cloudflare
import dev
import install
from download_release import unpack
from package_release import pin_compose

ACCOUNT = 'a' * 32
ZONE = 'b' * 32
ORIGIN = 'https://suite.example.com'
TUNNEL = '11111111-1111-4111-8111-111111111111'


class FakeCloudflare:
    def __init__(self):
        self.tunnel = None
        self.records = []
        self.config = {}
        self.creates = 0
        self.fail_after_create = False
        self.fail_after_dns = False
        self.calls = []

    def request(self, method, path, data=None):
        self.calls.append((method, path, data))
        if path == '/zones/' + ZONE:
            return {'account': {'id': ACCOUNT}, 'status': 'active', 'name': 'example.com'}
        if '/dns_records?' in path:
            return self.records.copy()
        if path.endswith('/dns_records') and method == 'POST':
            self.records = [{**data, 'id': 'record'}]
            if self.fail_after_dns:
                self.fail_after_dns = False
                raise RuntimeError('connection interrupted after DNS commit')
            return self.records[0]
        if '/cfd_tunnel?' in path:
            return [self.tunnel] if self.tunnel else []
        if path.endswith('/cfd_tunnel') and method == 'POST':
            self.creates += 1
            self.tunnel = {**data, 'id': TUNNEL, 'deleted_at': None}
            if self.fail_after_create:
                self.fail_after_create = False
                raise RuntimeError('connection interrupted after tunnel commit')
            return self.tunnel
        if path.endswith('/' + TUNNEL):
            return self.tunnel
        if path.endswith('/configurations'):
            if method == 'PUT': self.config = data['config']
            return {'config': self.config}
        if path.endswith('/token'):
            return 'private-connector-token'
        raise AssertionError((method, path))


class ProvisioningTests(unittest.TestCase):
    def test_hostname_discovers_authorized_zone_and_account_without_writes(self):
        class Discovery:
            calls = []
            def request(self, method, path):
                self.calls.append((method, path))
                name = parse_qs(urlsplit(path).query)['name'][0]
                return [{'id': ZONE, 'name': 'example.co.uk', 'status': 'active', 'account': {'id': ACCOUNT}}] if name == 'example.co.uk' else []
        client = Discovery()
        self.assertEqual(cloudflare.discover(client, 'https://suite.example.co.uk'), (ACCOUNT, ZONE))
        self.assertTrue(all(method == 'GET' for method, _ in client.calls))
        self.assertEqual(len(client.calls), 2)

    def test_discovery_rejects_inactive_ambiguous_and_apex_zones(self):
        from unittest.mock import Mock
        zone = {'id': ZONE, 'name': 'example.com', 'status': 'active', 'account': {'id': ACCOUNT}}
        for response, origin in [([zone, zone], ORIGIN), ([{**zone, 'status': 'pending'}], ORIGIN), ([zone], 'https://example.com'), ([], ORIGIN)]:
            client = Mock()
            client.request.return_value = response
            with self.subTest(response=response, origin=origin), self.assertRaises(ValueError):
                cloudflare.discover(client, origin)
        client.request.return_value = zone
        with self.assertRaisesRegex(ValueError, 'different account'):
            cloudflare.discover(client, ORIGIN, 'c' * 32, ZONE)

    def test_resume_after_each_external_commit_does_not_duplicate_resources(self):
        for failure in ('fail_after_create', 'fail_after_dns'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                state = Path(directory)
                client = FakeCloudflare()
                setattr(client, failure, True)
                with self.assertRaises(RuntimeError):
                    cloudflare.provision(client, state, ACCOUNT, ZONE, ORIGIN)
                result = cloudflare.provision(client, state, ACCOUNT, ZONE, ORIGIN)
                again = cloudflare.provision(client, state, ACCOUNT, ZONE, ORIGIN)
                self.assertEqual(result, again)
                self.assertEqual(client.creates, 1)
                self.assertEqual(len(client.records), 1)
                self.assertEqual((state / 'cloudflare-token').stat().st_mode & 0o777, 0o600)
                self.assertNotIn('private-connector-token', json.dumps(result))
                self.assertNotIn('private-connector-token', (state / 'cloudflare.json').read_text())

    def test_conflicting_dns_fails_before_any_external_write(self):
        with tempfile.TemporaryDirectory() as directory:
            client = FakeCloudflare()
            client.records = [{'type': 'A', 'content': '192.0.2.1'}]
            with self.assertRaisesRegex(ValueError, 'already has a DNS'):
                cloudflare.provision(client, Path(directory), ACCOUNT, ZONE, ORIGIN)
            self.assertTrue(all(method == 'GET' for method, _, _ in client.calls))

    def test_foreign_tunnel_changes_and_new_hostname_are_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory)
            client = FakeCloudflare()
            cloudflare.provision(client, state, ACCOUNT, ZONE, ORIGIN)
            client.config['ingress'][0]['service'] = 'http://other:8080'
            before = len(client.calls)
            with self.assertRaisesRegex(ValueError, 'changed outside'):
                cloudflare.provision(client, state, ACCOUNT, ZONE, ORIGIN)
            self.assertTrue(all(method == 'GET' for method, _, _ in client.calls[before:]))
            with self.assertRaisesRegex(ValueError, 'different Cloudflare route'):
                cloudflare.provision(client, state, ACCOUNT, ZONE, 'https://other.example.com')

    def test_token_file_must_be_private(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'token'
            path.write_text('secret' * 8)
            path.chmod(0o644)
            with self.assertRaises(ValueError): cloudflare.Cloudflare(path)
            path.chmod(0o600)
            cloudflare.Cloudflare(path)


class InstallationTests(unittest.TestCase):
    def test_readiness_wait_rechecks_without_restarting_or_reprovisioning(self):
        with patch('install.verify', side_effect=[{'ready': False}, {'ready': True}]) as verify, patch('install.time.sleep') as sleep, patch('install.time.monotonic', side_effect=[0, 1, 2]):
            self.assertTrue(install.wait_verify({}, 10)['ready'])
            self.assertEqual(verify.call_count, 2)
            sleep.assert_called_once_with(5)
        with self.assertRaises(ValueError): install.wait_verify({}, 901)

    def test_port_retry_preserves_project_and_issuer(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(dev, 'ROOT', Path(directory)), patch.dict(os.environ, {}, clear=True):
            original = dev.configuration()
            install.save_config(original)
            with patch('dev.configure'):
                changed = install.configure(Namespace(project='looski-suite', port=58999, url=None))
            self.assertEqual(changed['LOOSKI_PUBLIC_URL'], 'http://localhost:58999')
            self.assertEqual(changed['LOOSKI_AUTH_ISSUER'], original['LOOSKI_AUTH_ISSUER'])
            self.assertEqual(install.values()['LOOSKI_HTTP_PORT'], '58999')
            with self.assertRaises(ValueError):
                install.configure(Namespace(project='different', port=None, url=None))
            with self.assertRaises(SystemExit):
                install.configure(Namespace(project=None, port=0, url=None))

    def test_promotion_preserves_authentication_identity_and_ignores_ambient_overrides(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(dev, 'ROOT', Path(directory)):
            with patch.dict(os.environ, {}, clear=True):
                config = dev.configuration()
            config['LOOSKI_AUTH_ISSUER'] = 'http://localhost:58888/auth/v1'
            config['LOOSKI_PUBLIC_URL'] = ORIGIN
            config['LOOSKI_TUNNEL_MODE'] = 'cloudflare-managed'
            install.save_config(config)
            with patch.dict(os.environ, {'COMPOSE_PROJECT_NAME': 'unrelated', 'LOOSKI_PUBLIC_URL': 'https://other.example.com'}):
                loaded = install.values()
                self.assertEqual(loaded['LOOSKI_AUTH_ISSUER'], 'http://localhost:58888/auth/v1')
                self.assertEqual(loaded['LOOSKI_PUBLIC_URL'], ORIGIN)
                self.assertEqual(loaded['COMPOSE_PROJECT_NAME'], 'looski-suite')
                self.assertEqual(os.environ['COMPOSE_PROJECT_NAME'], 'unrelated')

    def test_healthy_http_is_not_reported_as_claimed_or_verified_email(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(dev, 'ROOT', Path(directory)), patch('install.owner_exists', return_value=False):
            config = {'COMPOSE_PROJECT_NAME': 'test', 'LOOSKI_PUBLIC_URL': 'https://suite.example.com', 'LOOSKI_MAIL_MODE': 'smtp'}
            class Response:
                status = 200
                def __enter__(self): return self
                def __exit__(self, *args): pass
                def read(self, *args): return json.dumps({'public_url': 'https://wrong.example.com'}).encode()
            with patch('install.build_opener') as opener, patch('install.services', return_value=[]):
                opener.return_value.open.return_value = Response()
                result = install.verify(config)
            self.assertFalse(result['ready'])
            self.assertEqual(result['owner'], 'awaiting_owner')
            self.assertEqual(result['email'], 'configured_unverified')
            self.assertEqual(result['backup'], 'not_configured')


class DistributionTests(unittest.TestCase):
    def archive(self, root, entries):
        path = root / 'release.tar.gz'
        with tarfile.open(path, 'w:gz') as archive:
            for name, content in entries:
                member = tarfile.TarInfo(name)
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
        return path, hashlib.sha256(path.read_bytes()).hexdigest()

    def test_verified_archive_extracts_once_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive, checksum = self.archive(root, [('release.json', b'{"format":"looski-release","version":"v0.1.0-preview.1"}'), ('compose.yaml', b'{}'), ('looski-admin', b'#!/bin/sh'), ('scripts/install.py', b'')])
            unpack(archive, root / 'installation', checksum)
            self.assertEqual((root / 'installation/looski-admin').stat().st_mode & 0o777, 0o755)
            with self.assertRaises(ValueError): unpack(archive, root / 'installation', checksum)
            with self.assertRaises(ValueError): unpack(archive, root / 'wrong', '0' * 64)
            self.assertFalse((root / 'wrong').exists())

    def test_traversal_duplicate_and_installation_state_are_rejected_before_extraction(self):
        for entries in [[('../escape', b'')], [('.local/secret', b'')], [('safe', b''), ('safe', b'')], [('.env', b'')]]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                archive, checksum = self.archive(root, entries)
                with self.assertRaises(ValueError): unpack(archive, root / 'installation', checksum)
                self.assertFalse((root / 'installation').exists())

    def test_release_requires_every_service_pinned_and_removes_builds(self):
        reference = 'ghcr.io/looskis/looski-suite-api@sha256:' + 'a' * 64
        document = {'services': {'api': {'build': {'context': '.'}}, 'worker': {}}}
        with self.assertRaises(ValueError): pin_compose(document, {'api': reference})
        document = pin_compose(document, {'api': reference, 'worker': reference})
        self.assertNotIn('build', document['services']['api'])
        self.assertEqual(document['services']['worker']['image'], reference)


if __name__ == '__main__':
    unittest.main()
