"""Written, NOT RUN: pure validation of the actual reader admission function."""
import copy,unittest
from reader_runtime import validate_capture

class ReaderRuntime(unittest.TestCase):
    def fixture(self):
        cfg=dict(argv=['/private/typed','/private/copy/db','MATCH query'],sha256='a'*64,expected=dict(HOME='/fixture',LD_LIBRARY_PATH='/sdk59'),cgroup='0::/owned\n',library='/sdk59/librag3db.so',deadline=100.)
        row=dict(pid=123,start='42',inferior=1,exe=cfg['argv'][0],exe_sha256=cfg['sha256'],cgroup=cfg['cgroup'],argv=cfg['argv'],rawlen=100,expected=cfg['expected'],actual=cfg['expected'],diff={},present={key:True for key in cfg['expected']},maps='1 2 3 /sdk59/librag3db.so',native=['1 2 3 /sdk59/librag3db.so'],observed=10.)
        row['after']=dict(start=row['start'],exe=row['exe'],cgroup=row['cgroup'],exe_sha256=row['exe_sha256'])
        first=copy.deepcopy(row);main=copy.deepcopy(row);main['observed']=20.
        exited=dict(pid=123,start='42',inferior=1,exit_code=0,observed=30.)
        return cfg,first,main,exited
    def test_same_real_inferior_sdk_and_exit_are_accepted(self):validate_capture(*self.fixture())
    def test_pid_reuse_or_other_inferior_refused(self):
        for field,value in [('pid',124),('start','43'),('inferior',2)]:
            with self.subTest(field=field):
                cfg,first,main,exited=self.fixture();main[field]=value
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_real_exit_absent_nonzero_or_bool_is_not_gdb_success(self):
        for value in [None,1,True]:
            with self.subTest(code=value):
                cfg,first,main,exited=self.fixture();exited['exit_code']=value
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_sdk_absent_or_other_path_refused(self):
        for maps in ['', '1 2 3 /other/librag3db.so']:
            with self.subTest(maps=maps):
                cfg,first,main,exited=self.fixture();main['maps']=maps;main['native']=maps.splitlines()
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_missing_empty_or_changed_actual_profile_refused(self):
        for change in ['absent','raw-empty','different']:
            with self.subTest(change=change):
                cfg,first,main,exited=self.fixture()
                if change=='absent':main['present']['HOME']=False
                elif change=='raw-empty':main['rawlen']=0
                else:main['actual']['HOME']='/real-home'
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_after_exec_or_cgroup_change_refused(self):
        for field,value in [('start','43'),('exe','/other'),('cgroup','0::/foreign\n'),('exe_sha256','b'*64)]:
            with self.subTest(field=field):
                cfg,first,main,exited=self.fixture();main['after'][field]=value
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_expired_exit_or_reordered_observations_refused(self):
        for value in [5.,100.,101.]:
            with self.subTest(observed=value):
                cfg,first,main,exited=self.fixture();exited['observed']=value
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)
    def test_exit_bound_to_same_pid_start(self):
        for field,value in [('pid',124),('start','43'),('inferior',2)]:
            with self.subTest(field=field):
                cfg,first,main,exited=self.fixture();exited[field]=value
                with self.assertRaises(ValueError):validate_capture(cfg,first,main,exited)

if __name__=='__main__':unittest.main(failfast=True)
