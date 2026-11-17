"""Written, NOT RUN: actual first_query with fake subprocess/proc only."""
from pathlib import Path as RealPath
import hashlib,importlib.util,io,json,struct,tempfile,time,types,unittest
from unittest.mock import patch
import reader_runtime as r

class FirstQuery(unittest.TestCase):
    def run_case(self,failure=None):
        # The actual typed framing source, with byte pin; no ambient module.
        path=RealPath(__file__).resolve().parents[2]/'16-11-2026/debt-copy-a/typed_rows.py'
        if hashlib.sha256(path.read_bytes()).hexdigest()!='41314cda227b3511f2daed3695d894bec150f3ee3dfc2fd7b6fa268db1374916':raise AssertionError('typed fixture source pin')
        spec=importlib.util.spec_from_file_location('pinned_typed_rows',path);typed=importlib.util.module_from_spec(spec);spec.loader.exec_module(typed)
        def binary(value):return struct.pack('<Q',len(value))+value
        cell=binary(b'STRING')+b'1s'+binary(b'a\0|\nb')
        protocol=b'R3TYP002'+struct.pack('<QQ',1,1)+binary(cell)
        trace=[]
        with tempfile.TemporaryDirectory() as directory:
            root=RealPath(directory);p=root/'P';p.mkdir();qdir=root/'Q';qdir.mkdir();lib=root/'sdk';lib.mkdir()
            elf=root/'typed';elf.write_bytes(b'fake ELF')
            def save(path,value):
                with path.open('x') as out:json.dump(value,out)
            q=types.SimpleNamespace(P=p,Q=qdir,BASE=p/'copy/db',LIB=lib,GDB='/fake/gdb',PYTHON='/fake/python',typed_rows=typed,sha=lambda path:hashlib.sha256(RealPath(path).read_bytes()).hexdigest(),save=save,require_time=lambda *args:None)
            reader=types.SimpleNamespace(end=time.monotonic()+300,counter=0,binary=elf,env=dict(HOME='/private'),timeout='/fake/timeout')
            class ProcFile:
                def read_text(self):return '0::/owned\n'
            def paths(value):return ProcFile() if str(value)=='/proc/self/cgroup' else RealPath(value)
            def child(argv,**kwargs):
                trace.append(argv);proof=p/'reader-sdk';cfg=json.loads((proof/'config.json').read_text());now=time.monotonic()
                row=dict(stage='exec',pid=123,start='42',inferior=1,exe=str(elf),exe_sha256=q.sha(elf),cgroup=cfg['cgroup'],argv=cfg['argv'],rawlen=100,expected=cfg['expected'],actual=cfg['expected'],diff={},present={key:True for key in cfg['expected']},maps='1 2 3 '+str(lib/'librag3db.so'),native=['1 2 3 '+str(lib/'librag3db.so')],observed=now)
                row['after']=dict(start='42',exe=str(elf),cgroup=cfg['cgroup'],exe_sha256=q.sha(elf))
                save(proof/'capture-exec.json',row);row=dict(row,stage='main',observed=now+.001)
                if failure=='cgroup':row['cgroup']='0::/foreign\n'
                save(proof/'capture-main.json',row)
                save(proof/'native-exit.json',dict(pid=123,start='42',inferior=1,exit_code=2 if failure=='native' else 0,observed=now+.002))
                (proof/'native.stdout').write_bytes(protocol);(proof/'native.stderr').write_bytes(b'BUILD_ID 59ac9c86bc2b3529b590a7462ab7051e3ad26b8a')
                return types.SimpleNamespace(returncode=1 if failure=='gdb' else 0)
            with patch.object(r,'Path',paths),patch.object(r.subprocess,'run',child):
                if failure:
                    with self.assertRaises(ValueError):r.first_query(reader,q,reader.env,'ONE REAL QUERY',True)
                else:
                    result=r.first_query(reader,q,reader.env,'ONE REAL QUERY',True)
                    self.assertEqual(result,[[('STRING',b'a\0|\nb')]])
                    script=(p/'reader-sdk/capture.gdb').read_text()
                    self.assertEqual(script.count('catch exec'),1);self.assertIn("capture('exec',config)",script);self.assertIn("capture('main',config)",script);self.assertIn('install_exit(config,main)',script)
            self.assertEqual(len(trace),1);self.assertEqual(reader.counter,1)
            self.assertEqual(json.loads((p/'reader-sdk/config.json').read_text())['argv'],[str(elf),str(q.BASE),'ONE REAL QUERY'])
            self.assertTrue((p/'reader-sdk/native.stdout').exists())
            return trace
    def test_actual_first_query_consumes_native_protocol_in_single_invocation(self):self.run_case()
    def test_gdb_nonzero_preserves_raw_and_refuses(self):self.run_case('gdb')
    def test_gdb_zero_does_not_hide_native_nonzero(self):self.run_case('native')
    def test_real_first_query_rejects_wrong_cgroup(self):self.run_case('cgroup')

if __name__=='__main__':unittest.main(failfast=True)
