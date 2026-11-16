"""Actual consumer expressions, fake providers/clocks only; no network or process."""
import ast,contextlib,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import qualify as q

class ReviewReserves(unittest.TestCase):
 def body(self):
  return next(n for n in ast.parse(Path(q.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='driver').body
 def test_actual_check_features_suffix_is_accepted(self):
  assertion=next(n for n in self.body() if isinstance(n,ast.Assert) and 'check_features.log' in ast.unparse(n))
  with tempfile.TemporaryDirectory() as d:
   official=Path(d);(official/'check_features.log').write_text('CHECK_FEATURES VERT 8/8\n')
   exec(compile(ast.Module(body=[assertion],type_ignores=[]),'<actual-features-assert>','exec'),dict(vars(q),official=official))
 def test_first_refusal_after_first_get_forbids_second_contact(self):
  call=next(n for n in self.body() if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and ast.unparse(n.value.func)=='health' and 'services-before' in ast.unparse(n))
  contacts=[];uid=q.os.getuid()
  ready=dict(clock='CLOCK_MONOTONIC@luciepc',ready_observed_monotonic=40.,grant_deadline_monotonic=100.,deadline=600.,nonce='fake',phase=dict(job='lifecycle-provider',pilot='a'*40,allocation='fake',authority=dict(path='/fake',pid=123,start='42',cgroup='/fake',uid=uid,script_sha256='a'*64),first_red_path='/fake/red'))
  def phase(_):
   return dict(clock='CLOCK_MONOTONIC@luciepc',snapshot_stable=True,first_red_exists=bool(contacts),snapshot_observed_monotonic=70.,authority=dict(etat='actif',pid=123,debut='42',cgroup='/fake',script_sha256='a'*64,echeance=600.),authority_uid=uid,process=dict(start='42',cgroup='/fake',uid=uid,state='S'))
  grant=dict(valid_until_monotonic=600.,providers=[dict(address='127.0.0.1:7878',identity=dict(model='first')),dict(address='127.0.0.1:7879',identity=dict(model='second'))])
  def response(url,**kwargs):
   contacts.append(url);row=grant['providers'][len(contacts)-1]
   class Response:
    status=200
    def read(self,n):return json.dumps(dict(row['identity'],usage=dict(baux=0,en_cours=0))).encode()
    def __enter__(self):return self
    def __exit__(self,*args):pass
   return Response()
  with tempfile.TemporaryDirectory() as d,patch.object(q,'P',Path(d)),patch.object(q.time,'monotonic',return_value=70.),patch.object(q,'provider_stable'),patch.object(q.ready_protocol,'read_phase_state',side_effect=phase),patch.object(q.urllib.request,'urlopen',side_effect=response):
   with self.assertRaises(ValueError):exec(compile(ast.Module(body=[call],type_ignores=[]),'<actual-services-before>','exec'),dict(vars(q),grant=grant,end=600.,ready=ready))
  self.assertEqual(len(contacts),1)

if __name__=='__main__':unittest.main()
