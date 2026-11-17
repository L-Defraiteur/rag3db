"""Pure model validation only: no OS/backend or physical admission capability."""
from math import isfinite
from typing import Literal, Mapping, Sequence, TypedDict

class InvocationPlan(TypedDict):
    invocation: str
    source: str
    phase: str
    allocation: str
    nonce: str
    manifest: str  # independent APP/pilot/Q/locks/defaults/reviewed-program pins
    catalogue: str
    artifact: str
    features: tuple[str, ...]
    argv: tuple[str, ...]
    cwd: str
    environment: Mapping[str, str | None]  # allowlist only, never full environment
    stdout: str
    stderr: str
    authority: str
    plan_sha256: str  # frozen initial plan reference, excludes future facts/ACK
    common_owner: tuple[int, int]  # confiner owning its direct child, outside owned subtree
    common_root: tuple[int, int]  # direct Common child, NOT necessarily runner
    runner_pid: int  # independently admitted by Common, not from actual facts
    runner_start: int
    uid: int
    cgroup: str
    clock: str  # all timestamps in this transcript must share this host clock
    start: float  # original Common window, never reset on publication
    issued: float  # authority issuance, independently admitted on the same clock
    end: float
    operation_end: float
    cleanup_reserve: float
    storage_profile: Literal['local-regular-exec-write-denial-v1']

class PublicationAttestation(TypedDict):
    # Acquired AFTER entry/copy by CP owner, independently from submitted facts.
    plan_sha256: str
    reference: Mapping[str, object]

class Fact(TypedDict):
    # A fact is producer-owned, never supplied by the tested program as evidence.
    sequence: int
    invocation: str
    clock: str
    at: float
    kind: str
    data: Mapping[str, object]

class InvocationResult(TypedDict):
    invocation: str
    entry_confirmed: bool
    invocation_complete: bool  # not a judgement that rc==0 means tests passed
    returncode: int | None  # exit code or negative terminating signal, not shell encoding
    first_refusal: str | None
    later_images: Literal['UNKNOWN']
    sdk: Literal['NOT_ATTESTED']
    result_scope: Literal['reviewed-program-invocation']

class Refusal(RuntimeError):
    code: str
    sequence: int | None
    invocation_returncode: int | None  # preserved if authentic child wait acquired

    def __init__(self, code: str, *, sequence: int | None = None,
                 invocation_returncode: int | None = None) -> None:
        self.code = code
        self.sequence = sequence
        self.invocation_returncode = invocation_returncode
        super().__init__(code)

# These predicates inspect values only. No I/O, clocks, publication or backend.
def _integer(value: object, minimum: int = 0) -> bool:
    return type(value) is int and value >= minimum


def _number(value: object) -> bool:
    if type(value) is int:
        return -1.7976931348623157e308 <= value <= 1.7976931348623157e308
    return type(value) is float and isfinite(value)


def _text(value: object, *, empty: bool = False) -> bool:
    return type(value) is str and (empty or bool(value)) and '\0' not in value


def _sha(value: object) -> bool:
    return (type(value) is str and len(value) == 64
            and all(char in '0123456789abcdef' for char in value))


def _record(value: object, required: set[str], optional: set[str] | None = None) -> bool:
    return (isinstance(value, Mapping) and required <= set(value)
            and set(value) <= required | (optional or set()))


def _same(left: object, right: object) -> bool:
    # Python's True == 1 must never establish an identity or typed binding.
    if isinstance(left, Mapping) and isinstance(right, Mapping):
        return (set(left) == set(right)
                and all(_same(left[key], right[key]) for key in left))
    if type(left) is not type(right):
        return False
    if type(left) in (tuple, list):
        return len(left) == len(right) and all(_same(a, b) for a, b in zip(left, right))
    return type(left) in (str, int, float, bool, type(None)) and left == right


def _process_pair(value: object) -> bool:
    return (type(value) is tuple and len(value) == 2
            and _integer(value[0], 2) and _integer(value[1], 1))


