"""FIXTURE_ONLY effect orchestration, published after the baseline red.

No OS backend is delivered. Importing this module has no protocol side effects.
The pure validator execution_entry.py and its evidence are independent/frozen.
"""
from collections.abc import Mapping
from hashlib import sha256
from types import MappingProxyType
# Value predicates reused from the frozen, pinned pure validator; no facts are
# fabricated to call verify_transcript and no physical admission is inherited.
from execution_entry import _number as _finite, _sha, _same


class RunnerRefusal(RuntimeError):
    """Functional refusal; cleanup cannot replace the first cause."""
    def __init__(self, code: str, *, first_cause: BaseException | None = None,
                 cleanup: Mapping[str, object] | None = None,
                 invocation_returncode: int | None = None) -> None:
        self.code = code
        self.first_cause = first_cause
        self.cleanup = cleanup
        self.invocation_returncode = invocation_returncode
        super().__init__(code)


def _freeze_request(value: object) -> object:
    if isinstance(value, Mapping):
        if not all(type(key) is str for key in value):
            raise RunnerRefusal('request-fields')
        return MappingProxyType({key: _freeze_request(item) for key, item in value.items()})
    if type(value) is tuple:
        return tuple(_freeze_request(item) for item in value)
    if type(value) in (str, int, float, bool, type(None)):
        return value
    raise RunnerRefusal('request-types')


def _port_view(value: object) -> object:
    # Never expose the immutable policy OR the caller's original containers.
    if isinstance(value, Mapping):
        return {key: _port_view(item) for key, item in value.items()}
    if type(value) is tuple:
        return tuple(_port_view(item) for item in value)
    return value


