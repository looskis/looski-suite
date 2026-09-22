import tempfile,unittest
from pathlib import Path
from migration_integrity import source_checksums,validate
class MigrationIntegrityTests(unittest.TestCase):
 def test_recorded_or_legacy_checksum_is_required(self):
  validate(['1'],{'1':'a'},{'1':'a','2':'b'},{})
  validate(['1'],{},{'1':'a'},{'1':'a'})
  validate(['1'],{'1':'a','2':'attempted'},{'1':'a','2':'corrected'}, {})
  with self.assertRaises(ValueError):validate(['1','2'],{'1':'a','2':'attempted'},{'1':'a','2':'corrected'}, {})
  for source,recorded,baseline in [({'1':'b'},{'1':'a'},{}),({}, {'1':'a'},{}),({'1':'a'},{},{})]:
   with self.assertRaises(ValueError):validate(['1'],recorded,source,baseline)
 def test_duplicate_versions_rejected_and_exact_bytes_hashed(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);(root/'1_one.sql').write_bytes(b'SELECT 1;\n')
   initial=source_checksums(root)
   (root/'1_one.sql').write_bytes(b'SELECT 1;')
   self.assertNotEqual(initial,source_checksums(root))
   (root/'1_two.sql').write_bytes(b'SELECT 2;')
   with self.assertRaises(ValueError):source_checksums(root)
if __name__=='__main__':unittest.main()
