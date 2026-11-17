"""Prepublished effect witnesses, NOT RUN. Never invoke an OS operation.

FakeEffects holds scenario data and call records only. The real runner module
must supply orchestration; this fixture is neither another runner nor CP.
"""
from copy import deepcopy
import unittest

from execution_entry_runner import RunnerRefusal, collect_common_closure, run_fixture_invocation


def request_fixture():
    return dict(mode='FIXTURE_ONLY', invocation='fixture-entry-A', phase='entry',
                declaration_sha256='a' * 64, runtime_plan_sha256='b' * 64,
                artifact_sha256='c' * 64, object_identity=(1, 2),
                argv=('/private/program-A', 'marker'), cwd='/private/proof',
                public_environment={'LANG': 'C', 'LD_LIBRARY_PATH': None},
                clock_domain='MODELED_HOST_1', start=0.0, issued=1.0,
                operation_end=105.0, end=120.0, observation_cutoff=125.0,
                publication_files=('entry-ledger.json', 'entry-object', 'entry-facts.json'))


class FakeEffects:
    """Ports/data only: no fork, file, clock, signal, CP, thread or process."""
    def __init__(self):
        self.calls = []
        self.launch = object()  # pre-fork opaque fixture token, NOT OS ownership
        self.held = object()
        self.seal = object()
        self.permit = object()
        self.clock = 10.0
        self.fail_at = None
        self.error = None
        self.owner_known = False
        self.cleanup_unknown = False
        self.actual = dict(event='EXEC_SUCCESS', object_identity=(1, 2), sha256='c' * 64,
                           argv=('/private/program-A', 'marker'), cwd='/private/proof',
                           environment={'LANG': 'C', 'LD_LIBRARY_PATH': None})
        self.current_path = 'A'
        self.write_controls = dict(before='WRITABLE', held='ETXTBSY', after='WRITABLE')
        self.permit_binding = dict(mode='FIXTURE_ONLY', declaration_sha256='a' * 64,
                                   runtime_plan_sha256='b' * 64, facts_sha256='d' * 64)
        self.canonical_facts = dict(sha256='d' * 64, declared_payload=True)
        self.cp_ack = False
        self.exit = dict(kind='exit', code=0, stdout=b'marker\n', stderr=b'')
        self.root = dict(kind='dbus-run-session', direct_child_of='Common')
        self.later_image = 'UNKNOWN'
        self.proc_ticks = 1000

    def _step(self, name):
        self.calls.append(name)
        if self.fail_at == name:
            raise self.error if self.error is not None else RunnerRefusal(name)

    def now(self):
        return self.clock

    def prepare_owned_launch(self, request):
        self._step('prepare_owned_launch')
        return self.launch

    def probe_write(self, launch, stage):
        assert launch is self.launch
        self._step('probe_write:' + stage)
        return self.write_controls[stage]

    def fork_owned(self, launch):
        assert launch is self.launch
        self._step('fork_owned')
        self.owner_known = True  # registry written before failing pidfd step

    def acquire_identity_and_pidfd(self, launch):
        assert launch is self.launch
        self._step('acquire_identity_and_pidfd')

    def start_and_hold(self, launch):
        assert launch is self.launch
        self._step('start_and_hold')
        return self.held, deepcopy(self.actual)

    def seal_entry(self, launch, held):
        assert launch is self.launch and held is self.held
        self._step('seal_entry')
        return self.seal, deepcopy(self.canonical_facts)

    def edit_fixture_only(self, launch, seal):
        assert launch is self.launch and seal is self.seal
        self._step('edit_fixture_only')
        return self.permit, deepcopy(self.permit_binding)

    def release(self, launch, held, permit):
        assert launch is self.launch and held is self.held and permit is self.permit
        self._step('release')

    def wait_child(self, launch):
        assert launch is self.launch
        self._step('wait_child')
        return deepcopy(self.exit)

    def finish_fixture(self, launch):
        assert launch is self.launch
        self._step('finish_fixture')
        return dict(mode='FIXTURE_ONLY', common_origin='MODELED', root=deepcopy(self.root))

    def cancel_owned(self, launch, first_cause):
        assert launch is self.launch
        self._step('cancel_owned')
        return dict(origin='MODELED', quiet=not self.cleanup_unknown,
                    reaped=not self.cleanup_unknown, child_known=self.owner_known,
                    remaining='UNKNOWN' if self.cleanup_unknown else [])


