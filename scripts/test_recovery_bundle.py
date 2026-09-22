import io,json,tarfile,tempfile,unittest
from pathlib import Path
from recovery_bundle import package,inspect,extract

class RecoveryBundleTests(unittest.TestCase):
 def bundle(self,root):
  source=root/'source';source.write_bytes(b'fixture bytes')
  bundle=root/'bundle.tar'
  package(bundle,{'database.dump':source,'roles.sql':source,'secrets.json':source,'recipe/Cargo.toml':source,'pgmq-meta.bin':source,'pgmq-queue.bin':source,'pgmq-archive.bin':source,'images.tar.gz':source},{'schema_version':28})
  return bundle
 def test_complete_bundle_is_verified_before_private_extraction(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);bundle=self.bundle(root)
   self.assertEqual(inspect(bundle)['metadata']['schema_version'],28)
   extract(bundle,root/'output');self.assertEqual((root/'output/database.dump').read_bytes(),b'fixture bytes')
   self.assertEqual((root/'output/database.dump').stat().st_mode&0o777,0o600)
   with self.assertRaises(FileExistsError):extract(bundle,root/'output')
 def test_corruption_is_rejected_without_destination(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);bundle=self.bundle(root)
   bundle.write_bytes(bundle.read_bytes().replace(b'fixture bytes',b'altered bytes',1))
   with self.assertRaises(ValueError):extract(bundle,root/'output')
   self.assertFalse((root/'output').exists())
 def test_paths_symlinks_and_duplicate_members_are_rejected(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory)
   for index,kind in enumerate(['traversal','symlink','duplicate']):
    bundle=root/(kind+'.tar')
    with tarfile.open(bundle,'w') as archive:
     info=tarfile.TarInfo('../escape' if kind=='traversal' else 'database.dump')
     if kind=='symlink':info.type=tarfile.SYMTYPE;info.linkname='/tmp/escape'
     archive.addfile(info)
     if kind=='duplicate':archive.addfile(info)
    with self.assertRaises(ValueError):extract(bundle,root/('output'+str(index)))
    self.assertFalse((root/('output'+str(index))).exists())
if __name__=='__main__':unittest.main()
