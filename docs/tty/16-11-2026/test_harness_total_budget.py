"""Bound the actual allocation finalization tail, not only the Common subprocess."""
import ast
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import types
import unittest
import harness_only


def execute_real_finalization_tail(root):
    root = Path(root)
    q, p, target = root/'controller', root/'proofs', root/'target'
    q.mkdir(); p.mkdir(); target.mkdir()
    authority = root/'authority.json'
    authority.write_text(json.dumps(dict(etat='termine')))
    (root/'verdict.json').write_text(json.dumps(dict(verdict='VERT',code=0,restes=[],nettoye=True,prive=str(root/'absent-bus'))))
    (p/'authority-path.json').write_text(json.dumps(dict(path=str(authority))))
    (q/'target-ledger.json').write_text('{}')
    (p/'driver-result.json').write_text(json.dumps(dict(verdict='VERT')))
    source = ast.parse(Path(harness_only.__file__).read_text())
    name = 'allocation_worker' if hasattr(harness_only, 'allocation_worker') else 'main'
    function = next(node for node in source.body if isinstance(node,ast.FunctionDef) and node.name==name)
    start = next(i for i,node in enumerate(function.body) if isinstance(node,ast.Assign)
                 and any(isinstance(t,ast.Name) and t.id=='rc' for t in node.targets))
    tail = ast.Module(body=function.body[start:],type_ignores=[])
    def blocked_final_snapshot(_):
        time.sleep(3)  # the actual post-Common finalization step must be inside the watchdog
        return {}
    context = dict(Q=q,P=p,T=target,W=root,args=types.SimpleNamespace(source='fixture'),
                   config=dict(controllers={},carried={},sdk={},elf='fixture',elf_sha='fixture'),
                   start=time.monotonic(),end=time.monotonic()+0.2,record=dict(verdict='ROUGE'),
                   rc=2,env={},json=json,sys=sys,time=time,Path=Path,math=harness_only.math,
                   subprocess=types.SimpleNamespace(run=lambda *a,**k:types.SimpleNamespace(returncode=0)),
                   snapshot=blocked_final_snapshot,require_hashes=lambda *a:{},scan=lambda *a:dict(found=[]),
                   require_source=lambda *a:None,production_identity=lambda:None,
                   finish=harness_only.finish,publish_result=harness_only.publish_result)
    try:
        exec(compile(ast.fix_missing_locations(tail),'<real-finalization-tail>','exec'),context)
    except SystemExit:
        pass


class HarnessBudgetTests(unittest.TestCase):
    def test_actual_post_common_finalization_is_bounded_by_outer_watchdog(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            worker=root/'worker.py'
            worker.write_text('import sys\nsys.path.insert(0,'+repr(str(Path(__file__).parent))+')\n'
                              'from test_harness_total_budget import execute_real_finalization_tail\n'
                              'execute_real_finalization_tail('+repr(str(root))+')\n')
            if hasattr(harness_only,'outer_watchdog_command'):
                command=harness_only.outer_watchdog_command(worker,'fixture','fixture',time.monotonic(),0.2,0.05)
            else:
                command=[sys.executable,'-B',str(worker)]
            started=time.monotonic()
            result=subprocess.run(command,capture_output=True,text=True,timeout=5)
            self.assertLess(time.monotonic()-started,1.0,'finalization ran outside the actual outer watchdog')
            self.assertNotEqual(result.returncode,0)

    def test_historical_receipt_is_independent_of_new_controller_number(self):
        self.assertEqual(harness_only.PREVIOUS_SCOPE.name,'validation-flsb10fe')
        self.assertEqual(harness_only.PREVIOUS_SCOPE.parent.name,'cx-tty-aliases-continuation-common-v1')

    def test_actual_main_execs_watchdog_and_does_no_finalization_after_it(self):
        from unittest.mock import patch
        class ExecIntercept(Exception):
            pass
        captured = {}
        def intercept(program, argv):
            captured.update(program=program, argv=argv)
            raise ExecIntercept()
        argv = ['harness_only.py','--source','fixture','--allocation','hermetic-only']
        with patch.object(sys,'argv',argv), patch.object(harness_only.os,'execv',side_effect=intercept):
            with self.assertRaises(ExecIntercept):
                harness_only.main()
        self.assertEqual(captured['program'],'/usr/bin/timeout')
        self.assertIn('--worker',captured['argv'])
        self.assertIn(str(harness_only.Q/'harness_only.py'),captured['argv'])
        self.assertEqual(captured['argv'][-3],'--worker')


if __name__=='__main__':
    unittest.main()