class FixtureRunnerTests(unittest.TestCase):
    def fixture(self):
        return request_fixture(), FakeEffects()

    def refuse(self, request, effects, code):
        with self.assertRaises(RunnerRefusal) as caught:
            run_fixture_invocation(request, effects)
        self.assertEqual(caught.exception.code, code)
        self.assertNotIn('release', effects.calls)
        return caught.exception

    def test_n01_nominal_seals_before_release_and_wait_fixture_only(self):
        request, effects = self.fixture()
        try:
            result = run_fixture_invocation(request, effects)
        except RunnerRefusal as error:
            if error.code != 'backend-not-implemented':
                raise
            self.fail('fixture entry runner backend is not implemented')
        self.assertTrue(result['entry_confirmed'])
        self.assertEqual(result['returncode'], 0)
        self.assertEqual(result['mode'], 'FIXTURE_ONLY')
        self.assertFalse(result['cp_ack'])
        self.assertFalse(result['global_admission'])
        self.assertEqual(result['later_images'], 'UNKNOWN')
        self.assertEqual(result['sdk'], 'NOT_ATTESTED')
        self.assertEqual(result['proof_origin'], 'MODELED')
        self.assertLess(effects.calls.index('prepare_owned_launch'), effects.calls.index('fork_owned'))
        self.assertLess(effects.calls.index('seal_entry'), effects.calls.index('edit_fixture_only'))
        self.assertLess(effects.calls.index('edit_fixture_only'), effects.calls.index('release'))
        self.assertLess(effects.calls.index('release'), effects.calls.index('wait_child'))

    def test_n02_eof_never_confirms_exec(self):
        request, effects = self.fixture()
        effects.actual['event'] = 'CLOEXEC_EOF'
        self.refuse(request, effects, 'entry-event')

    def test_n03_death_before_entry_does_not_require_future_receipt(self):
        request, effects = self.fixture()
        effects.fail_at = 'start_and_hold'
        effects.error = RunnerRefusal('child-died-before-entry')
        self.refuse(request, effects, 'child-died-before-entry')
        self.assertNotIn('edit_fixture_only', effects.calls)
        self.assertIn('cancel_owned', effects.calls)

    def test_n04_original_fd_a_survives_path_replacement_with_b(self):
        request, effects = self.fixture()
        effects.current_path = 'B'  # no change to the actual executable FD A
        result = run_fixture_invocation(request, effects)
        self.assertTrue(result['entry_confirmed'])
        self.assertEqual(result['artifact_sha256'], request['artifact_sha256'])

    def test_n05_same_inode_changed_content_refuses(self):
        request, effects = self.fixture()
        effects.actual['sha256'] = 'e' * 64
        self.refuse(request, effects, 'entry-content')

    def test_n06_write_denial_needs_writable_controls_and_etxtbsy(self):
        request, effects = self.fixture()
        effects.write_controls['held'] = 'EACCES'
        self.refuse(request, effects, 'storage-profile-unqualified')

    def test_n07_partial_or_failed_seal_never_releases(self):
        request, effects = self.fixture()
        effects.fail_at = 'seal_entry'
        effects.error = RunnerRefusal('copy-incomplete')
        self.refuse(request, effects, 'copy-incomplete')
        self.assertNotIn('edit_fixture_only', effects.calls)

    def test_n08_wrong_permit_facts_refuses_before_release(self):
        request, effects = self.fixture()
        effects.permit_binding['facts_sha256'] = 'e' * 64
        self.refuse(request, effects, 'publication-binding')

    def test_n09_fixture_port_cannot_provide_live_grant(self):
        request, effects = self.fixture()
        request['mode'] = 'LIVE'
        self.refuse(request, effects, 'unsupported-live')
        self.assertEqual(effects.calls, [])

    def test_n10_parent_loss_cannot_be_made_quiet_by_cleanup_claim(self):
        request, effects = self.fixture()
        effects.fail_at = 'start_and_hold'
        effects.error = RunnerRefusal('parent-lost')
        effects.cleanup_unknown = True
        error = self.refuse(request, effects, 'parent-lost')
        self.assertEqual(error.cleanup['remaining'], 'UNKNOWN')

    def test_n11_nonzero_child_status_belongs_to_invocation(self):
        request, effects = self.fixture()
        effects.exit.update(kind='exit', code=7)
        result = run_fixture_invocation(request, effects)
        self.assertEqual(result['returncode'], 7)
        self.assertFalse(result['global_admission'])

    def test_n12_dbus_root_is_not_falsely_waited_as_runner(self):
        request, effects = self.fixture()
        result = run_fixture_invocation(request, effects)
        self.assertEqual(result['common_origin'], 'MODELED')
        self.assertNotIn('wait_nonchild', effects.calls)
        self.assertLess(effects.calls.index('wait_child'), effects.calls.index('finish_fixture'))

    def test_n13_unknown_common_final_is_unsupported_not_fixture_fallback(self):
        with self.assertRaises(RunnerRefusal) as caught:
            collect_common_closure(None, None)
        self.assertEqual(caught.exception.code, 'unsupported-common-closure')

    def test_n14_late_ack_and_unknown_residue_never_leave_green(self):
        request, effects = self.fixture()
        original = effects.edit_fixture_only
        def delayed(launch, seal):
            result = original(launch, seal)
            effects.clock = 126.0  # logical cutoff crossed; no physical kill inferred
            effects.cleanup_unknown = True
            return result
        effects.edit_fixture_only = delayed
        error = self.refuse(request, effects, 'operation-deadline')
        self.assertEqual(error.cleanup['remaining'], 'UNKNOWN')

    def test_n15_undeclared_phase_environment_refuses(self):
        request, effects = self.fixture()
        effects.actual['environment'] = {'LANG': 'C', 'LD_LIBRARY_PATH': '/foreign'}
        self.refuse(request, effects, 'entry-context')

    def test_n16_future_images_remain_unknown_after_release(self):
        request, effects = self.fixture()
        effects.later_image = 'B'
        result = run_fixture_invocation(request, effects)
        self.assertEqual(result['later_images'], 'UNKNOWN')
        self.assertEqual(result['result_scope'], 'reviewed-program-invocation')

    def test_n17_proc_ticks_do_not_extend_monotonic_origin(self):
        request, effects = self.fixture()
        effects.proc_ticks += 999999999
        result = run_fixture_invocation(request, effects)
        self.assertEqual(result['original_end'], request['end'])

    def test_n18_facts_must_be_declared_payload_before_release(self):
        request, effects = self.fixture()
        effects.canonical_facts['declared_payload'] = False
        self.refuse(request, effects, 'publication-payload')

    def test_n19_real_entry_b_refuses_without_promoting_expected_a(self):
        request, effects = self.fixture()
        effects.actual.update(object_identity=(1, 99), sha256='e' * 64)
        self.refuse(request, effects, 'entry-object')
        self.assertEqual(request['artifact_sha256'], 'c' * 64)

    def test_n20_failure_after_fork_preserves_preheld_ownership(self):
        request, effects = self.fixture()
        effects.fail_at = 'acquire_identity_and_pidfd'
        effects.error = PermissionError('MODELED pidfd refused')
        error = self.refuse(request, effects, 'startup-failed')
        self.assertIs(error.first_cause, effects.error)
        self.assertTrue(error.cleanup['child_known'])
        self.assertTrue(error.cleanup['reaped'])
        self.assertIn('cancel_owned', effects.calls)

    def test_n21_cleanup_failure_never_overwrites_first_cause(self):
        request, effects = self.fixture()
        effects.fail_at = 'start_and_hold'
        effects.error = RunnerRefusal('startup-parent-race')
        def failed_cleanup(launch, first_cause):
            effects.calls.append('cancel_owned')
            raise PermissionError('MODELED cleanup refused')
        effects.cancel_owned = failed_cleanup
        error = self.refuse(request, effects, 'startup-parent-race')
        self.assertEqual(error.cleanup['remaining'], 'UNKNOWN')

    def test_sigtrap_without_exec_event_is_not_confirmation(self):
        request, effects = self.fixture()
        effects.actual['event'] = 'SIGTRAP'
        self.refuse(request, effects, 'entry-event')

    def test_signaled_child_is_not_debugger_rc_zero(self):
        request, effects = self.fixture()
        effects.exit.update(kind='signal', code=15)
        result = run_fixture_invocation(request, effects)
        self.assertEqual(result['returncode'], -15)

    def test_nonwritable_before_control_cannot_qualify_exec_write_denial(self):
        request, effects = self.fixture()
        effects.write_controls['before'] = 'EROFS'
        self.refuse(request, effects, 'storage-profile-unqualified')
        self.assertNotIn('fork_owned', effects.calls)