def _object(value: object) -> bool:
    return (_record(value, {'dev', 'inode', 'sha256'})
            and _integer(value['dev']) and _integer(value['inode'], 1)
            and _sha(value['sha256']))


def _environment(value: object) -> bool:
    return (isinstance(value, Mapping)
            and all(_text(key) and (item is None or _text(item, empty=True))
                    for key, item in value.items()))


def _strings(value: object, *, nonempty: bool = False) -> bool:
    return (type(value) is tuple and (not nonempty or bool(value))
            and all(_text(item, empty=True) for item in value))


def _check_plan(plan: InvocationPlan) -> None:
    if not _record(plan, set(InvocationPlan.__annotations__)):
        raise Refusal('plan-fields')
    text_fields = ('invocation', 'source', 'phase', 'allocation', 'nonce', 'manifest',
                   'catalogue', 'cwd', 'stdout', 'stderr', 'authority', 'cgroup', 'clock')
    if not all(_text(plan[key]) for key in text_fields):
        raise Refusal('plan-types')
    if not (_sha(plan['artifact']) and _sha(plan['plan_sha256'])
            and _strings(plan['features']) and len(set(plan['features'])) == len(plan['features'])
            and _strings(plan['argv'], nonempty=True) and _environment(plan['environment'])
            and plan['cwd'].startswith('/') and plan['stdout'] != plan['stderr']
            and _integer(plan['runner_pid'], 2) and _integer(plan['runner_start'], 1)
            and _integer(plan['uid']) and _process_pair(plan['common_owner'])
            and _process_pair(plan['common_root'])
            and plan['common_owner'][0] not in (plan['common_root'][0], plan['runner_pid'])
            and plan['storage_profile'] == 'local-regular-exec-write-denial-v1'):
        raise Refusal('plan-types')
    if (plan['common_root'][0] == plan['runner_pid']
            and plan['common_root'][1] != plan['runner_start']):
        raise Refusal('plan-topology')
    fields = ('start', 'issued', 'end', 'operation_end', 'cleanup_reserve')
    if not all(_number(plan[key]) for key in fields):
        raise Refusal('clock')
    if not (plan['cleanup_reserve'] > 0
            and 0 <= plan['start'] <= plan['issued'] < plan['operation_end']
            <= plan['end'] - plan['cleanup_reserve'] < plan['end']):
        raise Refusal('authority-window')


