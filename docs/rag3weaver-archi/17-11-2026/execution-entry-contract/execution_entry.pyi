"""Proposed API, no implementation or OS capability. Protocol review required."""
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
    publication: Mapping[str, object]  # exact independently verified CP reference
    common_owner: tuple[int, int]  # external closer PID/start, outside owned subtree
    runner_pid: int  # independently admitted by Common, not from actual facts
    runner_start: int
    uid: int
    cgroup: str
    clock: str  # all timestamps in this transcript must share this host clock
    end: float
    operation_end: float
    cleanup_reserve: float
    storage_profile: Literal['local-regular-exec-write-denial-v1']

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

# PURE future validator: no files, clocks, subprocesses, signals, publication.
# Raises Refusal on incomplete/invalid transcript, preserving earliest failure.
# Model evidence is accepted ONLY by this pure review layer, never OS admission.
def verify_transcript(plan: InvocationPlan, facts: Sequence[Fact]) -> InvocationResult: ...

# Future operational entry point, deliberately NOT specified as a callable yet:
# launch/observe/release/wait must implement README protocol after its RO.
# No caller should dynamically fabricate a receipt from verify_transcript output.
