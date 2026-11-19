import os

from conftest import ShellTest
from test_helper import RAG3DB_ROOT

# A point left alone far from an indexed table, then joined by a cloud of new points, must stay the
# first answer of a search for its own vector (ticket
# docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md,
# 19 Nov. 2026). The vector index is built in parallel; two threads appending to the same neighbour
# list while one of them shrinks it lose edges, and the lone point, which has few, loses its links to
# the route searches take. Measured at luciepc with this recipe, 12 runs per thread count: missed
# 0 times at 1 and 4 threads, 5 at 8, 4 at 24. The test builds at 8 threads: it can only be red on a
# machine with at least 8 cores. Driven through the shell because the same recipe through the C++ API
# in transaction_test never missed (100 rounds), for a reason not yet found.

EXTENSION = os.path.join(
    RAG3DB_ROOT, "extension", "vector", "build", "libvector.rag3db_extension"
)
RUNS = 10
BUILD_THREADS = 8
CLOUD = 150


def recipe() -> list[str]:
    statements = [
        f"LOAD EXTENSION '{EXTENSION}';",
        f"CALL threads={BUILD_THREADS};",
        "CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);",
        (
            "UNWIND range(0, 19999) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, i % 29, i % 31, i % 37, "
            "i % 41, i % 43, i % 47, i % 53, i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]});"
        ),
        "MATCH (n:C) WHERE n.id = 1 SET n.vec = [500, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1];",
        "CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');",
    ]
    for j in range(1, CLOUD + 1):
        statements.append(
            f"MATCH (n:C) WHERE n.id = {2 * j + 1} SET n.vec = [{500 + j}, {j % 7}, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1];"
        )
    statements.append(
        "CALL QUERY_VECTOR_INDEX('C', 'c_index', [500, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1], 1, efs := 64) "
        "RETURN node.id AS first_id;"
    )
    return statements


def first_id(stdout: str) -> str:
    rows = [
        line
        for line in stdout.splitlines()
        if line.startswith("│ ") and line.strip("│ ").strip().isdigit()
    ]
    return rows[-1].strip("│ ").strip() if rows else ""


def test_a_lone_point_is_found_first_once_a_cloud_forms_beside_it(tmp_path) -> None:
    assert os.path.exists(EXTENSION), (
        "[check: setup] build rag3db_vector_extension before this test"
    )
    misses = []
    for run in range(RUNS):
        test = ShellTest().add_argument(str(tmp_path / f"db{run}"))
        for statement in recipe():
            test.statement(statement)
        result = test.run()
        assert "Error" not in result.stdout + result.stderr, (
            "[check: setup] " + (result.stdout + result.stderr)[-400:]
        )
        got = first_id(result.stdout)
        if got != "1":
            misses.append(f"run {run} got {got or 'nothing'}")
    assert not misses, (
        f"[check: lone-point-found-first-after-set-cloud] row 1 not first at its exact vector: {misses}"
    )
