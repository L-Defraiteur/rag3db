"""Actual driver TMPDIR and actual Rust Unix suffix, arithmetic only, no bind."""
import ast,os,re,unittest,tempfile
from unittest.mock import patch
from pathlib import Path
import qualify as q

class TmpProfile(unittest.TestCase):
 def test_actual_driver_tmpdir_fits_the_actual_pont_socket_for_all_counter_widths(self):
  tree=ast.parse(Path(q.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  statement=next(n for n in driver.body if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and ast.unparse(n.value.func)=='env.update' and any(v.arg=='TMPDIR' for v in n.value.keywords))
  namespace=dict(vars(q),env={})
  exec(compile(ast.Module(body=[statement],type_ignores=[]),'<actual-driver-env>','exec'),namespace)
  tmp=Path(namespace['env']['TMPDIR']);root=Path(q.__file__).resolve().parents[4]
  source=(root/'extension/rag3weaver/src/pont_navigateur/temoins_relai.rs').read_text()
  prefix=re.search(r'format!\("([^"\n]+)", std::process::id\(\), N.fetch_add',source).group(1)
  self.assertEqual(prefix.count('{}'),2)
  suffix=re.search(r'let sock = d.join\("([^"\n]+)"\).join\("([^"\n]+)"\)',source)
  self.assertIsNotNone(suffix)
  paths=[tmp/prefix.format(pid,n)/suffix[1]/suffix[2] for pid,n in [(1,0),(9999999,0),(2**32-1,2**64-1)]]
  sizes=[len(os.fsencode(path)) for path in paths]
  self.assertTrue(all(n<=107 for n in sizes),f'Unix socket paths exceed sun_path: TMPDIR={tmp}, lengths={sizes}')
  self.assertTrue(tmp.is_relative_to(q.R));self.assertNotEqual(str(tmp),'/tmp')
 def test_private_tmp_creation_is_exclusive_and_its_identity_stays_pinned(self):
  source=Path(q.__file__).resolve().parents[4]
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);proofs=root/'proofs';proofs.mkdir();private=root/'t5'
   with patch.object(q,'R',root),patch.object(q,'P',proofs),patch.object(q,'W',source),patch.object(q,'SHORT_TMP',private):
    q.create_private_tmp();initial=q.tmp_state();q.private_tmp_guard()
    self.assertEqual(initial['mode'],0o700);self.assertEqual(initial['uid'],os.getuid())
    (private/'proof').write_bytes(b'preserve')
    with self.assertRaises(FileExistsError):q.create_private_tmp()
    self.assertEqual((private/'proof').read_bytes(),b'preserve')
    private.chmod(0o755)
    with self.assertRaises(AssertionError):q.private_tmp_guard()
    private.chmod(0o700);private.rename(root/'old-tmp-preserved');private.mkdir(mode=0o700)
    with self.assertRaises(AssertionError):q.private_tmp_guard()
    self.assertEqual((root/'old-tmp-preserved/proof').read_bytes(),b'preserve')
 def test_private_tmp_refuses_an_existing_symlink_without_creating_receipt(self):
  source=Path(q.__file__).resolve().parents[4]
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);proofs=root/'proofs';proofs.mkdir();target=root/'elsewhere';target.mkdir();alias=root/'t5';alias.symlink_to(target,target_is_directory=True)
   with patch.object(q,'R',root),patch.object(q,'P',proofs),patch.object(q,'W',source),patch.object(q,'SHORT_TMP',alias):
    with self.assertRaises((AssertionError,FileExistsError)):q.create_private_tmp()
    self.assertFalse((proofs/'tmp-profile.json').exists());self.assertEqual(list(target.iterdir()),[])

if __name__=='__main__':unittest.main(failfast=True)
