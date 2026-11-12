#!/usr/bin/env python3
"""Périmètre autorisé : une TU puis une seule cellule de trois paires, sans retry.
Ticket : docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md
À appeler dans les unités bornées, sous poste lourd/mesure respectivement.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import time

RECIPE_SHA = "b4d424cae166f90ba5d3c67aa7a0533bc579bcd17a86c1b973e250e0e618b985"
PLAN_SHA = "61e32429d7b06f8fffdc490698e19709e0c6ab9a8a3b9de72382f9a977dcaec7"
MANIFEST_SHA = "0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7"


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(data)
    return h.hexdigest()


def ensure(ok, why):
    if not ok:
        raise RuntimeError(why)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("phase", choices=("compile", "measure"))
    p.add_argument("--unit", required=True)
    args = p.parse_args()
    root = Path(__file__).resolve().parent
    started = time.monotonic()
    deadline = started + (160 if args.phase == "compile" else 880)
    os.environ["GIT_OPTIONAL_LOCKS"] = "0"
    os.environ["LC_ALL"] = "C"
    os.environ["TMPDIR"] = str(root / "tmp")
    (root / "tmp").mkdir(exist_ok=True)

    def write(name, value):
        (root / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")

    def run(command, name, child_limit=None):
        remain = deadline - time.monotonic()
        ensure(remain > 0, "borne globale atteinte")
        # 57 s + au plus 2 s d'attente après SIGKILL : marge sous 60 s/enfant.
        timeout = min(remain, 57 if child_limit is not None else remain)
        before = time.monotonic()
        with (root / (name + ".stdout")).open("w") as stdout, (root / (name + ".stderr")).open("w") as stderr:
            child = subprocess.Popen(command, cwd=root, stdout=stdout, stderr=stderr,
                                     start_new_session=True)
            try:
                rc = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(child.pid, signal.SIGKILL)  # uniquement le groupe créé ici
                except ProcessLookupError:
                    pass
                child.wait(timeout=2)
                write(name + "-result.json", {"timeout": True, "argv": command,
                                               "elapsed_s": time.monotonic() - before})
                raise RuntimeError("premier timeout : arrêt sans retry")
        result = {"argv": command, "pid": child.pid, "code": rc,
                  "elapsed_s": time.monotonic() - before, "timeout": False}
        write(name + "-result.json", result)
        ensure(rc == 0, "premier échec " + name + " code " + str(rc) + " ; arrêt sans retry")
        return result

    cg = next(line.split(":", 2)[2] for line in Path("/proc/self/cgroup").read_text().splitlines()
              if line.startswith("0::"))
    ensure(args.unit in cg, "unité/cgroup non prouvé")
    cgroup = Path("/sys/fs/cgroup") / cg.lstrip("/")
    expected = (8 if args.phase == "compile" else 1) * 1024**3
    limits = {"memory_max": (cgroup / "memory.max").read_text().strip(),
              "swap_max": (cgroup / "memory.swap.max").read_text().strip()}
    ensure(limits == {"memory_max": str(expected), "swap_max": "0"}, "limites effectives incorrectes")
    ensure(len(Path("/proc/net/route").read_text().splitlines()) == 1, "réseau non isolé")
    write(args.phase + "-limites.json", {**limits, "unit": args.unit,
                                          "deadline_soft_s": deadline - started})
    recipe_path = root / "cx-commandes-lien-revue.json"
    plan_path = root / "cx-premiere-cellule-revue.json"
    ensure(sha(recipe_path) == RECIPE_SHA and sha(plan_path) == PLAN_SHA, "recette/plan non revus")
    recipe = json.loads(recipe_path.read_text())
    plan = json.loads(plan_path.read_text())
    ensure(str(root) == recipe["cwd"], "espace pilote incorrect")
    manifest_path = Path(recipe["accepted_manifest"]["path"])
    ensure(sha(manifest_path) == MANIFEST_SHA, "manifeste non attesté")
    manifest = json.loads(manifest_path.read_text())
    package = Path(recipe["required_archive"]).parent
    source = root / "cx-mesure-retrait.cpp"
    ensure(sha(source) == recipe["pilot_source"]["sha256"], "source pilote changé")
    source_checkout = Path(recipe["required_headers"]["third_party_checkout"]).parent
    ensure(subprocess.check_output(["git", "-C", str(source_checkout), "rev-parse", "HEAD"], text=True).strip()
           == manifest["source_commit"], "checkout tiers incorrect")
    ensure(not subprocess.check_output(["git", "-C", str(source_checkout), "status", "--porcelain"], text=True).strip(),
           "checkout tiers non propre")
    for arg in recipe["compile_argv"] + recipe["link_argv"]:
        ensure("rag3db-colonnes" not in arg and "/test/" not in arg, "repli base/test interdit")
    binary = root / "cx-mesure-retrait"
    if args.phase == "compile":
        ensure(not binary.exists() and not (root / "pilote.o").exists(), "espace déjà compilé")
        for rel, info in manifest["files"].items():
            file = package / rel
            ensure(not file.is_symlink() and file.stat().st_size == info["bytes"] and sha(file) == info["sha256"],
                   "entrée du paquet modifiée : " + rel)
        view = root / "generated-build-src/include"
        ensure(not view.exists(), "vue déjà créée")
        for rel, info in recipe["required_staging_not_executed"]["files"].items():
            target = view / Path(rel).relative_to("headers-generes")
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(package / rel, target)
            ensure(sha(target) == info["sha256"], "généré non assorti")
        write("staging-verifie.json", {"package_files_verified": len(manifest["files"]),
                                       "generated_view_files": 2, "source_sha256": sha(source)})
        run(recipe["compile_argv"], "compile")
        run(recipe["link_argv"], "link")
        run(["readelf", "-h", str(binary)], "elf-header")
        run(["readelf", "-d", str(binary)], "elf-dynamic")
        header = (root / "elf-header.stdout").read_text()
        dynamic = (root / "elf-dynamic.stdout").read_text()
        ensure(re.search(r"Class:\s+ELF64", header) is not None, "pas ELF64")
        ensure(re.search(r"Machine:\s+Advanced Micro Devices X86-64", header) is not None, "architecture inattendue")
        needed = re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic)
        ensure(bool(needed) and set(needed) <= {"libstdc++.so.6", "libm.so.6", "libgcc_s.so.1", "libc.so.6"},
               "dépendance ELF non système autorisée")
        ensure("RUNPATH" not in dynamic and "RPATH" not in dynamic, "chemin de charge inattendu")
        write("compile-manifest.json", {
            "status": "COMPILE_LINK_VERIFIES", "recipe_sha256": RECIPE_SHA,
            "manifest_input_sha256": MANIFEST_SHA, "source_sha256": sha(source),
            "binary_sha256": sha(binary), "object_sha256": sha(root / "pilote.o"),
            "compile_argv": recipe["compile_argv"], "link_argv": recipe["link_argv"],
            "configuration_identity": manifest["configuration_identity"],
            "source_commit": manifest["source_commit"], "needed_system_libraries": needed,
            "limits": limits, "elapsed_s": time.monotonic() - started,
            "engine_tus_compiled": 0, "pilot_executed": False})
        print("COMPILE_LINK_VERIFIES", flush=True)
        return

    receipt = json.loads((root / "compile-manifest.json").read_text())
    ensure(receipt["status"] == "COMPILE_LINK_VERIFIES" and sha(binary) == receipt["binary_sha256"],
           "binaire non attesté")
    ensure(receipt["recipe_sha256"] == RECIPE_SHA and receipt["manifest_input_sha256"] == MANIFEST_SHA,
           "lien attesté sur autres entrées")
    ensure(plan["entries"] == 65536 and plan["ranges"] == 64 and plan["removed"] == 4096
           and not plan["full_matrix_automatic"] and len(plan["processes"]) == 6, "périmètre de cellule incorrect")
    order = ["key-discard", "range-discard", "range-discard", "key-discard", "key-discard", "range-discard"]
    ensure([p["argv"][-1] for p in plan["processes"]] == order, "ordre incorrect")
    summaries = []
    all_samples = []
    for i, process in enumerate(plan["processes"]):
        ensure(process["pair"] == i // 2 + 1 and process["argv"][0] == str(binary), "processus hors plan")
        result = run(process["argv"], "cellule-" + str(i + 1), child_limit=60)
        rows = [json.loads(line) for line in (root / ("cellule-" + str(i + 1) + ".stdout")).read_text().splitlines()]
        summary = [r for r in rows if r.get("kind") == "summary"]
        samples = [r for r in rows if r.get("kind") == "range"]
        ensure(len(summary) == 1 and len(samples) == 64, "sortie incomplète")
        summary = summary[0]
        ensure(summary["validated"] is True and summary["mode"] == order[i]
               and (summary["entries"], summary["ranges"], summary["removed"]) == (65536, 64, 4096),
               "validation/volumes incorrects")
        ensure(sum(s["count"] for s in samples) == 4096
               and sum(s["ns"] for s in samples) == summary["latency_ns"], "comptes bruts incohérents")
        for u, sample in enumerate(samples):
            ensure((sample["u"], sample["first"], sample["count"]) == (u, 61440 + 64 * u, 64),
                   "plage réalisée différente")
        if summaries:
            ensure((summary["input_fnv1a64"], summary["survivors_fnv1a64"])
                   == (summaries[0]["input_fnv1a64"], summaries[0]["survivors_fnv1a64"]),
                   "entrées/survivants différents : arrêt")
        summary.update(pair=process["pair"], process=i + 1, process_elapsed_s=result["elapsed_s"])
        summaries.append(summary)
        all_samples.extend(dict(s, pair=process["pair"], process=i + 1) for s in samples)
        write("cellule-progress.json", {"completed_processes": len(summaries), "summaries": summaries})
    ensure(time.monotonic() < deadline, "budget global atteint")
    write("cellule-resultats.json", {"status": "PREMIERE_CELLULE_TERMINEE", "mechanism_only": True,
                                     "exact_engine_base_vs_fix": False, "full_matrix_run": False,
                                     "summaries": summaries, "raw_ranges": all_samples,
                                     "binary_sha256": receipt["binary_sha256"],
                                     "recipe_sha256": RECIPE_SHA, "plan_sha256": PLAN_SHA,
                                     "input_manifest_sha256": MANIFEST_SHA, "limits": limits,
                                     "elapsed_global_s": time.monotonic() - started})
    print("PREMIERE_CELLULE_TERMINEE", flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        root = Path(__file__).resolve().parent
        (root / "premier-echec.json").write_text(json.dumps({"error": type(error).__name__, "detail": str(error)},
                                                           ensure_ascii=False, indent=2) + "\n")
        print("STOP_SANS_RETRY", type(error).__name__, str(error), flush=True)
        raise SystemExit(2)