def run_fixture_invocation(request: Mapping[str, object], effects: object) -> Mapping[str, object]:
    """Orchestrate MODELED fixture ports, never issue an OS/CP admission.

    The native CLI and Common adapter remain unsupported. This entry point's
    return value ALWAYS labels its facts MODELED, even if a port claims otherwise.
    A real native adapter will need separate source/pins/review/GO before use.
    """
    if not isinstance(request, Mapping) or request.get('mode') != 'FIXTURE_ONLY':
        raise RunnerRefusal('unsupported-live')
    request = _freeze_request(request)
    required = {'mode', 'invocation', 'phase', 'declaration_sha256', 'runtime_plan_sha256',
                'artifact_sha256', 'object_identity', 'argv', 'cwd', 'public_environment',
                'clock_domain', 'start', 'issued', 'operation_end', 'end',
                'observation_cutoff', 'publication_files'}
    if set(request) != required:
        raise RunnerRefusal('request-fields')
    for key in ('invocation', 'phase', 'cwd', 'clock_domain'):
        if type(request[key]) is not str or not request[key] or '\0' in request[key]:
            raise RunnerRefusal('request-types')
    if not all(_sha(request[key]) for key in
               ('declaration_sha256', 'runtime_plan_sha256', 'artifact_sha256')):
        raise RunnerRefusal('request-types')
    identity = request['object_identity']
    if not (type(identity) is tuple and len(identity) == 2
            and all(type(part) is int and part >= 0 for part in identity)):
        raise RunnerRefusal('request-types')
    if not (type(request['argv']) is tuple and request['argv']
            and all(type(arg) is str and '\0' not in arg for arg in request['argv'])
            and request['cwd'].startswith('/')
            and isinstance(request['public_environment'], Mapping)
            and all(type(key) is str and key and '\0' not in key
                    and (value is None or type(value) is str and '\0' not in value)
                    for key, value in request['public_environment'].items())):
        raise RunnerRefusal('request-types')
    if not _same(request['publication_files'],
                 ('entry-ledger.json', 'entry-object', 'entry-facts.json')):
        raise RunnerRefusal('publication-payload')
    times = ('start', 'issued', 'operation_end', 'end', 'observation_cutoff')
    if not (all(_finite(request[key]) for key in times)
            and 0 <= request['start'] <= request['issued'] < request['operation_end']
            < request['end'] <= request['observation_cutoff']):
        raise RunnerRefusal('authority-window')

    launch = None
    held = None
    returncode = None
    last_now = request['issued']

    def require(condition: bool, code: str) -> None:
        if not condition:
            raise RunnerRefusal(code, invocation_returncode=returncode)

    def clock() -> float:
        nonlocal last_now
        value = effects.now()
        require(_finite(value) and value >= last_now, 'clock')
        last_now = value
        return value

    def check(final: bool = False) -> None:
        end = request['end'] if final else request['operation_end']
        require(clock() < end, 'final-deadline' if final else 'operation-deadline')

    def operation(method, *args, final: bool = False):
        check(final)
        value = method(*args)
        check(final)
        return value

    try:
        check()
        # Keep the token before any post-call clock/check that could fail.
        launch = effects.prepare_owned_launch(_port_view(request))
        require(launch is not None, 'launch-ownership')
        check()
        require(operation(effects.probe_write, launch, 'before') == 'WRITABLE',
                'storage-profile-unqualified')
        operation(effects.fork_owned, launch)
        operation(effects.acquire_identity_and_pidfd, launch)
        held, actual = operation(effects.start_and_hold, launch)
        require(held is not None and isinstance(actual, Mapping), 'entry-event')
        require(_same(actual.get('event'), 'EXEC_SUCCESS'), 'entry-event')
        require(_same(actual.get('object_identity'), request['object_identity']), 'entry-object')
        require(_same(actual.get('sha256'), request['artifact_sha256']), 'entry-content')
        require(_same(actual.get('argv'), request['argv'])
                and _same(actual.get('cwd'), request['cwd'])
                and _same(actual.get('environment'), request['public_environment']), 'entry-context')
        require(operation(effects.probe_write, launch, 'held') == 'ETXTBSY',
                'storage-profile-unqualified')
        seal, facts = operation(effects.seal_entry, launch, held)
        require(seal is not None and isinstance(facts, Mapping)
                and facts.get('declared_payload') is True and _sha(facts.get('sha256')),
                'publication-payload')
        permit, binding = operation(effects.edit_fixture_only, launch, seal)
        require(permit is not None and isinstance(binding, Mapping)
                and set(binding) == {'mode', 'declaration_sha256', 'runtime_plan_sha256', 'facts_sha256'}
                and _same(binding['mode'], 'FIXTURE_ONLY')
                and _same(binding['declaration_sha256'], request['declaration_sha256'])
                and _same(binding['runtime_plan_sha256'], request['runtime_plan_sha256'])
                and _same(binding['facts_sha256'], facts['sha256']), 'publication-binding')
        operation(effects.release, launch, held, permit)
        check()
        exit_record = effects.wait_child(launch)
        require(isinstance(exit_record, Mapping), 'wait-kind')
        kind, code = exit_record.get('kind'), exit_record.get('code')
        require(type(kind) is str and kind in ('exit', 'signal') and type(code) is int,
                'wait-kind')
        require(0 <= code <= 255 if kind == 'exit' else 1 <= code <= 127, 'wait-code')
        returncode = code if kind == 'exit' else -code
        # Record authentic returncode before a late post-wait refusal.
        check()
        require(type(exit_record.get('stdout')) is bytes and type(exit_record.get('stderr')) is bytes,
                'raw-binding')
        require(operation(effects.probe_write, launch, 'after') == 'WRITABLE',
                'storage-profile-unqualified')
        finish = operation(effects.finish_fixture, launch, final=True)
        require(isinstance(finish, Mapping) and _same(finish.get('mode'), 'FIXTURE_ONLY')
                and _same(finish.get('common_origin'), 'MODELED'), 'unsupported-common-closure')
        check(final=True)
        result = {'entry_confirmed': True, 'returncode': returncode,
                'artifact_sha256': request['artifact_sha256'], 'facts_sha256': facts['sha256'],
                'stdout_sha256': sha256(exit_record['stdout']).hexdigest(),
                'stderr_sha256': sha256(exit_record['stderr']).hexdigest(),
                'mode': 'FIXTURE_ONLY', 'proof_origin': 'MODELED', 'common_origin': 'MODELED',
                'cp_ack': False, 'global_admission': False, 'later_images': 'UNKNOWN',
                'sdk': 'NOT_ATTESTED', 'original_end': request['end'],
                'result_scope': 'reviewed-program-invocation'}
        check(final=True)  # Fresh AFTER both hashes and full result construction.
        return result
    except BaseException as error:
        # A known launch token survives every failure, including pre-HeldEntry.
        cleanup = {'origin': 'MODELED', 'quiet': False, 'reaped': False, 'remaining': 'UNKNOWN'}
        if launch is not None:
            try:
                observed = effects.cancel_owned(launch, error)
                if (isinstance(observed, Mapping) and observed.get('origin') == 'MODELED'
                        and type(observed.get('quiet')) is bool
                        and type(observed.get('reaped')) is bool):
                    cleanup = dict(observed)
                    if not (observed['quiet'] is True and observed['reaped'] is True
                            and type(observed.get('remaining')) is list
                            and observed['remaining'] == []):
                        cleanup.update(quiet=False, remaining='UNKNOWN')
                else:
                    cleanup['error_type'] = 'invalid-cleanup-record'
            except BaseException as secondary:
                cleanup['error_type'] = type(secondary).__name__
        else:
            # prepare_owned_launch is contractually before fork; no child exists.
            cleanup.update(quiet=True, reaped=True, remaining=[], launch_created=False)
        try:
            observed_at = clock()
            cleanup.update(observed_at=observed_at, late=observed_at >= request['end'],
                           overrun=max(0.0, observed_at - request['end']),
                           observation_cutoff_crossed=observed_at >= request['observation_cutoff'],
                           observation_overrun=max(0.0, observed_at - request['observation_cutoff']))
        except BaseException:
            cleanup.update(clock_unknown=True, remaining='UNKNOWN')
        if isinstance(error, RunnerRefusal):
            error.cleanup = cleanup
            if error.invocation_returncode is None:
                error.invocation_returncode = returncode
            raise
        if isinstance(error, OSError):
            raise RunnerRefusal('startup-failed' if held is None else 'effect-failed',
                                first_cause=error, cleanup=cleanup,
                                invocation_returncode=returncode) from error
        # Import/fixture/programming exceptions never become functional RED.
        raise


def collect_common_closure(*args: object, **kwargs: object) -> Mapping[str, object]:
    """No unknown Common relationship may become a fixture fallback."""
    raise RunnerRefusal('unsupported-common-closure')


def main() -> int:
    """CLI deliberately unavailable until a separately reviewed native backend."""
    raise RunnerRefusal('unsupported-native-backend')


if __name__ == '__main__':
    raise SystemExit(main())
