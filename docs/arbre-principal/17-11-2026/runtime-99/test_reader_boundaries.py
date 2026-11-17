"""Five causal witnesses BEFORE the grouped fix; written, NOT RUN.

Actual first_query/reader_exec bodies, fake clocks/subprocess/exec/proc only.
No native program, signal, process observation or database is used.
"""
from pathlib import Path as RealPath
import hashlib,io,json,tempfile,types,unittest
from unittest.mock import patch
import reader_runtime as runtime
import reader_exec

class ReaderBoundaries(unittest.TestCase):
    def invoke_query(self,mode):
        traces=[];clock=[100.];ticks=[0];records={}
        def now():
            ticks[0]+=1
            if mode=='two-now' and ticks[0]>=2:clock[0]=101.
            return clock[0]
        with tempfile.TemporaryDirectory() as directory:
            root=RealPath(directory);proof=root/'P';proof.mkdir();qdir=root/'Q';qdir.mkdir();elf=root/'typed';elf.write_bytes(b'PINNED READER')
            expected=hashlib.sha256(elf.read_bytes()).hexdigest()
            if mode=='promote':elf.write_bytes(b'SUBSTITUTED READER')
            def require_time(end,margin=15):
                if clock[0]>=end-margin:raise ValueError('fake deadline expired')
            def save(path,value):
                with path.open('x') as stream:json.dump(value,stream)
                if mode=='late-popen' and path.name=='command.json':clock[0]=201.
            typed=types.SimpleNamespace(consume=lambda stream,callback:dict(count=0,columns=1,rows={}))
            q=types.SimpleNamespace(P=proof,Q=qdir,BASE=proof/'copy/db',LIB=root/'sdk',GDB='/fake/gdb',PYTHON='/fake/python',READER_SHA=expected,PINS={elf:expected},typed_rows=typed,save=save,sha=lambda path:hashlib.sha256(RealPath(path).read_bytes()).hexdigest(),require_time=require_time)
            reader=types.SimpleNamespace(end=200.,counter=0,binary=elf,env=dict(HOME='/private'),timeout='/fake/timeout')
            class ProcFile:
                def read_text(self):return '0::/owned\n'
            def paths(path):return ProcFile() if str(path)=='/proc/self/cgroup' else RealPath(path)
            def child(argv,**kwargs):
                traces.append(dict(argv=argv,observed=clock[0]));folder=proof/'reader-sdk';cfg=json.loads((folder/'config.json').read_text());records.update(config=cfg)
                # The fake process agrees with the current executable, just as
                # real capture can: only an independent pin detects promotion.
                sample=dict(pid=123,start='42',inferior=1,exe=str(elf),exe_sha256=q.sha(elf),cgroup=cfg['cgroup'],argv=cfg['argv'],rawlen=100,expected=cfg['expected'],actual=cfg['expected'],diff={},present={key:True for key in cfg['expected']},maps='1 2 3 '+str(q.LIB/'librag3db.so'),native=['1 2 3 '+str(q.LIB/'librag3db.so')],observed=clock[0])
                sample['after']=dict(start='42',exe=str(elf),cgroup=cfg['cgroup'],exe_sha256=q.sha(elf))
                save(folder/'capture-exec.json',sample);save(folder/'capture-main.json',sample)
                save(folder/'native-exit.json',dict(pid=123,start='42',inferior=1,exit_code=0,observed=clock[0]))
                (folder/'native.stdout').write_bytes(b'fake consumed stream');(folder/'native.stderr').write_bytes(b'BUILD_ID 59ac9c86bc2b3529b590a7462ab7051e3ad26b8a')
                return types.SimpleNamespace(returncode=0)
            with patch.object(runtime,'Path',paths),patch.object(runtime.time,'monotonic',now),patch.object(runtime.subprocess,'run',child):
                if mode in ['promote','late-popen']:
                    with self.assertRaises(ValueError):runtime.first_query(reader,q,reader.env,'ONE QUERY',True)
                    self.assertEqual(len(traces),0,'native invocation must not begin after refusal/deadline')
                else:runtime.first_query(reader,q,reader.env,'ONE QUERY',True)
            return records,traces

    def invoke_exec(self,mode):
        clock=[100.];traces=[];original_hash=hashlib.sha256;original_open=reader_exec.os.open
        with tempfile.TemporaryDirectory() as directory:
            root=RealPath(directory);elf=root/'typed';elf.write_bytes(b'PINNED READER');proof=root/'proof';proof.mkdir();path=root/'config.json'
            cfg=dict(deadline=200.,argv=[str(elf),'/private/db','ONE QUERY'],sha256=original_hash(elf.read_bytes()).hexdigest(),proof=str(proof))
            path.write_text(json.dumps(cfg))
            def digest(raw):
                result=original_hash(raw)
                if mode=='hash':clock[0]=201.
                return result
            def opened(*args):
                fd=original_open(*args)
                if mode=='open':clock[0]=201.
                return fd
            def executed(*args):traces.append(dict(argv=args[1],observed=clock[0]))
            with patch.object(reader_exec.time,'monotonic',lambda:clock[0]),patch.object(reader_exec.hashlib,'sha256',digest),patch.object(reader_exec.os,'open',opened),patch.object(reader_exec.os,'dup2',lambda *args:None),patch.object(reader_exec.os,'execvpe',executed):
                with self.assertRaises(ValueError):reader_exec.main(path)
                self.assertEqual(traces,[],'exec must not start after effective deadline')

    def test_current_elf_hash_must_not_be_promoted_to_expected_pin(self):self.invoke_query('promote')
    def test_hash_crossing_deadline_must_refuse_before_exec(self):self.invoke_exec('hash')
    def test_open_crossing_deadline_must_refuse_before_exec(self):self.invoke_exec('open')
    def test_duration_and_deadline_must_use_one_observation(self):
        record,traces=self.invoke_query('two-now')
        self.assertEqual(len(traces),1)
        self.assertLessEqual(record['config']['deadline'],185.,'query deadline must not extend original END minus 15')
    def test_deadline_crossed_before_popen_must_not_start_native(self):self.invoke_query('late-popen')

if __name__=='__main__':unittest.main(failfast=False)
