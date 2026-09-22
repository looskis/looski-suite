import copy
import json
from pathlib import Path
import tempfile
import unittest
from export_collection import collect,canonical_selection

WORKSPACE='00000000-0000-0000-0000-000000000001'
CURSOR='00000000-0000-0000-0000-000000000002'
class ExportArtifactTests(unittest.TestCase):
    def page(self,selection=None,next=None):
        return {'manifest':{'format':'looski-business-json','format_version':1,'application_version':'test','workspace_id':WORKSPACE,'selection':canonical_selection(selection or {'collection':'contacts'}),'consistency':'live_authorized_pages'},'items':[{'value':'=keep exactly\n雪'}],'next':next}
    def test_failed_later_page_leaves_no_artifact_or_plaintext_temporary(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'export.json';calls=[]
            def fetch(selection):
                calls.append(selection)
                if len(calls)==2:raise ValueError('Permission revoked')
                return self.page(selection,{'collection':'contacts','after':CURSOR})
            with self.assertRaisesRegex(ValueError,'Permission revoked'):collect(fetch,{'collection':'contacts'},path,workspace=WORKSPACE)
            self.assertEqual(list(Path(directory).iterdir()),[])
    def test_completed_private_package_and_existing_output_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'export.json'
            result=collect(lambda selection:self.page(selection),{'collection':'contacts'},path,workspace=WORKSPACE)
            self.assertEqual(result['pages'],1)
            data=json.loads(path.read_text());self.assertTrue(data['complete'])
            self.assertEqual(data['pages'][0]['items'][0]['value'],'=keep exactly\n雪')
            self.assertEqual(path.stat().st_mode&0o777,0o600)
            before=path.read_bytes()
            with self.assertRaisesRegex(ValueError,'already exists'):collect(lambda _:self.page(),{'collection':'contacts'},path,workspace=WORKSPACE)
            self.assertEqual(path.read_bytes(),before)
    def test_page_bound_and_repeated_continuation_do_not_publish(self):
        for limit in [1,3]:
            with tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(ValueError):collect(lambda selection:self.page(selection,{'collection':'contacts','after':CURSOR}),{'collection':'contacts'},Path(directory)/'export.json',limit,workspace=WORKSPACE)
                self.assertEqual(list(Path(directory).iterdir()),[])
    def test_mismatched_selection_scope_and_response_do_not_publish(self):
        good=self.page(next={'collection':'contacts','after':CURSOR})
        cases=[]
        for location,field,value in [('next','collection','deals'),('next','parent_id',CURSOR),('next','archived',True),('selection','collection','deals'),('selection','parent_id',CURSOR),('manifest','workspace_id',CURSOR),('manifest','consistency','snapshot')]:
            bad=copy.deepcopy(good)
            target=bad['manifest']['selection'] if location=='selection' else bad[location]
            target[field]=value;cases.append(bad)
        missing=copy.deepcopy(good);del missing['manifest']['selection'];cases.append(missing)
        malformed=copy.deepcopy(good);malformed['items']={'not':'a list'};cases.append(malformed)
        for response in cases:
            with self.subTest(response=response),tempfile.TemporaryDirectory() as directory:
                responses=iter([response,self.page()])
                with self.assertRaises(ValueError):collect(lambda _:next(responses),{'collection':'contacts'},Path(directory)/'export.json',workspace=WORKSPACE)
                self.assertEqual(list(Path(directory).iterdir()),[])

if __name__=='__main__':unittest.main()
