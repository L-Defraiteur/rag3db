"""Fixture runner refusal baseline, published before implementation and tests.

No OS backend is delivered. Importing this module has no protocol side effects.
The pure validator execution_entry.py and its evidence are independent/frozen.
"""
from collections.abc import Mapping


class RunnerRefusal(RuntimeError):
    """Functional refusal; cleanup cannot replace the first cause."""
    def __init__(self, code: str, *, first_cause: BaseException | None = None,
                 cleanup: Mapping[str, object] | None = None) -> None:
        self.code = code
        self.first_cause = first_cause
        self.cleanup = cleanup
        super().__init__(code)


def run_fixture_invocation(request: Mapping[str, object], effects: object) -> Mapping[str, object]:
    """Future effect orchestration. Current baseline always refuses before effects.

    A FIXTURE_ONLY result will never be a CP ACK, global grant or SDK attestation.
    Modeled effect tests cannot qualify a native syscall or opaque OS capability.
    """
    if not isinstance(request, Mapping) or request.get('mode') != 'FIXTURE_ONLY':
        raise RunnerRefusal('unsupported-live')
    raise RunnerRefusal('backend-not-implemented')


def collect_common_closure(*args: object, **kwargs: object) -> Mapping[str, object]:
    """No unknown Common relationship may become a fixture fallback."""
    raise RunnerRefusal('unsupported-common-closure')


def main() -> int:
    """CLI deliberately unavailable until a separately reviewed native backend."""
    raise RunnerRefusal('unsupported-native-backend')


if __name__ == '__main__':
    raise SystemExit(main())
