import unittest
from recovery_restore import role_recipe
class RecoveryRolesTests(unittest.TestCase):
 def test_creation_is_separated_from_preserved_dump_attributes(self):
  sql='CREATE ROLE postgres;\nALTER ROLE postgres WITH LOGIN;\nCREATE ROLE "A quoted role";\nGRANT postgres TO "A quoted role";\n'
  result=role_recipe(sql)
  self.assertNotIn('CREATE ROLE postgres;',result)
  self.assertIn("format('CREATE ROLE %I',name)",result)
  self.assertIn('ALTER ROLE postgres WITH LOGIN;',result)
  self.assertIn('GRANT postgres TO "A quoted role";',result)
 def test_multiline_or_unparseable_create_is_rejected(self):
  for sql in ['CREATE ROLE "multiline\nrole";','CREATE ROLE user WITH SUPERUSER;','-- no role definitions']:
   with self.assertRaises(ValueError):role_recipe(sql)
if __name__=='__main__':unittest.main()
