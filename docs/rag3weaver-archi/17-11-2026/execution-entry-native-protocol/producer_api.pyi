"""Proposed operational interface only. No backend or executable bodies supplied."""
from typing import Literal, Mapping, Protocol, TypedDict

Mode = Literal['LIVE', 'FIXTURE_ONLY']

class ProcessIdentity(TypedDict):
    pid: int
    start: int
    uid: int
    cgroup: str

class DeclaredInvocation(TypedDict):
    # Frozen before Common/runner launch. Never contains future PIDs or ACK hashes.
    declaration_sha256: str
    source_manifest_sha256: str
    invocation: str
    phase: str
    artifact_path: str
    artifact_sha256: str
    features: tuple[str, ...]
    argv: tuple[str, ...]
    cwd: str
    public_environment: Mapping[str, str | None]
    opaque_authority_inheritance: Literal['verified-common-only-never-log']
    stdout_path: str
    stderr_path: str
    publication_plan_sha256: str
    publication_stage: str
    profile: Literal['local-regular-exec-write-denial-v1']
    mode: Mode

class DeadlineOrigin(TypedDict):
    # Trusted bootstrap samples before launching physical timeout; ticks identify
    # processes only, never derive CLOCK_MONOTONIC deadlines.
    origin_sha256: str
    emitter: ProcessIdentity
    timeout_identity: ProcessIdentity
    clock_domain: str
    start: float
    soft_end: float
    kill_end: float

class AdmittedInvocation(TypedDict):
    # Constructed AFTER actual runner startup, BEFORE its child. Observed PIDs
    # are checked against declared commands/source/Common ancestry first.
    declaration_sha256: str
    deadline_origin_sha256: str
    runtime_plan_sha256: str
    runner: ProcessIdentity
    common_owner: ProcessIdentity
    common_root: ProcessIdentity  # real Popen child of Common, often dbus-run-session
    clock_domain: str
    start: float
    issued: float
    operation_end: float
    end: float
    cleanup_reserve: float
    public_proof_directory: str  # never exposes Common authority token

class OwnedLaunch:
    """Allocated BEFORE fork; owns startup gate and child registry immediately.

    Survives start_and_hold failure. Never reconstructed from a discovered PID.
    A launch that has not forked is distinguishable from an owned live child.
    """

class HeldEntry:
    """Opaque OS-owned capability, no constructor from a dict/PID or model bool."""
    @property
    def child(self) -> ProcessIdentity: ...

class EntrySeal:
    """Produced from real stopped /proc/PID/exe FD; complete copy+canonical facts."""
    @property
    def facts_sha256(self) -> str: ...
    @property
    def copied_sha256(self) -> str: ...

class ReleasePermit:
    """Bound to declaration/runtime-plan/seal/child/END/mode by the port owner."""
    @property
    def mode(self) -> Mode: ...

class InvocationExit(TypedDict):
    # Runner-level result only; not the external Common closure/global verdict.
    child: ProcessIdentity
    wait_status: int
    returncode: int
    stdout_sha256: str
    stderr_sha256: str
    invocation_facts_sha256: str
    later_images: Literal['UNKNOWN']
    sdk: Literal['NOT_ATTESTED']
    mode: Mode
    global_admission: Literal[False]

class LivePublicationPort(Protocol):
    # Delegates to the sole CP engine. No second publication/copy/SSH loop here.
    def publish_and_wait(self, declared: DeclaredInvocation,
                         admitted: AdmittedInvocation, seal: EntrySeal) -> ReleasePermit: ...

class FixtureOnlyPublicationPort(Protocol):
    # Explicitly NOT a CP ACK/live grant; never accepted by a LIVE invocation.
    def edit_fixture_only(self, declared: DeclaredInvocation,
                          admitted: AdmittedInvocation, seal: EntrySeal) -> ReleasePermit: ...

def admit_runner(declared: DeclaredInvocation) -> AdmittedInvocation: ...
def prepare_owned_launch(declared: DeclaredInvocation, admitted: AdmittedInvocation) -> OwnedLaunch: ...
def start_and_hold(declared: DeclaredInvocation, admitted: AdmittedInvocation,
                   launch: OwnedLaunch) -> HeldEntry: ...
def seal_entry(declared: DeclaredInvocation, admitted: AdmittedInvocation,
               held: HeldEntry) -> EntrySeal: ...
def release_and_wait(declared: DeclaredInvocation, admitted: AdmittedInvocation,
                     held: HeldEntry, seal: EntrySeal, permit: ReleasePermit) -> InvocationExit: ...
# Cleanup is bounded by admitted.end, preserves first cause, signals only owned
# pidfd identities, waits/reaps; UNKNOWN is not success. No raw discovered PID.
def cancel_owned(admitted: AdmittedInvocation, launch: OwnedLaunch,
                 first_cause: BaseException) -> None: ...
# This is called by an OUTSIDE coordinator after Common's own completion. It
# consumes authentic Common facts; it never calls waitpid on a non-child runner.
def collect_common_closure(admitted: AdmittedInvocation,
                           invocation: InvocationExit) -> Mapping[str, object]: ...
