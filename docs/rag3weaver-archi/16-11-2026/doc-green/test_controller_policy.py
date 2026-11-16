"""Exercise the real scanner against a private proc-shaped fixture; no real PID."""
import ast, copy, importlib.util, json, os, sys, tempfile, unittest
import cache_state as empty_cache
from unittest.mock import patch
from pathlib import Path

class ProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        spec = importlib.util.spec_from_file_location('controller_policy', Path(__file__).with_name('controller_policy.py'))
        self.module = importlib.util.module_from_spec(spec); spec.loader.exec_module(self.module)
        previous=sys.modules.get('controller_policy');sys.modules['controller_policy']=self.module
        def restore():
            if previous is None:sys.modules.pop('controller_policy',None)
            else:sys.modules['controller_policy']=previous
        self.addCleanup(restore)
        launch_spec=importlib.util.spec_from_file_location('native_launcher',Path(__file__).with_name('qualify_green.py'))
        launcher=importlib.util.module_from_spec(launch_spec);launch_spec.loader.exec_module(launcher)
        self.assertEqual(launcher.Q,self.module.Q);self.assertEqual(launcher.REL,self.module.REL)
        m = self.module
        m.W=self.root/'source';m.C=m.W/'extension';m.T=self.root/'target';m.OLD_W=self.root/'old';m.Q=self.root/'controller'
        m.T.mkdir();st=m.T.lstat();self.identity=dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(m.T))
        (m.C/'scripts').mkdir(parents=True);(m.C/'scripts/poste').write_text('#!/bin/sh\n')
        self.control=self.root/'control';self.pilot='a'*40
        self.expected=m.policy(self.control,self.pilot);self.commands=m.commands(self.control,self.pilot)
        self.proc=self.root/'proc';self.proc.mkdir()
        self.add(10,1,self.expected['roles']['launch']['argv'],self.expected['roles']['launch']['exe'])
        self.add(15,10,self.commands['outer'],self.expected['roles']['watchdog']['exe'])
        self.add(20,15,self.expected['roles']['poste']['argv'],self.expected['roles']['poste']['exe'])
        (self.proc/'20/fd/255').symlink_to(m.C/'scripts/poste')
        self.expected['launch_identity']=m.snapshot(self.proc/'10')
    def add(self,pid,parent,argv,exe):
        p=self.proc/str(pid);p.mkdir();(p/'fd').mkdir()
        fields=['S',str(parent)]+['0']*17+[str(pid*100)]
        (p/'stat').write_text(f'{pid} (fixture) '+' '.join(fields))
        (p/'cmdline').write_bytes(b'\0'.join(os.fsencode(a) for a in argv)+b'\0')
        (p/'cgroup').write_text('0::/fixture\n')
        (p/'exe').symlink_to(exe);(p/'cwd').symlink_to('/var/tmp')
    def scan(self):
        m=self.module
        return m.scan(self.expected,[m.T,m.W,m.OLD_W,self.control],'/old-scope',self.proc,30,20)
    def test_real_launcher_assignments_and_commands_accept_exact_three_controllers(self):
        result=self.scan();self.assertEqual(result['found'],[])
        self.assertEqual({r['role'] for r in result['controllers']},{'launch','poste','watchdog'})
        self.assertEqual(result['expected']['outer_command'],self.commands['outer'])
    def test_watchdog_argument_difference_is_refused(self):
        wrong=list(self.commands['outer']);wrong[3]='1319s'
        (self.proc/'15/cmdline').write_bytes(b'\0'.join(os.fsencode(v) for v in wrong)+b'\0')
        found=next(r for r in self.scan()['found'] if r['pid']==15)
        self.assertIsNone(found['role']);self.assertFalse(found['comparisons']['watchdog']['argv'])
    def test_watchdog_pid_reuse_is_refused(self):
        m=self.module;row=m.snapshot(self.proc/'15')
        ancestry={pid:m.snapshot(self.proc/str(pid)) for pid in [10,15,20]}
        ancestry[15]['start']='reused'
        role,checks=m.match_controller(row,ancestry,self.expected)
        self.assertIsNone(role);self.assertFalse(checks['watchdog']['ancestor'])
    def test_exact_watchdog_outside_ancestor_chain_is_refused(self):
        self.add(40,10,self.commands['outer'],self.expected['roles']['watchdog']['exe'])
        found=next(r for r in self.scan()['found'] if r['pid']==40)
        self.assertIsNone(found['role']);self.assertFalse(found['comparisons']['watchdog']['ancestor'])
    def test_watchdog_target_fd_is_never_exempted(self):
        (self.proc/'15/fd/3').symlink_to(self.module.T/'active')
        found=next(r for r in self.scan()['found'] if r['pid']==15)
        self.assertEqual(found['role'],'watchdog');self.assertEqual(found['reasons'][0]['field'],'fd')
    def test_controller_target_fd_is_never_exempted(self):
        target=self.module.T/'active';(self.proc/'20/fd/3').symlink_to(target)
        found=self.scan()['found'];self.assertEqual(len(found),1)
        self.assertEqual(found[0]['role'],'poste');self.assertIn(dict(field='fd',fd='3',path=str(target)),found[0]['reasons'])
    def test_launcher_flag_breaks_exact_policy(self):
        (self.proc/'10/cmdline').write_bytes(b'python3\0-B\0'+b'\0'.join(os.fsencode(v) for v in self.commands['launch'])+b'\0')
        found=next(r for r in self.scan()['found'] if r['pid']==10)
        self.assertIsNone(found['role']);self.assertFalse(found['comparisons']['launch']['argv'])
    def test_nonancestor_source_reader_is_captured(self):
        self.add(40,1,['reader'],str(Path('/usr/bin/timeout').resolve()))
        (self.proc/'40/fd/4').symlink_to(self.module.T/'file')
        found=next(r for r in self.scan()['found'] if r['pid']==40)
        self.assertEqual(found['reasons'][0]['field'],'fd');self.assertEqual(found['fds'][0]['fd'],'4')

    def test_scan_archives_partial_attributed_unknown(self):
        self.add(40,1,['reader'],str(Path('/usr/bin/timeout').resolve()))
        (self.proc/'40/fd/4').symlink_to(self.module.T/'active')
        original=self.module.snapshot
        def read(proc):
            if proc.name=='40':raise PermissionError('fixture denied exe')
            return original(proc)
        with patch.object(self.module,'snapshot',side_effect=read):record=self.scan()
        unknown=next(r for r in record['unknown'] if r['pid']==40)
        self.assertTrue(unknown['attributed']);self.assertEqual(unknown['reasons'][0]['field'],'fd')
        self.assertEqual(unknown['readable']['argv'],['reader'])
        with self.assertRaises(AssertionError):self.run_real_admission_tail(record)
    def test_scan_keeps_historical_unknown_without_claiming_it_clear(self):
        self.add(40,1,['reader'],str(Path('/usr/bin/timeout').resolve()))
        original=self.module.snapshot
        def read(proc):
            if proc.name=='40':raise PermissionError('fixture denied')
            return original(proc)
        with patch.object(self.module,'snapshot',side_effect=read):record=self.scan()
        unknown=next(r for r in record['unknown'] if r['pid']==40)
        self.assertIsNone(unknown['attributed'])
        self.run_real_admission_tail(record)
    def test_chain_stops_at_recorded_real_launcher_not_opaque_ssh_parent(self):
        original=self.module.snapshot
        def read(proc):
            if proc.name=='1':raise PermissionError('outside admitted chain')
            return original(proc)
        with patch.object(self.module,'snapshot',side_effect=read):record=self.scan()
        self.assertFalse(record['ancestor_errors']);self.assertEqual(record['ancestors'][-1]['pid'],10)
    def test_reused_recorded_launcher_is_refused(self):
        self.expected['launch_identity']['start']='old-start'
        record=self.scan();self.assertTrue(record['ancestor_errors'])
        with self.assertRaises(AssertionError):self.run_real_admission_tail(record)
    def test_read_only_source_reader_is_allowed_but_writer_is_refused(self):
        self.add(40,1,['reader'],str(Path('/usr/bin/timeout').resolve()))
        (self.proc/'40/fd/4').symlink_to(self.module.W/'file')
        (self.proc/'40/fdinfo').mkdir();info=self.proc/'40/fdinfo/4';info.write_text('flags:\t0100000\n')
        self.assertFalse(self.scan()['found'])
        info.write_text('flags:\t0100001\n')
        found=next(r for r in self.scan()['found'] if r['pid']==40)
        self.assertEqual(found['reasons'][0]['field'],'fd')
    def test_unknown_owned_current_scope_is_refused_even_without_paths(self):
        self.add(40,1,['reader'],str(Path('/usr/bin/timeout').resolve()))
        (self.proc/'40/cgroup').write_text('0::/new-owned\n')
        original=self.module.snapshot
        def read(proc):
            if proc.name=='40':raise PermissionError('fixture')
            return original(proc)
        m=self.module
        with patch.object(m,'snapshot',side_effect=read):
            record=m.scan(self.expected,[m.T,m.W,self.control],'/old-scope',self.proc,30,20,owned_cgroup='/new-owned')
        self.assertTrue(record['unknown'][0]['attributed'])
        with self.assertRaises(AssertionError):self.run_real_admission_tail(record)
    def run_real_admission_tail(self, record):
        tree=ast.parse(Path(__file__).with_name('qualify_green.py').read_text())
        function=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='fresh_cache')
        index=next(i for i,n in enumerate(function.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='expected' for t in n.targets))
        function.body=function.body[index:]
        saved=[]
        receipt=self.root/'receipt.json';receipt.write_text(json.dumps({'cgroup':'/new-owned'}))
        ns=dict(controller=self.module,empty_cache=empty_cache,Path=Path,T=self.module.T,W=self.module.W,OLD_W=self.module.OLD_W,os=os,json=json,
                authority={'cgroup':'/old-scope'},save=lambda name,value:saved.append((name,value)))
        exec(compile(ast.Module(body=[function],type_ignores=[]),'actual-admission-tail','exec'),ns)
        with patch.dict(os.environ,{'RAG3WEAVER_VALIDATION_RECU':str(receipt)}),patch.object(self.module,'policy',return_value=self.expected),patch.object(self.module,'scan',return_value=record):
            try:ns['fresh_cache']({'control':str(self.control),'pilot':self.pilot,'launch_identity':{},'cache_identity':self.identity})
            finally:self.assertEqual(saved,[('cache-after-poste.json',record)])
    def test_initial_empty_admission_keeps_unattributed_unknown_after_saving(self):
        record=self.scan();record['unknown'].append(dict(pid=50,attributed=None,error='PermissionError'))
        self.run_real_admission_tail(record)
    def test_real_admission_rejects_attributed_unknown_after_saving(self):
        record=self.scan();record['unknown'].append(dict(pid=50,attributed=True,error='PermissionError'))
        with self.assertRaises(AssertionError):self.run_real_admission_tail(record)
    def test_real_admission_rejects_incomplete_ancestor_capture_after_saving(self):
        record=self.scan();record['ancestor_errors'].append(dict(pid=10,error='PermissionError'))
        with self.assertRaises(AssertionError):self.run_real_admission_tail(record)
    def test_real_admission_accepts_complete_known_quiet_snapshot(self):
        self.run_real_admission_tail(self.scan())

if __name__=='__main__':unittest.main(failfast=True)
