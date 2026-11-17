"""Actual compose argv against actual Common signature; fake proc files only."""
from pathlib import Path
from unittest.mock import patch
import ast,hashlib,os,sys,tempfile,unittest
import lifecycle as c

class CommonEntrySignature(unittest.TestCase):
 def test_actual_compose_argv_is_accepted_by_real_common_signature(self):
  source=Path(c.__file__).resolve().parents[4]/'extension/rag3weaver/scripts/confiner_validation.py'
  tree=ast.parse(source.read_text());signature=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='signature_superviseur')
  compose=next(n for n in ast.parse(Path(c.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='compose')
  entry=next(n for n in ast.walk(compose) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='rc' for t in n.targets))
  argv_node=entry.value.value.args[0]
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);script=root/'confiner_validation.py';script.write_bytes(source.read_bytes());proc=root/'123';proc.mkdir()
   namespace=dict(os=os,sys=sys,hashlib=hashlib,__file__=str(source),Path=lambda value:root if str(value)=='/proc' else Path(value))
   exec(compile(ast.Module(body=[signature],type_ignores=[]),'<actual-common-signature>','exec'),namespace)
   receipt=dict(script=str(script),script_sha256=hashlib.sha256(source.read_bytes()).hexdigest())
   def accepts(argv):
    (proc/'cmdline').write_bytes(b'\0'.join(os.fsencode(x) for x in argv)+b'\0')
    with patch.object(os,'readlink',return_value=sys.executable):return namespace['signature_superviseur'](123,receipt)
   self.assertTrue(accepts([sys.executable,str(script),'lancer']),'positive control of actual signature')
   argv=eval(compile(ast.Expression(argv_node),'<actual-compose-argv>','eval'),dict(cfg={'python':sys.executable},Q=root,S=root/'scope'))
   self.assertTrue(accepts(argv),repr(argv))
   self.assertEqual(argv[-4:],[sys.executable,'-B',str(root/'lifecycle.py'),'--controller'])
   environment=next(k.value for k in entry.value.value.keywords if k.arg=='env')
   actual_env=eval(compile(ast.Expression(environment),'<actual-common-env>','eval'),dict(os=os))
   self.assertEqual(actual_env['PYTHONDONTWRITEBYTECODE'],'1')

if __name__=='__main__':unittest.main()
