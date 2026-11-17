"""Prepublished model witnesses. NOT RUN; future pure implementation absent.

No OS backend, process, file, real clock or signals. Successful model validation
will never confer physical admission. Import path to be pinned in a distinct GO.
"""
from copy import deepcopy
import unittest

from execution_entry import Refusal, verify_transcript


def fixture():
    plan = dict(invocation='inv-1', source='source-A', phase='lib',
                allocation='allocation-1', nonce='nonce-1', manifest='pins-A',
                catalogue='catalogue-A',
                artifact='a' * 64, features=('navigateur',),
                argv=('/declared/A', '--exact', 'test'), cwd='/reviewed',
                environment={'LD_LIBRARY_PATH': '/private/sdk'},
                stdout='raw-stdout', stderr='raw-stderr', authority='common-1',
                runner_pid=100, runner_start=11, uid=1000, cgroup='scope-1',
                clock='host-1', end=100.0, operation_end=90.0,
                cleanup_reserve=10.0,
                storage_profile='local-regular-exec-write-denial-v1')
    identity = dict(pid=101, start=12, uid=1000, cgroup='scope-1')
    obj = dict(dev=1, inode=2, sha256='a' * 64)
    chosen = {key: deepcopy(plan[key]) for key in
              ('artifact', 'source', 'phase', 'allocation', 'nonce', 'manifest',
               'catalogue', 'features', 'argv', 'cwd',
               'environment', 'stdout', 'stderr', 'authority', 'storage_profile')}
    chosen['object'] = deepcopy(obj)
    records = [
        ('selected', chosen),
        ('spawned', dict(identity, ppid=100, runner_pid=100, runner_start=11)),
        ('exec_stop', dict(identity, event='EXEC_SUCCESS', object=deepcopy(obj),
                           stable=True, argv=plan['argv'], cwd=plan['cwd'],
                           environment=deepcopy(plan['environment']))),
        ('copied', dict(identity, source_object=deepcopy(obj), complete=True,
                        sha256='a' * 64, rehash='a' * 64)),
        ('published', dict(invocation='inv-1', catalogue='catalogue-A',
                           copied_sha256='a' * 64, ack=True)),
        ('released', dict(identity, detach_success=True)),
        ('waited', dict(identity, parent_pid=100, kind='exit', code=0)),
        ('finalized', dict(stdout='raw-stdout', stderr='raw-stderr',
                           stdout_sha256='b' * 64, stderr_sha256='c' * 64,
                           quiet=True, reaped=True, published=True)),
    ]
    facts = [dict(sequence=i, invocation='inv-1', clock='host-1',
                  at=float(i + 1), kind=kind, data=data)
             for i, (kind, data) in enumerate(records)]
    return plan, facts


