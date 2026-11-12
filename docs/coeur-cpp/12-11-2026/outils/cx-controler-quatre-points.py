#!/usr/bin/env python3
"""Quatre points autorisés seulement, 24 processus max, premier échec stop.
Ticket : docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
POINTS = ((16384, 64), (262144, 64), (65536, 8), (65536, 256))
ORDER = ("key-discard", "range-discard", "range-discard", "key-discard", "key-discard", "range-discard")
BINARY_SHA = "0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704"
SOURCE_SHA = "fdc3e90feb364178537c11f2a05743fbb965851e898987288d1f69cb6eca1d08"


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def ensure(condition, detail):
    if not condition:
        raise RuntimeError(detail)


def main(out, unit):
    started = time.monotonic()
    deadline = started + 880
    ensure(not (out / "tentative.json").exists(), "tentative déjà commencée : aucun retry")
    (out / "tentative.json").write_text(json.dumps({"unit": unit, "points": POINTS}) + "\n")
    os.environ["LC_ALL"] = "C"
    os.environ["GIT_OPTIONAL_LOCKS"] = "0"
    os.environ["TMPDIR"] = str(out / "tmp")
    (out / "tmp").mkdir()
    cg = next(s.split(":", 2)[2] for s in Path("/proc/self/cgroup").read_text().splitlines()
              if s.startswith("0::"))
    ensure(unit in cg, "unité non établie")
    cgroup = Path("/sys/fs/cgroup") / cg.lstrip("/")
    limits = {"memory_max": (cgroup / "memory.max").read_text().strip(),
              "swap_max": (cgroup / "memory.swap.max").read_text().strip()}
    ensure(limits == {"memory_max": "1073741824", "swap_max": "0"}, "limites réelles incorrectes")
    ensure(len(Path("/proc/net/route").read_text().splitlines()) == 1, "réseau non isolé")
    ensure(os.statvfs(ROOT).f_flag & os.ST_RDONLY, "binaire/source non montés RO")
    (out / "limites.json").write_text(json.dumps(limits, indent=2) + "\n")
    binary = ROOT / "cx-mesure-retrait"
    ensure(sha(binary) == BINARY_SHA and sha(ROOT / "cx-mesure-retrait.cpp") == SOURCE_SHA, "entrées changées")
    receipt = json.loads((ROOT / "compile-manifest.json").read_text())
    ensure(receipt["binary_sha256"] == BINARY_SHA and receipt["requalification_metadata_only"], "binaire non qualifié")
    ensure(len(receipt["needed_system_libraries"]) == 5 and set(receipt["needed_system_libraries"]) ==
           {"libstdc++.so.6", "libm.so.6", "libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"}, "ELF non qualifié")
    ensure(sha(Path(receipt["loader_canonical"])) == receipt["loader_sha256"] ==
           "2a950d46575efa0210d22440988f5a6b4507ebddc6c86d9a42e6e7ef6f01463e", "chargeur changé")
    initial_refusal = sha(ROOT / "premier-echec.json")
    ensure(initial_refusal == receipt["initial_refusal_preserved_sha256"], "préparation altérée")
    anchor = json.loads((ROOT / "cellule-resultats.json").read_text())
    ensure(anchor["binary_sha256"] == BINARY_SHA, "ancre autre binaire")
    anchor_signature = (anchor["summaries"][0]["input_fnv1a64"], anchor["summaries"][0]["survivors_fnv1a64"])
    all_points = []
    processes = 0
    for point_index, (entries, ranges) in enumerate(POINTS, 1):
        point = {"point": point_index, "entries": entries, "ranges": ranges, "removed": 4096,
                 "summaries": [], "raw_ranges": []}
        expected_signature = None
        for local_index, mode in enumerate(ORDER, 1):
            ensure(time.monotonic() < deadline, "global atteint avant lancement")
            processes += 1
            ensure(processes <= 24, "au-delà des 24 autorisés")
            name = f"point-{point_index}-process-{local_index}"
            argv = [str(binary), "--entries", str(entries), "--ranges", str(ranges),
                    "--remove", "4096", "--pool-mib", "512", "--mode", mode]
            start = time.monotonic()
            with (out / (name + ".stdout")).open("w") as stdout, (out / (name + ".stderr")).open("w") as stderr:
                child = subprocess.Popen(argv, cwd=out, stdout=stdout, stderr=stderr, start_new_session=True)
                try:
                    code = child.wait(timeout=min(57, deadline - time.monotonic()))
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    child.wait(timeout=2)
                    raise RuntimeError(name + " timeout : stop sans retry")
            result = {"argv": argv, "pid": child.pid, "code": code, "elapsed_s": time.monotonic() - start}
            (out / (name + "-result.json")).write_text(json.dumps(result, indent=2) + "\n")
            ensure(code == 0, name + " code " + str(code) + ", stop sans retry")
            rows = [json.loads(line) for line in (out / (name + ".stdout")).read_text().splitlines()]
            raw = [r for r in rows if r.get("kind") == "range"]
            summaries = [r for r in rows if r.get("kind") == "summary"]
            ensure(len(raw) == ranges and len(summaries) == 1 and len(rows) == ranges + 1, "sortie incomplète")
            s = summaries[0]
            ensure(s["validated"] is True and s["mode"] == mode and
                   (s["entries"], s["ranges"], s["removed"]) == (entries, ranges, 4096), "volumes incohérents")
            for u, row in enumerate(raw):
                ensure((row["u"], row["first"], row["count"]) ==
                       (u, entries - 4096 + u * (4096 // ranges), 4096 // ranges), "plage différente")
            ensure(sum(r["ns"] for r in raw) == s["latency_ns"] and sum(r["count"] for r in raw) == 4096,
                   "totaux incohérents")
            signature = (s["input_fnv1a64"], s["survivors_fnv1a64"])
            if expected_signature is None:
                expected_signature = signature
            ensure(signature == expected_signature, "entrées/survivants différents dans cellule")
            if entries == 65536:
                ensure(signature == anchor_signature, "changement des entrées à N constant")
            s.update(pair=(local_index + 1) // 2, process=processes, argv=argv, elapsed_process_s=result["elapsed_s"])
            point["summaries"].append(s)
            point["raw_ranges"].extend(dict(r, process=processes, pair=s["pair"]) for r in raw)
            (out / "progress.json").write_text(json.dumps({"completed_processes": processes,
                                                          "current_point": point_index}) + "\n")
        all_points.append(point)
        (out / f"point-{point_index}-valide.json").write_text(json.dumps(point, indent=2) + "\n")
    ensure(processes == 24 and time.monotonic() < deadline, "périmètre/budget invalide")
    ensure(sha(binary) == BINARY_SHA and sha(ROOT / "cx-mesure-retrait.cpp") == SOURCE_SHA, "entrées finales changées")
    ensure(sha(ROOT / "premier-echec.json") == initial_refusal, "refus initial écrasé")
    result = {"status": "QUATRE_POINTS_TERMINES_STOP_MESURES", "mechanism_only": True,
              "exact_engine_base_vs_fix": False, "full_matrix_run": False, "processes": processes,
              "points": all_points, "binary_sha256": BINARY_SHA, "source_sha256": SOURCE_SHA,
              "limits": limits, "elapsed_global_s": time.monotonic() - started,
              "next_execution_automatic": False}
    (out / "resultats.json").write_text(json.dumps(result, indent=2) + "\n")
    print("QUATRE_POINTS_TERMINES_STOP_MESURES", flush=True)


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--unit", required=True)
    args = p.parse_args()
    out = ROOT / "quatre-points-12nov-01"
    try:
        main(out, args.unit)
    except Exception as error:
        (out / "premier-echec.json").write_text(json.dumps({"error": type(error).__name__, "detail": str(error)},
                                                         ensure_ascii=False) + "\n")
        print("STOP_SANS_RETRY", type(error).__name__, str(error), flush=True)
        raise SystemExit(2)
