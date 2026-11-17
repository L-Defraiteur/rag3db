"""Pure refusal baseline. No OS/backend or physical admission capability."""
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

# PURE future validator: no files, clocks, subprocesses, signals, publication.
# Raises Refusal on incomplete/invalid transcript, preserving earliest failure.
# None is valid before published; a preceding failure keeps its first cause.
# published without an independently acquired attestation refuses.
# Model evidence is accepted ONLY by this pure review layer, never OS admission.
def verify_transcript(plan: InvocationPlan, facts: Sequence[Fact], publication: PublicationAttestation | None) -> InvocationResult:
    # Fail closed until the pure contract has its separately reviewed implementation.
    # This does not inspect facts, attest an image or acquire any external capability.
    raise Refusal('capability-not-implemented')

# Future operational entry point, deliberately NOT specified as a callable yet:
# launch/observe/release/wait must implement README protocol after its RO.
# No caller should dynamically fabricate a receipt from verify_transcript output.
