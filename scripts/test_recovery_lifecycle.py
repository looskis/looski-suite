import tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import recovery
from recovery_images import SERVICES

class FakeInstallation:
 def __init__(self,root):
  self.root=self.state=Path(root);self.calls=[]
 def running(self):return ['db','api','worker']
 def image_ids(self):return {s:'sha256:'+'a'*64 for s in SERVICES}
 def run(self,*args,**kwargs):self.calls.append(args)
 def start_services(self,services,require_healthy=False):self.calls.append(('start_exact',*services))

class RecoveryLifecycleTests(unittest.TestCase):
 def test_status_storage_failure_cannot_skip_writer_restart(self):
  with tempfile.TemporaryDirectory() as directory:
   i=FakeInstallation(directory)
   def status(_,document):
    if document['phase'] in ('capturing','restoring_services'):raise OSError('disk full')
   with patch.object(recovery,'status_write',side_effect=status),patch.object(recovery.image_bundle,'capture'),patch.object(recovery,'verify_migrations'):
    with self.assertRaises(OSError):recovery.create(i,Path(directory)/'backup.age','age1'+'a'*58)
   self.assertIn(('stop','-t','30','api','worker'),i.calls)
   self.assertIn(('start_exact','worker','api'),i.calls)
 def test_missing_service_image_rejects_before_maintenance(self):
  with tempfile.TemporaryDirectory() as directory:
   i=FakeInstallation(directory);i.image_ids=lambda:{'db':'sha256:'+'a'*64}
   with self.assertRaises(ValueError):recovery.create(i,Path(directory)/'backup.age','age1'+'a'*58)
   self.assertEqual(i.calls,[])
if __name__=='__main__':unittest.main()
