"""Exercise the real launcher and armed worker with simulated process boundaries."""
from pathlib import Path
import os,tempfile,unittest
from types import SimpleNamespace
from unittest.mock import patch
import qualify_green as q

class Arm(unittest.TestCase):
    def test_actual_launch_arms_watchdog_before_allocation_publication(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);p=root/'proof';p.mkdir();control=Path(q.__file__).resolve().parents[4]
            runtime=root/'run';runtime.mkdir();(runtime/str(os.getuid())).mkdir()
            (runtime/str(os.getuid())/'bus').touch()
            clock=[100.];events=[];pilot='a'*40;config=dict(pilot=pilot,control=str(control),previous_expected_red={},cache={})
            def local_path(*parts):
                if parts==('/run/user',):return runtime
                return Path(*parts)
            def publish(path,value):
                events.append(('allocation',clock[0],value.copy()))
                clock[0]+=10.  # Simulated slow journal/flush, never a real sleep.
            def process(args,**kwargs):
                events.append(('watchdog',clock[0],list(args)))
                if args[4:]==[q.sys.executable,str(q.Q/'qualify_green.py'),'--arm',pilot]:
                    with patch.dict(os.environ,kwargs.get('env',dict(os.environ)),clear=True):q.arm(pilot)
                return SimpleNamespace(returncode=0)
            def execute(path,args,env):events.append(('poste',clock[0],list(args)))
            q.configure_controller()
            with patch.object(q,'P',p),patch.object(q,'Path',side_effect=local_path),\
                 patch.object(q,'cfg',return_value=config),patch.object(q,'guard'),\
                 patch.object(q.proof,'previous_expected_red',return_value={}),\
                 patch.object(q.proof,'cache_contract',return_value={}),patch.object(q.proof,'save',side_effect=publish),\
                 patch.object(q.controller,'snapshot',side_effect=[dict(stable=True,argv=q.controller.commands(control,pilot)['outer'],ppid=7),dict(stable=True,argv=['python3',str(control/q.REL),'--launch',pilot])]),\
                 patch.object(q.time,'monotonic',side_effect=lambda:clock[0]),\
                 patch.object(q.os,'getcwd',return_value='/var/tmp'),patch.object(Path,'is_socket',return_value=True),\
                 patch.object(q.os,'dup2'),patch.object(q.os,'execve',side_effect=execute),\
                 patch.object(q.subprocess,'run',side_effect=process),\
                 patch.object(q.sys,'orig_argv',['python3',str(control/q.REL),'--launch',pilot]),\
                 patch.dict(os.environ,dict(HOME=str(root),PATH='/usr/bin',RAG3WEAVER_C7_ALLOCATION='CPU-only'),clear=True):
                result=q.launch(pilot)
            self.assertEqual(events[0][0],'watchdog','no allocation journal may precede the real watchdog spawn')
            self.assertEqual([row[0] for row in events],['watchdog','allocation','poste'])
            record=events[1][2]
            self.assertGreaterEqual(record['whole_start'],events[0][1])
            self.assertEqual(record['soft_end'],record['whole_start']+780)
            self.assertEqual(record['hard_end'],record['whole_start']+795)
            self.assertEqual(result,0)

if __name__=='__main__':unittest.main(failfast=True)
