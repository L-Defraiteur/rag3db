"""Actual Git worktree proof fixtures; no Cargo, services or real source checkout."""
from pathlib import Path
import subprocess, tempfile, unittest
from unittest.mock import patch
import source_proof

class SourceProof(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.live=self.root/'live';self.live.mkdir();self.archive=self.root/'archive'
        self.git(self.live,'init','-q');(self.live/'source').write_bytes(b'old')
        self.commit(self.live,'original')
        self.head=source_proof.git(self.live,'rev-parse','HEAD')
        self.git(self.live,'worktree','add','--detach',str(self.archive),self.head)
        self.patcher=patch.object(source_proof,'OLD_APP',self.head);self.patcher.start();self.addCleanup(self.patcher.stop)
    def git(self,root,*args):
        return subprocess.run(['git','-C',str(root),*args],check=True,capture_output=True,text=True)
    def commit(self,root,message):
        self.git(root,'add','-A');self.git(root,'-c','user.name=Lucie Defraiteur','-c','user.email=luciedefraiteur@gmail.com','commit','-q','-m',message)
    def test_detached_archive_has_independent_admin_and_exact_tracked_bytes(self):
        record=source_proof.physical_archive(self.archive)
        self.assertNotEqual(record['gitdir'],source_proof.git(self.live,'rev-parse','--absolute-git-dir'))
        self.assertEqual(record['tracked'],source_proof.tracked(self.live));source_proof.verify_archive(record)
    def test_archive_stays_frozen_when_live_head_moves(self):
        record=source_proof.physical_archive(self.archive)
        (self.live/'source').write_bytes(b'new');self.commit(self.live,'changed')
        self.assertNotEqual(source_proof.git(self.live,'rev-parse','HEAD'),self.head)
        source_proof.verify_archive(record);self.assertEqual((self.archive/'source').read_bytes(),b'old')
    def test_gitfile_pointing_to_mobile_head_is_rejected(self):
        (self.archive/'.git').write_text('gitdir: '+source_proof.git(self.live,'rev-parse','--absolute-git-dir')+'\n')
        with self.assertRaises(AssertionError):source_proof.physical_archive(self.archive)

    def test_secret_path_is_rejected_before_any_content_read(self):
        with patch.object(source_proof.subprocess,'check_output',return_value=b'100644 fake 0\t.env\0'):
            with self.assertRaises(AssertionError):source_proof.tracked(self.live)

if __name__=='__main__':unittest.main(failfast=True)
