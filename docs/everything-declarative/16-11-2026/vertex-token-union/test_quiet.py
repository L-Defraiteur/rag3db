"""Actual quiet() AST, all proc/UID/argv/FD observations are synthetic."""
import ast,io,json,types,unittest
from pathlib import Path

HERE=Path(__file__).parent
ROOT='/cache';Q=ROOT+'/control';W='/work/source';T=ROOT+'/target';P=ROOT+'/proofs'
CFG=dict(timeout='/usr/bin/timeout',bash='/usr/bin/bash',bash_sha256='BASH')
ARGV=['bash',Q+'/poste','lourd',CFG['timeout'],'--signal=TERM','--kill-after=2s','1198s','/usr/bin/python3',Q+'/qualify.py','--compose']

def observe(case):
    def stat_text(pid):
        fields=['S']+['0']*18+[str({200:20,300:30,400:40}[pid])]
        fields[1]=str({200:300,300:1,400:1}[pid])
        return str(pid)+' (synthetic) '+' '.join(fields)
    class FakePath:
        def __init__(self,path):self.path=str(path)
        def __str__(self):return self.path
        def __fspath__(self):return self.path
        def __truediv__(self,part):return FakePath(self.path+'/'+str(part))
        @property
        def name(self):return self.path.rsplit('/',1)[-1]
        def exists(self):return False
        def is_file(self):return False
        def stat(self):return types.SimpleNamespace(st_uid=1000)
        def read_text(self):
            if self.path==Q+'/configuration.json':return json.dumps(CFG)
            if self.path.endswith('/stat'):return stat_text(int(self.path.split('/')[2]))
            raise AssertionError(self.path)
        def read_bytes(self):
            if self.path.endswith('/cmdline'):
                argv=list(ARGV)
                if case=='wrong-command':argv[2]='autre-mode'
                return b'\0'.join(s.encode() for s in argv)+b'\0'
            raise AssertionError(self.path)
        def iterdir(self):
            if self.path=='/proc':return iter([FakePath('/proc/300'),FakePath('/proc/400')])
            if self.path.endswith('/fd'):
                pid=int(self.path.split('/')[2])
                return iter([self/'255'] if pid==300 else [])
            raise AssertionError(self.path)
    def readlink(path):
        path=str(path);pid=int(path.split('/')[2]);role=path.rsplit('/',1)[-1]
        if role=='exe':return '/usr/bin/bash' if pid==300 else (T+'/leak' if case=='permission' else '/usr/bin/idle')
        if role=='cwd':
            if pid==400 and case=='permission':raise PermissionError('protected after attributed exe')
            return '/var/tmp'
        if role=='255':return Q+'/poste' if case!='foreign-fd' else T+'/retained-data'
        raise AssertionError(path)
    fake_os=types.SimpleNamespace(getpid=lambda:200,getuid=lambda:1000,readlink=readlink,fsdecode=lambda x:x.decode() if isinstance(x,bytes) else str(x))
    tree=ast.parse((HERE/'qualify.py').read_text())
    functions=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['quiet','poste_fd_allowed','ancestor_identities']]
    ns=dict(Path=FakePath,os=fake_os,json=json,time=types.SimpleNamespace(monotonic=lambda:99),sha=lambda p:'BASH',sys=types.SimpleNamespace(executable='/usr/bin/python3'),OUTER=1198,CFG=CFG,W=FakePath(W),T=FakePath(T),P=FakePath(P),Q=FakePath(Q))
    exec(compile(ast.Module(body=functions,type_ignores=[]),'<actual-quiet>', 'exec'),ns)
    return ns['quiet']([])

class Quiet(unittest.TestCase):
    def test_verified_ancestor_poste_fd_is_not_a_leak(self):
        result=observe('legal')
        self.assertEqual(result['found'],[])
        self.assertFalse(any(x.get('attributed') or x.get('owned') for x in result['unknown']))
    def test_permission_after_attributed_exe_remains_refused(self):
        result=observe('permission')
        self.assertTrue(any(x['path']==T+'/leak' for x in result['found']))
        self.assertTrue(any(x.get('attributed') for x in result['unknown']))
    def test_other_target_fd_is_never_exempted(self):
        self.assertTrue(any(x['path']==T+'/retained-data' for x in observe('foreign-fd')['found']))
    def test_wrong_ancestor_command_cannot_exempt_poste_fd(self):
        self.assertTrue(any(x['path']==Q+'/poste' for x in observe('wrong-command')['found']))

if __name__=='__main__':unittest.main()