class EntryContract(unittest.TestCase):
    def refused(self, plan, facts, code):
        with self.assertRaises(Refusal) as caught:
            verify_transcript(plan, facts)
        self.assertEqual(caught.exception.code, code)

    def test_confirmed_entry_result_belongs_to_invocation(self):
        plan, facts = fixture()
        result = verify_transcript(plan, facts)
        self.assertTrue(result['entry_confirmed'])
        self.assertTrue(result['invocation_complete'])
        self.assertEqual(result['returncode'], 0)
        self.assertEqual(result['result_scope'], 'reviewed-program-invocation')
        self.assertEqual(result['later_images'], 'UNKNOWN')
        self.assertEqual(result['sdk'], 'NOT_ATTESTED')

    def test_path_substitution_cannot_promote_foreign_object(self):
        plan, facts = fixture()
        facts[2]['data']['object']['inode'] = 999
        self.refused(plan, facts, 'entry-object')

    def test_same_inode_changed_bytes_refused(self):
        plan, facts = fixture()
        facts[2]['data']['object']['sha256'] = 'd' * 64
        self.refused(plan, facts, 'entry-content')

    def test_fd_and_two_hashes_without_stability_not_enough(self):
        plan, facts = fixture()
        facts[2]['data']['stable'] = False
        self.refused(plan, facts, 'entry-stability')

    def test_eof_is_not_exec_confirmation(self):
        plan, facts = fixture()
        facts[2]['data']['event'] = 'CLOEXEC_EOF'
        self.refused(plan, facts, 'entry-event')

    def test_stop_is_not_successful_exec(self):
        plan, facts = fixture()
        facts[2]['data']['event'] = 'SIGSTOP'
        self.refused(plan, facts, 'entry-event')

    def test_death_before_confirmation_cannot_be_completed_by_cleanup(self):
        plan, facts = fixture()
        facts[2]['kind'] = 'failure'
        facts[2]['data'] = dict(code='child-died-before-entry')
        self.refused(plan, facts, 'child-died-before-entry')

    def test_runner_pid_not_child_pid(self):
        plan, facts = fixture()
        facts[1]['data']['pid'] = plan['runner_pid']
        self.refused(plan, facts, 'child-identity')

    def test_pid_reuse_before_entry_refused(self):
        plan, facts = fixture()
        facts[2]['data']['start'] += 1
        self.refused(plan, facts, 'child-identity')

    def test_copy_must_be_of_actual_entry_object(self):
        plan, facts = fixture()
        facts[3]['data']['source_object']['inode'] = 999
        self.refused(plan, facts, 'copy-object')

    def test_partial_or_mutated_copy_cannot_release(self):
        plan, facts = fixture()
        facts[3]['data']['complete'] = False
        self.refused(plan, facts, 'copy-content')

    def test_ack_does_not_authorize_new_catalogue(self):
        plan, facts = fixture()
        facts[4]['data']['catalogue'] = 'actual-B'
        self.refused(plan, facts, 'publication-binding')

    def test_release_requires_successful_detach(self):
        plan, facts = fixture()
        facts[5]['data']['detach_success'] = False
        self.refused(plan, facts, 'release')

    def test_wait_must_be_natural_parents_actual_child(self):
        plan, facts = fixture()
        facts[6]['data']['pid'] = 999
        self.refused(plan, facts, 'wait-identity')

    def test_wait_stop_is_not_exit(self):
        plan, facts = fixture()
        facts[6]['data']['kind'] = 'stop'
        self.refused(plan, facts, 'wait-kind')

    def test_signal_return_remains_negative_signal_not_zero(self):
        plan, facts = fixture()
        facts[6]['data'].update(kind='signal', code=15)
        result = verify_transcript(plan, facts)
        self.assertEqual(result['returncode'], -15)
        self.assertTrue(result['invocation_complete'])

    def test_raw_sink_cannot_be_reassigned(self):
        plan, facts = fixture()
        facts[7]['data']['stdout'] = 'other-invocation'
        self.refused(plan, facts, 'raw-binding')

    def test_end_during_copy_refuses(self):
        plan, facts = fixture()
        for fact in facts[3:]:
            fact['at'] += 90.0
        self.refused(plan, facts, 'operation-deadline')

    def test_end_during_wait_refuses_without_reset(self):
        plan, facts = fixture()
        facts[6]['at'] = 90.0
        facts[7]['at'] = 91.0
        self.refused(plan, facts, 'operation-deadline')

    def test_end_after_hash_before_publication_refuses(self):
        plan, facts = fixture()
        facts[7]['at'] = 100.0
        self.refused(plan, facts, 'final-deadline')

    def test_producer_loss_is_first_cause_even_after_cleanup_error(self):
        plan, facts = fixture()
        facts[5]['kind'] = 'failure'
        facts[5]['data'] = dict(code='producer-lost')
        facts[6]['kind'] = 'failure'
        facts[6]['data'] = dict(code='cleanup-timeout')
        self.refused(plan, facts, 'producer-lost')

    def test_unknown_cleanup_is_not_quiet(self):
        plan, facts = fixture()
        facts[7]['data']['quiet'] = None
        self.refused(plan, facts, 'cleanup')

    def test_missing_confirmation_never_inferred_from_rc_zero(self):
        plan, facts = fixture()
        del facts[2]
        self.refused(plan, facts, 'event-order')

    def test_host_clocks_cannot_be_mixed(self):
        plan, facts = fixture()
        facts[2]['clock'] = 'other-host'
        self.refused(plan, facts, 'clock')

    def test_actual_environment_cannot_change_expected(self):
        plan, facts = fixture()
        facts[2]['data']['environment']['LD_LIBRARY_PATH'] = '/foreign'
        self.refused(plan, facts, 'entry-context')

    def test_future_image_claim_is_not_accepted_as_exhaustive(self):
        plan, facts = fixture()
        facts[7]['data']['later_images'] = 'ALL_A'
        self.refused(plan, facts, 'unsupported-claim')

    def test_post_release_transition_does_not_change_invocation_scope(self):
        plan, facts = fixture()
        # Future images are deliberately not observed by this producer. A model
        # annotation must not turn them into proof or alter the chosen entry.
        facts[7]['data']['later_images'] = 'UNKNOWN'
        result = verify_transcript(plan, facts)
        self.assertEqual(result['result_scope'], 'reviewed-program-invocation')
        self.assertEqual(result['later_images'], 'UNKNOWN')

    def test_mixed_phase_or_allocation_refused(self):
        plan, facts = fixture()
        facts[0]['data']['phase'] = 'other-phase'
        self.refused(plan, facts, 'selection-binding')

    def test_unadmitted_runner_identity_cannot_choose_its_own_cgroup(self):
        plan, facts = fixture()
        facts[1]['data']['cgroup'] = 'other-scope'
        self.refused(plan, facts, 'child-identity')

    def test_nonfinite_timestamp_cannot_bypass_deadline(self):
        plan, facts = fixture()
        facts[3]['at'] = float('nan')
        self.refused(plan, facts, 'clock')

    def test_final_copy_rehash_must_still_match_entry(self):
        plan, facts = fixture()
        facts[3]['data']['rehash'] = 'd' * 64
        self.refused(plan, facts, 'copy-content')
