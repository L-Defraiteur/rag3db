"""Témoins hermétiques du profil : aucun Cargo, SDK, bus ou modèle ouvert."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from validation_profile import PRIVATE_PATHS, check_private_environment, private_environment


class PrivateProfileTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='rate-key-profile-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.proof = self.root/'proof'
        self.personal = self.root/'personal'
        self.proof.mkdir(); self.personal.mkdir()
        self.inherited = dict(os.environ)
        self.inherited.update({key: str(self.personal/key.lower()) for key in PRIVATE_PATHS})
        self.inherited.update(HOME=str(self.personal),
            DBUS_SESSION_BUS_ADDRESS='unix:path=/current-private-scope/bus',
            XDG_RUNTIME_DIR='/current-private-scope/runtime',
            RAG3DB_LIBS_DIR='/external-sdk-store',
            RAG3WEAVER_GRANITE_107M_BPK='/external-model-store/granite107.bpk',
            SECRET_FOR_PROFILE_TEST='must-not-be-recorded')

    def test_child_writes_stay_in_private_roots(self):
        env = private_environment(self.proof, self.inherited)
        child = """import os,pathlib,sys
for key in sys.argv[1:]:
 path=pathlib.Path(os.environ[key]);path.mkdir(parents=True,exist_ok=True)
 (path/(key+'.probe')).write_text('owned fixture')
"""
        completed = subprocess.run([sys.executable, '-c', child, *PRIVATE_PATHS], env=env,
                                   capture_output=True, text=True, timeout=10)
        self.assertEqual(completed.returncode, 0, (completed.stdout, completed.stderr))
        self.assertEqual(list(self.personal.iterdir()), [], 'private validation wrote into personal roots')
        self.assertEqual(len(list(self.proof.rglob('*.probe'))), len(PRIVATE_PATHS))

    def test_state_roots_are_distinct_nonoverlapping_siblings(self):
        env = private_environment(self.proof, self.inherited)
        state = Path(env['XDG_STATE_HOME']); fixture = Path(env['RAG3WEAVER_STATE_DIR'])
        self.assertEqual(state, self.proof/'xdg-state')
        self.assertEqual(fixture, self.proof/'state-fixture')
        self.assertEqual(state.parent, fixture.parent)
        self.assertFalse(state.is_relative_to(fixture) or fixture.is_relative_to(state))

    def test_current_scope_and_external_artifacts_stay_inherited(self):
        before = dict(self.inherited)
        env = private_environment(self.proof, self.inherited)
        for key in ('HOME','DBUS_SESSION_BUS_ADDRESS','XDG_RUNTIME_DIR','RAG3DB_LIBS_DIR','RAG3WEAVER_GRANITE_107M_BPK'):
            self.assertEqual(env[key], before[key], key)
        self.assertEqual(self.inherited, before, 'profile construction mutated inherited environment')

    def test_final_check_rejects_an_escaped_root(self):
        env = private_environment(self.proof, self.inherited)
        env['XDG_CACHE_HOME'] = str(self.personal/'escaped-cache')
        with self.assertRaises(ValueError):
            check_private_environment(self.proof, env)

    def test_public_profile_does_not_record_secrets(self):
        env = private_environment(self.proof, self.inherited)
        snapshot = check_private_environment(self.proof, env)
        serialized = json.dumps(snapshot)
        self.assertNotIn('SECRET_FOR_PROFILE_TEST', serialized)
        self.assertNotIn('must-not-be-recorded', serialized)


if __name__ == '__main__':
    unittest.main(verbosity=2)
