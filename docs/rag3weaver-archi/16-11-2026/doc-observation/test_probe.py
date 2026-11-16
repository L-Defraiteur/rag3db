"""Exercise the real scanner against a private proc-shaped fixture; no real PID."""
import importlib.util, json, os, tempfile, unittest
from pathlib import Path

class ProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        spec = importlib.util.spec_from_file_location('observation_probe', Path(__file__).with_name('probe.py'))
        self.module = importlib.util.module_from_spec(spec); spec.loader.exec_module(self.module)
        m = self.module
        m.W=self.root/'source';m.C=m.W/'extension';m.T=self.root/'target';m.OLD_W=self.root/'old';m.Q=self.root/'controller'
        (m.C/'scripts').mkdir(parents=True);(m.C/'scripts/poste').write_text('#!/bin/sh\n')
        self.control=self.root/'control';self.pilot='a'*40
        self.expected=m.policy(self.control,self.pilot);self.commands=m.commands(self.control,self.pilot)
        self.proc=self.root/'proc';self.proc.mkdir()
        self.add(10,1,['python3',*self.commands['launch']],self.expected['roles']['launch']['exe'])
        self.add(20,10,['/bin/sh',*self.commands['poste']],self.expected['roles']['poste']['exe'])
        (self.proc/'20/fd/255').symlink_to(m.C/'scripts/poste')
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
    def test_exact_real_command_assignments_accept_only_two_controllers(self):
        result=self.scan();self.assertEqual(result['found'],[])
        self.assertEqual({r['role'] for r in result['controllers']},{'launch','poste'})
        self.assertEqual(result['expected']['outer_command'],self.commands['outer'])
    def test_watchdog_is_observed_and_not_exempted(self):
        m=self.module
        self.add(15,10,self.commands['outer'],str(Path('/usr/bin/timeout').resolve()))
        stat=self.proc/'20/stat';stat.write_text(stat.read_text().replace('(fixture) S 10 ','(fixture) S 15 '))
        result=self.scan();found=next(r for r in result['found'] if r['pid']==15)
        self.assertIsNone(found['role']);self.assertTrue(any(r['field']=='argv' for r in found['reasons']))
        self.assertEqual(found['argv'],self.commands['outer']);self.assertEqual(found['parent']['pid'],10)
        self.assertEqual(found['start'],'1500');self.assertIn('exe_sha256',found)
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
        (self.proc/'40/fd/4').symlink_to(self.module.OLD_W/'file')
        found=next(r for r in self.scan()['found'] if r['pid']==40)
        self.assertEqual(found['reasons'][0]['field'],'fd');self.assertEqual(found['fds'][0]['fd'],'4')

if __name__=='__main__':unittest.main(failfast=True)
