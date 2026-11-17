"""Témoins correctifs préparés avant fix ; CPU futur, aucun vrai proc/verrou."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import tool_qualification as tool
import tool_phase


class PosteAuthorityFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.base=Path(self.tmp.name);self.root=self.base/'locks';self.root.mkdir()
        self.proc=self.base/'proc';(self.proc/'4242/fd').mkdir(parents=True)
        (self.proc/'4242/fdinfo').mkdir();(self.root/'fiches').mkdir()
        (self.root/'fiches/4242').write_text('tient lourd depuis fixture\n')
        self.lock=self.root/'poste.lock';self.lock.write_bytes(b'')
        (self.proc/'4242/fd/9').symlink_to(self.lock)
        st=self.lock.stat();self.lockline=f'lock:\t1: FLOCK  ADVISORY  READ 0 {os.major(st.st_dev):02x}:{os.minor(st.st_dev):02x}:{st.st_ino} 0 EOF\n'
        (self.proc/'4242/fdinfo/9').write_text(self.lockline)
        (self.proc/'4242/cmdline').write_bytes(b'/bin/bash\0/private/poste\0lourd\0')
        self.table={4242:dict(parent=1,session=4242,debut='123',cgroup='/fixture'),
                    os.getpid():dict(parent=4242,session=4242,debut='456',cgroup='/fixture')}
        path=Path(__file__).with_name('common-confiner.py')
        spec=importlib.util.spec_from_file_location('fixture_common',path)
        self.common=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.common)
        def common_path(value):
            p=Path(value)
            return self.proc/p.relative_to('/proc') if p.is_relative_to('/proc') else p
        self.addCleanup(patch.stopall)
        patch.object(self.common,'Path',side_effect=common_path).start()
        patch.object(self.common,'processus',return_value=self.table).start()
        patch.object(tool,'PROC',self.proc).start()
        patch.dict(os.environ,{'HOME':str(self.base/'private-home'),'POSTE_LOCK_DIR':str(self.root)},clear=True).start()
    def proof(self):return tool.attest_poste(self.common,self.table,str(self.root))
    def test_real_environment_and_common_find_actual_ancestor_with_private_home(self):
        proof=self.proof();env=tool.controller_environment(self.base,proof)
        self.assertEqual(env['HOME'],str(self.base/'home'))
        self.assertEqual(env['POSTE_LOCK_DIR'],str(self.root))
        with patch.dict(os.environ,env,clear=True):
            self.assertEqual(self.common.poste_tenu(self.table),4242)
            self.assertEqual(self.proof(),proof)
    def test_arbitrary_lock_root_is_refused(self):
        with patch.dict(os.environ,{'POSTE_LOCK_DIR':str(self.base)}):
            with self.assertRaises(ValueError):self.proof()
    def test_fiche_without_kernel_flock_is_refused(self):
        (self.proc/'4242/fdinfo/9').write_text('pos: 0\n')
        with self.assertRaises(ValueError):self.proof()
    def test_fiche_without_matching_fd_is_refused(self):
        (self.proc/'4242/fd/9').unlink()
        with self.assertRaises((OSError,ValueError)):self.proof()
    def test_changed_pid_start_is_refused(self):
        changed={**self.table,4242:{**self.table[4242],'debut':'124'}}
        with patch.object(self.common,'processus',return_value=changed):
            with self.assertRaises(ValueError):self.proof()
    def test_native_environment_contains_no_common_or_lock_authority(self):
        with patch.dict(os.environ,{'RAG3WEAVER_VALIDATION_JETON':'PRIVATE-FAKE'}):
            env=tool_phase.native_environment(self.base)
        self.assertNotIn('POSTE_LOCK_DIR',env)
        self.assertNotIn('RAG3WEAVER_VALIDATION_JETON',env)
        self.assertNotIn('RAG3WEAVER_VALIDATION_RECU',env)


if __name__=='__main__':unittest.main(verbosity=2)