def verify_transcript(plan: InvocationPlan, facts: Sequence[Fact],
                      publication: PublicationAttestation | None) -> InvocationResult:
    """Validate model facts, never perform or attest the physical protocol.

    The caller supplies an independently trusted initial plan and, if acquired,
    a separate CP attestation. Hash references are opaque at this pure layer.
    A Refusal terminates at the first invalid event, before later cleanup facts.
    """
    _check_plan(plan)
    if type(facts) not in (list, tuple):
        raise Refusal('event-order')
    order = ('selected', 'spawned', 'exec_stop', 'copied', 'published',
             'released', 'waited', 'finalized', 'common_root_waited', 'common_closed')
    position = 0
    last_sequence = -1
    last_time = plan['issued']
    child = None
    selected_object = None
    entry_object = None
    returncode = None
    sequence = None

    def demand(condition: bool, code: str) -> None:
        if not condition:
            raise Refusal(code, sequence=sequence, invocation_returncode=returncode)

    def identity(data: Mapping[str, object], code: str) -> None:
        demand(_integer(data.get('pid'), 2) and _integer(data.get('start'), 1)
               and _integer(data.get('uid')) and _text(data.get('cgroup')), code)
        demand(_same({key: data[key] for key in ('pid', 'start', 'uid', 'cgroup')}, child), code)

    for fact in facts:
        demand(_record(fact, set(Fact.__annotations__)), 'event-fields')
        candidate_sequence = fact['sequence']
        demand(_integer(candidate_sequence) and candidate_sequence > last_sequence, 'event-order')
        sequence = candidate_sequence
        last_sequence = sequence
        demand(_same(fact['invocation'], plan['invocation']), 'invocation-binding')
        demand(_same(fact['clock'], plan['clock']) and _number(fact['at']), 'clock')
        at = fact['at']
        demand(at >= plan['issued'], 'before-issuance')
        demand(at >= last_time, 'clock')
        last_time = at
        demand(_text(fact['kind']) and isinstance(fact['data'], Mapping), 'event-fields')
        kind, data = fact['kind'], fact['data']
        # A pre-publication failure must not require a future CP attestation.
        if kind == 'failure':
            demand(_record(data, {'code'}) and _text(data['code']), 'failure-fields')
            raise Refusal(data['code'], sequence=sequence, invocation_returncode=returncode)
        demand(position < len(order) and kind == order[position], 'event-order')
        deadline = plan['operation_end'] if position <= 6 else plan['end']
        demand(at < deadline, 'operation-deadline' if position <= 6 else 'final-deadline')

        if kind == 'selected':
            fields = {'artifact', 'source', 'phase', 'allocation', 'nonce', 'manifest',
                      'catalogue', 'features', 'argv', 'cwd', 'environment',
                      'stdout', 'stderr', 'authority', 'storage_profile'}
            demand(_record(data, fields | {'object'}), 'selection-binding')
            demand(all(_same(data[key], plan[key]) for key in fields), 'selection-binding')
            demand(_object(data['object']) and data['object']['sha256'] == plan['artifact'],
                   'selection-object')
            selected_object = dict(data['object'])
        elif kind == 'spawned':
            demand(_record(data, {'pid', 'start', 'uid', 'cgroup', 'ppid',
                                  'runner_pid', 'runner_start'}), 'child-identity')
            demand(_integer(data['pid'], 2)
                   and data['pid'] not in (plan['runner_pid'], plan['common_owner'][0],
                                           plan['common_root'][0])
                   and _integer(data['start'], 1) and _integer(data['uid'])
                   and _same(data['uid'], plan['uid'])
                   and _same(data['cgroup'], plan['cgroup'])
                   and _integer(data['ppid'], 2) and data['ppid'] == plan['runner_pid']
                   and _integer(data['runner_pid'], 2) and data['runner_pid'] == plan['runner_pid']
                   and _integer(data['runner_start'], 1)
                   and data['runner_start'] == plan['runner_start'], 'child-identity')
            child = {key: data[key] for key in ('pid', 'start', 'uid', 'cgroup')}
        elif kind == 'exec_stop':
            demand(_record(data, {'pid', 'start', 'uid', 'cgroup', 'event', 'object',
                                  'stable', 'argv', 'cwd', 'environment'}), 'entry-fields')
            identity(data, 'child-identity')
            demand(_same(data['event'], 'EXEC_SUCCESS'), 'entry-event')
            demand(_object(data['object']), 'entry-object')
            demand(all(_same(data['object'][key], selected_object[key])
                       for key in ('dev', 'inode')), 'entry-object')
            demand(data['object']['sha256'] == plan['artifact'], 'entry-content')
            demand(data['stable'] is True, 'entry-stability')
            demand(all(_same(data[key], plan[key]) for key in ('argv', 'cwd', 'environment')),
                   'entry-context')
            entry_object = dict(data['object'])
        elif kind == 'copied':
            demand(_record(data, {'pid', 'start', 'uid', 'cgroup', 'source_object',
                                  'complete', 'sha256', 'rehash'}), 'copy-fields')
            identity(data, 'child-identity')
            demand(_object(data['source_object']) and _same(data['source_object'], entry_object),
                   'copy-object')
            demand(data['complete'] is True and _sha(data['sha256']) and _sha(data['rehash'])
                   and data['sha256'] == data['rehash'] == plan['artifact'], 'copy-content')
        elif kind == 'published':
            demand(publication is not None, 'publication-missing')
            demand(_record(publication, {'plan_sha256', 'reference'})
                   and _same(publication['plan_sha256'], plan['plan_sha256']), 'publication-binding')
            reference = publication['reference']
            ref_fields = {'prepared', 'plan', 'phase', 'allocation', 'nonce', 'sequence',
                          'ledger_sha256', 'entry_facts_sha256', 'ack_receipt_sha256'}
            demand(_record(reference, ref_fields), 'publication-binding')
            demand(_text(reference['prepared']) and _text(reference['plan'])
                   and _integer(reference['sequence'])
                   and all(_sha(reference[key]) for key in
                           ('ledger_sha256', 'entry_facts_sha256', 'ack_receipt_sha256'))
                   and all(_same(reference[key], plan[key]) for key in
                           ('phase', 'allocation', 'nonce')), 'publication-binding')
            demand(_record(data, {'invocation', 'catalogue', 'copied_sha256', 'reference'})
                   and _same(data['invocation'], plan['invocation'])
                   and _same(data['catalogue'], plan['catalogue'])
                   and _same(data['copied_sha256'], plan['artifact'])
                   and _same(data['reference'], reference), 'publication-binding')
        elif kind == 'released':
            demand(_record(data, {'pid', 'start', 'uid', 'cgroup', 'detach_success'}), 'release')
            identity(data, 'child-identity')
            demand(data['detach_success'] is True, 'release')
        elif kind == 'waited':
            demand(_record(data, {'pid', 'start', 'uid', 'cgroup', 'parent_pid', 'kind', 'code'}),
                   'wait-fields')
            identity(data, 'wait-identity')
            demand(_integer(data['parent_pid'], 2) and data['parent_pid'] == plan['runner_pid'],
                   'wait-identity')
            demand(data['kind'] in ('exit', 'signal') and type(data['kind']) is str, 'wait-kind')
            if data['kind'] == 'exit':
                demand(_integer(data['code']) and data['code'] <= 255, 'wait-code')
                returncode = data['code']
            else:
                demand(_integer(data['code'], 1) and data['code'] <= 127, 'wait-code')
                returncode = -data['code']
        elif kind == 'finalized':
            if 'later_images' in data:
                demand(_same(data['later_images'], 'UNKNOWN'), 'unsupported-claim')
            demand(_record(data, {'stdout', 'stderr', 'stdout_sha256', 'stderr_sha256', 'published'},
                           {'later_images'}), 'raw-binding')
            demand(_same(data['stdout'], plan['stdout']) and _same(data['stderr'], plan['stderr'])
                   and _sha(data['stdout_sha256']) and _sha(data['stderr_sha256'])
                   and data['published'] is True, 'raw-binding')
        elif kind == 'common_root_waited':
            demand(_record(data, {'pid', 'start', 'parent_pid', 'kind', 'code'}), 'common-root-wait')
            demand(_integer(data['pid'], 2) and _integer(data['start'], 1)
                   and _same((data['pid'], data['start']), plan['common_root'])
                   and _integer(data['parent_pid'], 2)
                   and data['parent_pid'] == plan['common_owner'][0], 'common-root-wait')
            demand(type(data['kind']) is str and data['kind'] == 'exit'
                   and _integer(data['code']) and data['code'] == 0, 'common-root-failed')
        elif kind == 'common_closed':
            demand(_record(data, {'owner', 'authority', 'quiet', 'reaped',
                                  'runner_absent', 'published'}), 'cleanup')
            demand(_process_pair(data['owner']) and _same(data['owner'], plan['common_owner']),
                   'closure-owner')
            demand(_same(data['authority'], plan['authority']) and data['quiet'] is True
                   and data['reaped'] is True and data['runner_absent'] is True
                   and data['published'] is True, 'cleanup')
        position += 1

    demand(position == len(order), 'event-order')
    return {'invocation': plan['invocation'], 'entry_confirmed': True,
            'invocation_complete': True, 'returncode': returncode, 'first_refusal': None,
            'later_images': 'UNKNOWN', 'sdk': 'NOT_ATTESTED',
            'result_scope': 'reviewed-program-invocation'}
