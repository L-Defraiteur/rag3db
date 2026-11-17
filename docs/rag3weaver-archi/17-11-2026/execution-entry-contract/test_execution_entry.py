"""Prepublished model witnesses. NOT RUN; shared pure baseline explicitly refuses.

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
                clock='host-1', start=0.0, issued=0.5, end=100.0, operation_end=90.0,
                cleanup_reserve=10.0,
                storage_profile='local-regular-exec-write-denial-v1',
                common_owner=(90, 9), common_root=(95, 10),
                plan_sha256='9' * 64)
    publication = dict(plan_sha256='9' * 64, reference=dict(prepared='prepared-1', plan='plan-1', phase='lib',
                                 allocation='allocation-1', nonce='nonce-1',
                                 sequence=1, ledger_sha256='d' * 64,
                                 entry_facts_sha256='e' * 64,
                                 ack_receipt_sha256='f' * 64))
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
                           copied_sha256='a' * 64,
                           reference=deepcopy(publication['reference']))),
        ('released', dict(identity, detach_success=True)),
        ('waited', dict(identity, parent_pid=100, kind='exit', code=0)),
        ('finalized', dict(stdout='raw-stdout', stderr='raw-stderr',
                           stdout_sha256='b' * 64, stderr_sha256='c' * 64,
                           published=True)),
        ('common_root_waited', dict(pid=95, start=10, parent_pid=90,
                                    kind='exit', code=0)),
        ('common_closed', dict(owner=(90, 9), authority='common-1',
                               quiet=True, reaped=True, runner_absent=True, published=True)),
    ]
    facts = [dict(sequence=i, invocation='inv-1', clock='host-1',
                  at=float(i + 1), kind=kind, data=data)
             for i, (kind, data) in enumerate(records)]
    return plan, facts, publication


class EntryContract(unittest.TestCase):
    def refused(self, plan, facts, publication, code):
        with self.assertRaises(Refusal) as caught:
            verify_transcript(plan, facts, publication)
        self.assertEqual(caught.exception.code, code)

    def test_confirmed_entry_result_belongs_to_invocation(self):
        plan, facts, publication = fixture()
        try:
            result = verify_transcript(plan, facts, publication)
        except Refusal as error:
            if error.code != 'capability-not-implemented':
                raise
            self.fail('pure entry validation capability is not implemented')
        self.assertTrue(result['entry_confirmed'])
        self.assertTrue(result['invocation_complete'])
        self.assertEqual(result['returncode'], 0)
        self.assertEqual(result['result_scope'], 'reviewed-program-invocation')
        self.assertEqual(result['later_images'], 'UNKNOWN')
        self.assertEqual(result['sdk'], 'NOT_ATTESTED')

    def test_path_substitution_cannot_promote_foreign_object(self):
        plan, facts, publication = fixture()
        facts[2]['data']['object']['inode'] = 999
        self.refused(plan, facts, publication, 'entry-object')

    def test_same_inode_changed_bytes_refused(self):
        plan, facts, publication = fixture()
        facts[2]['data']['object']['sha256'] = 'd' * 64
        self.refused(plan, facts, publication, 'entry-content')

    def test_fd_and_two_hashes_without_stability_not_enough(self):
        plan, facts, publication = fixture()
        facts[2]['data']['stable'] = False
        self.refused(plan, facts, publication, 'entry-stability')

    def test_eof_is_not_exec_confirmation(self):
        plan, facts, publication = fixture()
        facts[2]['data']['event'] = 'CLOEXEC_EOF'
        self.refused(plan, facts, publication, 'entry-event')

    def test_stop_is_not_successful_exec(self):
        plan, facts, publication = fixture()
        facts[2]['data']['event'] = 'SIGSTOP'
        self.refused(plan, facts, publication, 'entry-event')

    def test_death_before_confirmation_cannot_be_completed_by_cleanup(self):
        plan, facts, publication = fixture()
        facts[2]['kind'] = 'failure'
        facts[2]['data'] = dict(code='child-died-before-entry')
        publication = None
        self.refused(plan, facts, publication, 'child-died-before-entry')

    def test_runner_pid_not_child_pid(self):
        plan, facts, publication = fixture()
        facts[1]['data']['pid'] = plan['runner_pid']
        self.refused(plan, facts, publication, 'child-identity')

    def test_pid_reuse_before_entry_refused(self):
        plan, facts, publication = fixture()
        facts[2]['data']['start'] += 1
        self.refused(plan, facts, publication, 'child-identity')

    def test_copy_must_be_of_actual_entry_object(self):
        plan, facts, publication = fixture()
        facts[3]['data']['source_object']['inode'] = 999
        self.refused(plan, facts, publication, 'copy-object')

    def test_partial_or_mutated_copy_cannot_release(self):
        plan, facts, publication = fixture()
        facts[3]['data']['complete'] = False
        self.refused(plan, facts, publication, 'copy-content')

    def test_ack_does_not_authorize_new_catalogue(self):
        plan, facts, publication = fixture()
        facts[4]['data']['catalogue'] = 'actual-B'
        self.refused(plan, facts, publication, 'publication-binding')

    def test_release_requires_successful_detach(self):
        plan, facts, publication = fixture()
        facts[5]['data']['detach_success'] = False
        self.refused(plan, facts, publication, 'release')

    def test_wait_must_be_natural_parents_actual_child(self):
        plan, facts, publication = fixture()
        facts[6]['data']['pid'] = 999
        self.refused(plan, facts, publication, 'wait-identity')

    def test_wait_stop_is_not_exit(self):
        plan, facts, publication = fixture()
        facts[6]['data']['kind'] = 'stop'
        self.refused(plan, facts, publication, 'wait-kind')

    def test_signal_return_remains_negative_signal_not_zero(self):
        plan, facts, publication = fixture()
        facts[6]['data'].update(kind='signal', code=15)
        result = verify_transcript(plan, facts, publication)
        self.assertEqual(result['returncode'], -15)
        self.assertTrue(result['invocation_complete'])

    def test_raw_sink_cannot_be_reassigned(self):
        plan, facts, publication = fixture()
        facts[7]['data']['stdout'] = 'other-invocation'
        self.refused(plan, facts, publication, 'raw-binding')

    def test_end_during_copy_refuses(self):
        plan, facts, publication = fixture()
        for fact in facts[3:]:
            fact['at'] += 90.0
        self.refused(plan, facts, publication, 'operation-deadline')

    def test_end_during_wait_refuses_without_reset(self):
        plan, facts, publication = fixture()
        for i, fact in enumerate(facts[6:]):
            fact['at'] = 90.0 + i
        self.refused(plan, facts, publication, 'operation-deadline')

    def test_end_after_hash_before_publication_refuses(self):
        plan, facts, publication = fixture()
        for i, fact in enumerate(facts[7:]):
            fact['at'] = 100.0 + i
        self.refused(plan, facts, publication, 'final-deadline')

    def test_producer_loss_is_first_cause_even_after_cleanup_error(self):
        plan, facts, publication = fixture()
        facts[5]['kind'] = 'failure'
        facts[5]['data'] = dict(code='producer-lost')
        facts[6]['kind'] = 'failure'
        facts[6]['data'] = dict(code='cleanup-timeout')
        self.refused(plan, facts, publication, 'producer-lost')

    def test_unknown_cleanup_is_not_quiet(self):
        plan, facts, publication = fixture()
        facts[9]['data']['quiet'] = None
        self.refused(plan, facts, publication, 'cleanup')

    def test_missing_confirmation_never_inferred_from_rc_zero(self):
        plan, facts, publication = fixture()
        del facts[2]
        self.refused(plan, facts, publication, 'event-order')

    def test_host_clocks_cannot_be_mixed(self):
        plan, facts, publication = fixture()
        facts[2]['clock'] = 'other-host'
        self.refused(plan, facts, publication, 'clock')

    def test_actual_environment_cannot_change_expected(self):
        plan, facts, publication = fixture()
        facts[2]['data']['environment']['LD_LIBRARY_PATH'] = '/foreign'
        self.refused(plan, facts, publication, 'entry-context')

    def test_future_image_claim_is_not_accepted_as_exhaustive(self):
        plan, facts, publication = fixture()
        facts[7]['data']['later_images'] = 'ALL_A'
        self.refused(plan, facts, publication, 'unsupported-claim')

    def test_post_release_transition_does_not_change_invocation_scope(self):
        plan, facts, publication = fixture()
        # Future images are deliberately not observed by this producer. A model
        # annotation must not turn them into proof or alter the chosen entry.
        facts[7]['data']['later_images'] = 'UNKNOWN'
        result = verify_transcript(plan, facts, publication)
        self.assertEqual(result['result_scope'], 'reviewed-program-invocation')
        self.assertEqual(result['later_images'], 'UNKNOWN')

    def test_mixed_phase_or_allocation_refused(self):
        plan, facts, publication = fixture()
        facts[0]['data']['phase'] = 'other-phase'
        self.refused(plan, facts, publication, 'selection-binding')

    def test_unadmitted_runner_identity_cannot_choose_its_own_cgroup(self):
        plan, facts, publication = fixture()
        facts[1]['data']['cgroup'] = 'other-scope'
        self.refused(plan, facts, publication, 'child-identity')

    def test_nonfinite_timestamp_cannot_bypass_deadline(self):
        plan, facts, publication = fixture()
        facts[3]['at'] = float('nan')
        self.refused(plan, facts, publication, 'clock')

    def test_final_copy_rehash_must_still_match_entry(self):
        plan, facts, publication = fixture()
        facts[3]['data']['rehash'] = 'd' * 64
        self.refused(plan, facts, publication, 'copy-content')

    def test_ack_from_other_phase_is_not_a_binding(self):
        plan, facts, publication = fixture()
        facts[4]['data']['reference']['phase'] = 'other-phase'
        self.refused(plan, facts, publication, 'publication-binding')

    def test_ack_for_other_entry_facts_is_not_a_binding(self):
        plan, facts, publication = fixture()
        facts[4]['data']['reference']['entry_facts_sha256'] = '0' * 64
        self.refused(plan, facts, publication, 'publication-binding')

    def test_boolean_ack_cannot_replace_exact_cp_reference(self):
        plan, facts, publication = fixture()
        facts[4]['data']['reference'] = True
        self.refused(plan, facts, publication, 'publication-binding')

    def test_runner_cannot_close_its_own_common_ancestor(self):
        plan, facts, publication = fixture()
        facts[9]['data']['owner'] = (100, 11)
        self.refused(plan, facts, publication, 'closure-owner')

    def test_common_closure_cannot_precede_common_root_wait(self):
        plan, facts, publication = fixture()
        del facts[8]
        self.refused(plan, facts, publication, 'event-order')

    def test_missing_external_closure_never_inferred_from_child_rc(self):
        plan, facts, publication = fixture()
        del facts[9]
        self.refused(plan, facts, publication, 'event-order')

    def test_facts_before_authority_issuance_are_not_in_window(self):
        plan, facts, publication = fixture()
        plan['issued'] = 1.5
        self.refused(plan, facts, publication, 'before-issuance')

    def test_authority_cannot_predate_original_common_window(self):
        plan, facts, publication = fixture()
        plan['issued'] = -1.0
        self.refused(plan, facts, publication, 'authority-window')

    def test_new_issuance_after_operation_end_cannot_reset_budget(self):
        plan, facts, publication = fixture()
        plan['issued'] = 91.0
        self.refused(plan, facts, publication, 'authority-window')

    def test_future_ack_is_not_part_of_frozen_initial_plan(self):
        plan, facts, publication = fixture()
        self.assertNotIn('publication', plan)
        self.assertNotIn('ack_receipt_sha256', plan)
        result = verify_transcript(plan, facts, publication)
        self.assertTrue(result['entry_confirmed'])

    def test_independent_cp_attestation_must_bind_initial_plan(self):
        plan, facts, publication = fixture()
        publication['plan_sha256'] = '0' * 64
        self.refused(plan, facts, publication, 'publication-binding')

    def test_common_root_wait_is_not_fabricated_as_runner_wait(self):
        plan, facts, publication = fixture()
        facts[8]['data']['pid'] = plan['runner_pid']
        facts[8]['data']['start'] = plan['runner_start']
        self.refused(plan, facts, publication, 'common-root-wait')

    def test_producer_failure_after_child_zero_does_not_become_success(self):
        plan, facts, publication = fixture()
        facts[7]['kind'] = 'failure'
        facts[7]['data'] = dict(code='producer-failed-after-wait')
        with self.assertRaises(Refusal) as caught:
            verify_transcript(plan, facts, publication)
        self.assertEqual(caught.exception.code, 'producer-failed-after-wait')
        self.assertEqual(caught.exception.invocation_returncode, 0)

    def test_common_root_signal_after_child_zero_is_not_success(self):
        plan, facts, publication = fixture()
        facts[8]['data'].update(kind='signal', code=15)
        with self.assertRaises(Refusal) as caught:
            verify_transcript(plan, facts, publication)
        self.assertEqual(caught.exception.code, 'common-root-failed')
        self.assertEqual(caught.exception.invocation_returncode, 0)

    def test_death_before_publication_does_not_require_future_ack(self):
        plan, facts, publication = fixture()
        facts[4]['kind'] = 'failure'
        facts[4]['data'] = dict(code='child-died-before-publication')
        self.refused(plan, facts, None, 'child-died-before-publication')

    def test_published_requires_independent_attestation(self):
        plan, facts, publication = fixture()
        self.refused(plan, facts, None, 'publication-missing')
