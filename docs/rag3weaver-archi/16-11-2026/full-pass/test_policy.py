"""Prepublished hermetic witnesses; temporary fixture files only, no processes."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import patch
import policy


class Contracts(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='cx-pv7-policy-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()

    def snapshot(self, expected, additions=None):
        actual = dict(expected)
        actual.update(additions or {})
        raw = b''.join(str(k).encode()+b'='+str(v).encode()+b'\0' for k, v in actual.items() if v is not None)
        return policy.selected_profile(raw, expected)

    def test_fresh_authority_does_not_accept_old_window_or_reset(self):
        window = policy.Window.fresh(dict(debut='12345678', echeance=7300., etat='actif'), dict(delai=7200), 200.)
        self.assertEqual(window.phase_end(7200., 900), 7285.)
        for receipt in [dict(debut='12345678', echeance=100., etat='actif'),
                        dict(debut='12345678', echeance=7300., etat='termine')]:
            with self.assertRaises(ValueError):
                policy.Window.fresh(receipt, dict(delai=7200), 200.)
        with self.assertRaises(ValueError):
            window.phase_end(7290., 900)

    def test_jobs_respect_actual_poste_allocation(self):
        self.assertEqual([policy.jobs(n) for n in [2, 5, 16]], [2, 5, 8])
        with self.assertRaises(ValueError):
            policy.jobs(0)

    def test_native_profile_requires_seven_present_and_no_old_engine(self):
        profile = policy.sdk_profile(self.root/'store', self.root/'app', 'declared-LD')
        snapshot = self.snapshot(profile)
        policy.qualify_profile(snapshot, profile)
        self.assertTrue(all(snapshot['present'].values()))
        with self.assertRaises(ValueError):
            policy.qualify_profile(self.snapshot(profile, {'RAG3WEAVER_MOTEUR_ANCIEN': '1'}), profile)

    def test_cpu_features_do_not_erase_inherited_sdk_profile(self):
        inherited = policy.sdk_profile(self.root/'store', self.root/'app', 'declared-LD')
        admission = policy.CargoAdmission(self.root/'toy', self.root/'tmp-target',
                                         ('test', '--lib', '--', 'un_test'), (), inherited, 'unit-toy')
        admission.validate(admission.cwd, admission.target, admission.argv, self.snapshot(inherited))
        with self.assertRaises(ValueError):
            admission.validate(admission.cwd, admission.target, admission.argv,
                               self.snapshot(policy.cpu_profile(self.root/'toy', 'rust-lib')))

    def test_missing_empty_duplicate_and_wrong_selected_fields_refuse(self):
        expected = policy.sdk_profile(self.root/'store', self.root/'app', 'declared-LD')
        for changed in [{'RAG3DB_SHARED': None}, {'RAG3DB_SHARED': ''}, {'LD_LIBRARY_PATH': 'wrong'}]:
            with self.assertRaises(ValueError):
                policy.qualify_profile(self.snapshot(expected, changed), expected)
        for raw in [b'', b'X=1', b'RAG3DB_LIB=a\0RAG3DB_LIB=b\0']:
            with self.assertRaises(ValueError):
                policy.selected_profile(raw, expected)

    def test_snapshot_never_publishes_unknown_secret(self):
        expected = policy.cpu_profile(self.root/'app', 'rust-lib')
        row = self.snapshot(expected, {'SECRET_TOKEN': 'do-not-publish'})
        self.assertNotIn('do-not-publish', json.dumps(row))
        policy.qualify_profile(row, expected)

    def test_cargo_command_target_and_source_are_exact(self):
        profile = policy.cpu_profile(self.root/'task', 'rust-lib')
        admission = policy.CargoAdmission(self.root/'task', self.root/'task-targets/taches-x',
                                         tuple(policy.d6_argv()), ('rag3db-native', 'code', 'daemon'), profile, 'D6')
        for cwd, target, argv in [(self.root/'other', admission.target, admission.argv),
                                  (admission.cwd, self.root/'archive', admission.argv),
                                  (admission.cwd, admission.target, ('test', '--all'))]:
            with self.assertRaises(ValueError):
                admission.validate(cwd, target, argv, self.snapshot(profile))

    def test_repo_target_matches_ascii_sanitization(self):
        self.assertEqual(policy.task_target(self.root/'task-targets', 'dépot x'),
                         self.root/'task-targets/taches-d_pot_x')

    def test_fixture_preserves_approval_and_adds_only_private_reads(self):
        manifest = dict(workspace=dict(commands='approval', sandbox=dict(extra_read=['~/.rustup'], env={})))
        result = policy.fixture_sandbox(manifest, self.root/'Q', self.root/'store', self.root/'task-targets', 5)
        self.assertEqual(manifest['workspace']['sandbox']['extra_read'], ['~/.rustup'])
        self.assertEqual(result['workspace']['commands'], 'approval')
        self.assertEqual(result['workspace']['sandbox']['env']['CARGO_TARGET_DIR'], '{repo_target}')
        self.assertEqual(result['workspace']['sandbox']['env']['CARGO_BUILD_JOBS'], '5')
        self.assertEqual(result['workspace']['sandbox']['extra_read'], ['~/.rustup', str(self.root/'Q'), str(self.root/'store')])

    def test_preexisting_task_target_or_link_refuses_before_exclusion(self):
        root = self.root/'cache'
        root.mkdir()
        policy.task_root_precondition(root)
        target = root/'task-targets'
        target.mkdir()
        with self.assertRaises(ValueError):
            policy.task_root_precondition(root)
        target.rmdir()
        target.symlink_to(self.root/'missing')
        with self.assertRaises(ValueError):
            policy.task_root_precondition(root)

    def test_frozen_inventory_detects_old_change_and_never_excludes_outside(self):
        root = self.root/'cache'
        root.mkdir()
        old = root/'old'
        old.write_text('frozen')
        excluded = [root/'native-doc-ea670-v1', root/'task-targets']
        before = policy.frozen_inventory(root, excluded)
        excluded[1].mkdir()
        (excluded[1]/'new').write_text('active')
        self.assertEqual(policy.frozen_inventory(root, excluded), before)
        old.write_text('changed')
        self.assertNotEqual(policy.frozen_inventory(root, excluded), before)
        with self.assertRaises(ValueError):
            policy.frozen_inventory(root, [self.root/'other'])

    def test_final_open_cannot_publish_a_late_green(self):
        target = self.root/'result.json'
        clock = [99.]
        real_open = Path.open
        def delayed_open(path, *args, **kwargs):
            stream = real_open(path, *args, **kwargs)
            if path == target:
                clock[0] = 101.
            return stream
        with patch.object(Path, 'open', delayed_open):
            result = policy.publish_result(target, dict(verdict='VERT', deadline=100.), lambda: clock[0])
        self.assertEqual(result['verdict'], 'ROUGE')
        self.assertEqual(json.loads(target.read_text()), result)
        with self.assertRaises(FileExistsError):
            policy.publish_result(target, dict(verdict='VERT', deadline=200.), lambda: 102.)


if __name__ == '__main__':
    unittest.main(failfast=True)
