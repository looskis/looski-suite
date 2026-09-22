"""Failure/publication checks independent of Docker and real recovery fixtures."""
import os
import tempfile
import unittest
from pathlib import Path
from recovery_archive import publish, private_regular, validate_recipient

class RecoveryPublicationTests(unittest.TestCase):
    def test_publish_is_private_and_existing_or_failed_outputs_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            destination=Path(directory)/'backup.age'
            publish(destination, lambda output:output.write(b'complete'))
            self.assertEqual(destination.stat().st_mode&0o777,0o600)
            with self.assertRaises(FileExistsError):publish(destination,lambda output:output.write(b'replacement'))
            self.assertEqual(destination.read_bytes(),b'complete')
            def failure(output):
                output.write(b'partial')
                raise RuntimeError('interrupted')
            with self.assertRaises(RuntimeError):publish(Path(directory)/'failed.age',failure)
            self.assertEqual([p.name for p in Path(directory).iterdir()],['backup.age'])
    def test_identity_must_be_a_small_private_regular_file(self):
        with tempfile.TemporaryDirectory() as directory:
            identity=Path(directory)/'identity';identity.write_bytes(b'test');identity.chmod(0o600)
            self.assertEqual(private_regular(identity),b'test')
            link=Path(directory)/'link';link.symlink_to(identity)
            with self.assertRaises(OSError):private_regular(link)
            identity.chmod(0o644)
            with self.assertRaises(ValueError):private_regular(identity)
            identity.chmod(0o600);identity.write_bytes(b'x'*16385)
            with self.assertRaises(ValueError):private_regular(identity)
    def test_public_recipient_is_not_a_private_key_or_option(self):
        for value in ['AGE-SECRET-KEY-secret','--passphrase','age1bad','age1'+'a'*58+'\n']:
            with self.assertRaises(ValueError):validate_recipient(value)
if __name__=='__main__':unittest.main()
