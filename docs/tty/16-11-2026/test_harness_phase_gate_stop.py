"""Actual harness/controller AST connections, with filesystem and fake proc/pidfd only."""
import ast
import io
import json
from pathlib import Path
import re
import tempfile
import time
import types
import unittest

ROOT = Path(__file__).resolve().parents[3]
MEASURE = ROOT / 'extension/rag3weaver/tty/mesures/mesure.py'
DRIVER = Path(__file__).with_name('harness_only_driver.py')


def scenario():
    tree=ast.parse(MEASURE.read_text())
    start=next(i for i,n in enumerate(tree.body) if isinstance(n,ast.Assign)
               and any(isinstance(t,ast.Name) and t.id=='SECRET' for t in n.targets))
    index=next(i for i in range(start,len(tree.body)) if isinstance(tree.body[i],ast.Try))
    return tree,tree.body[start:index],tree.body[index]


def fixture_connection(root):
    old,new=root/'phase-0007',root/'phase-0008'
    old.mkdir();new.mkdir()
    ns={}
    def ssh(command):
        # Interpret only the legacy fixture write; never launch a shell.
        match=re.search(r'> ([^ ;]+/faux_sudo.sh)',command)
        if match:Path(match.group(1)).write_text('stty -echo; printf "Mot de passe : "; read p; stty echo')
        return ''
    def host_start():
        ns['DOSSIER']=str(new)
        ns['CURRENT_PHASE']=types.SimpleNamespace(record=lambda *a,**k:None,checkpoint=lambda *a:None)
    class StopAtTerminal(Exception):pass
    captured={}
    def terminal(*args,**kwargs):
        captured['path']=ns.get('BANG_SCRIPT',str(Path(ns['DOSSIER'])/'faux_sudo.sh'))
        raise StopAtTerminal()
    ns.update(DOSSIER=str(old),LOCAL=True,Path=Path,ssh=ssh,hote_demarrer=host_start,
              CURRENT_PHASE=None,HOTE='local',TUNNEL=17990,PORT=17990,FAUX='fake.py',
              subprocess=types.SimpleNamespace(Popen=lambda *a,**k:object()),
              time=types.SimpleNamespace(sleep=lambda *a:None),Terminal=terminal,
              hashlib=__import__('hashlib'))
    tree,prefix,block=scenario()
    helpers=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef))
             and n.name in ['prepare_bang_fixture','ScenarioFailed']]
    nodes=helpers+prefix+[block.body[0]]  # the real fixture setup through the actual Terminal constructor
    try:exec(compile(ast.fix_missing_locations(ast.Module(body=nodes,type_ignores=[])),'<actual-bang-setup>','exec'),ns)
    except StopAtTerminal:pass
    path=Path(captured['path'])
    return path.parent==new and path.is_file() and 'stty -echo' in path.read_text()


def missing_prompt_connection():
    tree,_,block=scenario()
    start=next(i for i,n in enumerate(block.body) if isinstance(n,ast.Assign)
               and any(isinstance(t,ast.Name) and t.id=='invite' for t in n.targets))
    stop=next(i for i in range(start,len(block.body)) if isinstance(block.body[i],ast.Assign)
              and isinstance(block.body[i].value,ast.Call) and isinstance(block.body[i].value.func,ast.Attribute)
              and block.body[i].value.func.attr=='ecrire')
    writes=[];reds=[]
    ns=dict(t=types.SimpleNamespace(attendre=lambda *a,**k:None,ecrire=lambda b:writes.append(b)),
            t0=0,SECRET='FAKE_MARKER',time=types.SimpleNamespace(sleep=lambda *a:None),rouge=reds.append)
    helpers=[n for n in tree.body if isinstance(n,ast.ClassDef) and n.name=='ScenarioFailed']
    try:exec(compile(ast.fix_missing_locations(ast.Module(body=helpers+block.body[start:stop+1],type_ignores=[])),'<actual-invite-gate>','exec'),ns)
    except RuntimeError:pass
    return not writes and bool(reds)


class FakeOutput(io.StringIO):
    def fileno(self):return 77


class FakeProcPath:
    def __init__(self,parts=()):self.parts=parts
    def __truediv__(self,part):return FakeProcPath(self.parts+(str(part),))
    def read_text(self):
        if self.parts[-1]=='stat':return '1234 (common) S 1'+' 0'*17+' 100'
        if self.parts[-1]=='cgroup':return '0::/private/common'
        raise AssertionError(self.parts)


def duplicate_red_connection(root):
    q,p=root/'q',root/'p';q.mkdir();p.mkdir()
    receipt=root/'authority.json'
    receipt.write_text(json.dumps(dict(pid=1234,debut='100',etat='actif',script=str(q/'confiner_validation.py'),cgroup='/private/common')))
    opened=[];signals=[]
    fake_os=types.SimpleNamespace(fsync=lambda *a:None,pidfd_open=lambda pid:opened.append(pid) or 99,close=lambda *a:None)
    fake_signal=types.SimpleNamespace(SIGTERM=15,pidfd_send_signal=lambda fd,sig:signals.append((fd,sig)))
    lines=['ROUGE first\n','ROUGE second\n']
    output=FakeOutput()
    ns=dict(p=p,q=q,receipt=receipt,output=output,child=types.SimpleNamespace(stdout=lines),
            json=json,os=fake_os,signal=fake_signal,time=time,stop_requested=False,
            Path=lambda value:FakeProcPath() if str(value)=='/proc' else Path(value))
    tree=ast.parse(DRIVER.read_text())
    withnode=next(n for n in tree.body if isinstance(n,ast.With))
    loop=next(n for n in withnode.body if isinstance(n,ast.For))
    try:exec(compile(ast.fix_missing_locations(ast.Module(body=[loop],type_ignores=[])),'<actual-red-reader>','exec'),ns)
    except FileExistsError:return False
    marker=json.loads((p/'first-red-stop.json').read_text())
    return opened==[1234] and signals==[(99,15)] and marker['cause']=='ROUGE first' and output.getvalue()==''.join(lines)


class HarnessConnectionsTests(unittest.TestCase):
    def test_three_actual_connections(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'fixture').mkdir();(root/'stopper').mkdir()
            failures=[]
            if not fixture_connection(root/'fixture'):failures.append('fixture points to previous phase')
            if not missing_prompt_connection():failures.append('marker sent despite missing prompt')
            if not duplicate_red_connection(root/'stopper'):failures.append('second red rewrites first stop or loses line')
            self.assertEqual(failures,[])


if __name__=='__main__':unittest.main(failfast=True)
