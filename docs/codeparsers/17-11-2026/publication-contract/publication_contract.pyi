"""Spécification proposée uniquement : aucun runtime ni adaptateur livré ici."""
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO, Callable, Literal, Mapping, NewType, Protocol, TypeAlias

SHA256 = NewType("SHA256", str)
FileId = NewType("FileId", str)
ArtifactId = NewType("ArtifactId", str)
StageLabel = NewType("StageLabel", str)
JSONValue: TypeAlias = None | bool | int | float | str | tuple["JSONValue", ...] | Mapping[str, "JSONValue"]

@dataclass(frozen=True)
class CurrentFile:
    file_id: FileId

@dataclass(frozen=True)
class PreviousAckFile:
    stage: StageLabel
    file_id: FileId

ExecutableOrigin: TypeAlias = CurrentFile | PreviousAckFile

@dataclass(frozen=True)
class FileSpec:
    file_id: FileId
    role: Literal["ledger", "data", "executable"]

@dataclass(frozen=True)
class ExecutableSpec:
    artifact_id: ArtifactId
    origin: ExecutableOrigin

@dataclass(frozen=True)
class StageSpec:
    label: StageLabel
    binding_sha256: SHA256
    contract_id: str
    ledger_file: FileId
    files: tuple[FileSpec, ...]
    executables: tuple[ExecutableSpec, ...]
    window_seconds: float | None

@dataclass(frozen=True)
class PublicationPlan:
    protocol_version: int
    execution_id: str
    phase_id: str
    stages: tuple[StageSpec, ...]

@dataclass(frozen=True)
class FileEvidence:
    file_id: FileId
    sha256: SHA256
    size_bytes: int

@dataclass(frozen=True)
class ExecutableEvidence:
    artifact_id: ArtifactId
    origin: ExecutableOrigin
    sha256: SHA256

@dataclass(frozen=True)
class StageHeader:
    execution_id: str
    phase_id: str
    sequence: int
    label: StageLabel
    prepared_sha256: SHA256
    plan_sha256: SHA256
    binding_sha256: SHA256
    window_event_sha256: SHA256 | None

@dataclass(frozen=True)
class PublicationEvidence:
    header: StageHeader
    ledger_file: FileId
    ledger_sha256: SHA256
    ledger_value: JSONValue
    files: tuple[FileEvidence, ...]
    elf_map: tuple[ExecutableEvidence, ...]

@dataclass(frozen=True)
class Snapshot:
    state: Literal["queued", "starting", "active", "finished", "refused"]
    queue_remaining: float | None
    operation_remaining: float | None
    phase_remaining: float | None
    publication_remaining: float | None
    publication: PublicationEvidence | None

@dataclass(frozen=True)
class Limits:
    queue_seconds: float
    hard_soft_seconds: float
    hard_kill_seconds: float
    stop_reserve_seconds: float
    control_call_seconds: float
    copy_call_seconds: float

@dataclass(frozen=True)
class PreparedContext:
    prepared_path: Path
    proofs: Path
    prepared_sha256: SHA256
    plan_sha256: SHA256
    component_hashes: Mapping[str, SHA256]
    limits: Limits

@dataclass(frozen=True)
class Acknowledgement:
    header: StageHeader
    ledger_sha256: SHA256
    files: tuple[FileEvidence, ...]
    elf_map: tuple[ExecutableEvidence, ...]
    peer_host: str
    coordinator_identity_sha256: SHA256
    peer_files: Mapping[FileId, str]

@dataclass(frozen=True)
class CancellationEvidence:
    remote_state: Literal["quiet", "UNKNOWN", "alive"]
    role_observations: tuple[JSONValue, ...]
    scope_evidence: JSONValue
    native_verdict_unchanged: bool

class Transport(Protocol):
    def ready(self) -> None: ...
    def start_phase(self) -> None: ...
    def running(self) -> bool: ...
    def snapshot(self, timeout: float) -> Snapshot: ...
    def fetch(self, stage: StageLabel, file_id: FileId, destination: Path, timeout: float) -> SHA256: ...
    def publish_ack(self, stage: StageLabel, ack: Acknowledgement, timeout: float) -> None: ...
    def finish(self, timeout: float) -> int: ...
    def abort(self, reason: str, timeout: float) -> CancellationEvidence: ...
    def stop(self) -> None: ...

class ActivePhase:
    """Créée et revérifiée exclusivement par PhaseLifecycle commun sur le PC."""
    @property
    def execution_id(self) -> str: ...
    @property
    def phase_id(self) -> str: ...
    @property
    def prepared_sha256(self) -> SHA256: ...

@dataclass(frozen=True)
class PublicationSource:
    evidence: PublicationEvidence
    source_files: Mapping[FileId, Path]

class PublicationProvider(Protocol):
    """Lecture/validation métier seulement ; aucun export, ACK ou appel natif."""
    def read(self, spec: StageSpec, phase: ActivePhase) -> PublicationSource: ...
    def validate(self, spec: StageSpec, source: PublicationSource, phase: ActivePhase) -> None: ...

class PhaseLifecycle:
    """Implémentation commune concrète, pas un callback active=True du fournisseur."""
    def __init__(self, context: PreparedContext, clock: Callable[[], float] = ...) -> None: ...
    def observe_active(self) -> ActivePhase: ...
    def snapshot(self) -> Snapshot: ...
    def cancel(self, reason: str) -> CancellationEvidence: ...

class PublicationGate:
    def __init__(self, plan: PublicationPlan, context: PreparedContext,
                 lifecycle: PhaseLifecycle, provider: PublicationProvider) -> None: ...
    def snapshot(self) -> Snapshot: ...
    def export(self, stage: StageLabel, file_id: FileId, out: BinaryIO) -> None: ...
    def acknowledge(self, stage: StageLabel, ack: Acknowledgement) -> None: ...
    def cancel(self, reason: str) -> CancellationEvidence: ...

class PublicationRelay:
    def __init__(self, plan: PublicationPlan, context: PreparedContext, transport: Transport,
                 clock: Callable[[], float] = ..., sleep: Callable[[float], None] = ...) -> None: ...
    def run(self) -> int: ...

# Les futures fabriques valident/fixent les conteneurs, pas seulement frozen=True.
def validate_plan(plan: PublicationPlan) -> PublicationPlan: ...
def plan_sha256(plan: PublicationPlan) -> SHA256: ...
