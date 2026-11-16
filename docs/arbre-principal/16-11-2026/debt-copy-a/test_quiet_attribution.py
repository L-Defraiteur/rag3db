"""Actual quiescence helper against synthetic proc objects only."""
import types,unittest
from unittest.mock import patch
import qualify as q

class FakePath:
 def __init__(self,name):self.key=name;self.name=name.rsplit('/',1)[-1]
 def __truediv__(self,key):return FakePath(self.key+'/'+str(key))
 def stat(self):return types.SimpleNamespace(st_uid=q.os.getuid())
 def read_text(self):return '123 (fake) '+' '.join(['R']+['0']*18+['42'])
 def iterdir(self):
  if self.key=='/proc':return iter([FakePath('/proc/123')])
  if self.key.endswith('/fd'):return iter([self/'1',self/'2'])
  raise AssertionError(self.key)

class QuietAttribution(unittest.TestCase):
 def test_observed_exe_then_cwd_denied_remains_attributed(self):
  def readlink(path):
   if path.key.endswith('/exe'):return str(q.BASE)
   raise PermissionError('cwd denied fixture')
  with patch.object(q,'Path',FakePath),patch.object(q.os,'readlink',readlink),self.assertRaisesRegex(ValueError,'occupied or attributed UNKNOWN'):q.copy_quiet()
 def test_observed_fd_then_later_fd_denied_remains_attributed(self):
  def readlink(path):
   if path.key.endswith('/fd/1'):return str(q.BASE)
   if path.key.endswith('/fd/2'):raise PermissionError('later FD denied fixture')
   return '/outside'
  with patch.object(q,'Path',FakePath),patch.object(q.os,'readlink',readlink),self.assertRaisesRegex(ValueError,'occupied or attributed UNKNOWN'):q.copy_quiet()
 def test_unattributed_unknown_is_recorded_not_invented_as_negative(self):
  with patch.object(q,'Path',FakePath),patch.object(q.os,'readlink',side_effect=PermissionError('exe denied fixture')):
   result=q.copy_quiet()
  self.assertEqual(result['found'],[]);self.assertEqual(len(result['unknown']),1)
  self.assertFalse(result['unknown'][0]['attributed']);self.assertEqual(result['unknown'][0]['start'],'42')

if __name__=='__main__':unittest.main(failfast=True)
