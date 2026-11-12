#!/usr/bin/env python3
"""Configuration/plan/revue/build statique, à exécuter seulement dans le lanceur borné.
Ticket : docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md
Ne crée ni worktree, ni unité systemd, ni namespace ; refuse sans limites effectives.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import time


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    p = argparse.ArgumentParser()
    for flag in ("source", "build", "output", "reference"):
        p.add_argument("--" + flag, required=True, type=Path)
    p.add_argument("--unit", required=True)
    args = p.parse_args()
    source, build, output, reference = (getattr(args, n).resolve()
                                         for n in ("source", "build", "output", "reference"))
    os.environ["GIT_OPTIONAL_LOCKS"] = "0"
    started = time.monotonic()
    deadline = started + 1680  # Le RuntimeMaxSec externe (1740 s) reste le filet dur.
    expected = "0511dece74f94db6937fb03e6ec45f0c58828c5a"

    def write(name, value):
        (output / name).write_text(json.dumps(value, indent=2) + "\n")

    def run(command, log, cwd=None):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("budget global atteint")
        with (output / log).open("w") as stream:
            subprocess.run(command, cwd=cwd, stdout=stream, stderr=subprocess.STDOUT,
                           check=True, timeout=remaining)

    def git(*argv):
        return subprocess.check_output(["git", "-C", str(source), *argv], text=True).strip()

    # Le root doit être ro-bind par bwrap : seules build et output sont écrites.
    cg = next(line.split(":", 2)[2] for line in Path("/proc/self/cgroup").read_text().splitlines()
              if line.startswith("0::"))
    cgroup = Path("/sys/fs/cgroup") / cg.lstrip("/")
    memory = (cgroup / "memory.max").read_text().strip()
    swap = (cgroup / "memory.swap.max").read_text().strip()
    if memory != str(8 * 1024**3) or swap != "0":
        raise RuntimeError("limites mémoire non établies")
    if args.unit not in cg:
        raise RuntimeError("unité du lot non établie")
    fs = os.statvfs(build)
    if fs.f_blocks * fs.f_frsize != 2 * 1024**3:
        raise RuntimeError("scratch non borné à 2 Gio")
    route_lines = Path("/proc/net/route").read_text().splitlines()
    if len(route_lines) > 1:
        raise RuntimeError("namespace réseau non isolé")
    if not (os.statvfs(source).f_flag & os.ST_RDONLY):
        raise RuntimeError("source non montée en lecture seule")
    if not (os.statvfs(reference).f_flag & os.ST_RDONLY):
        raise RuntimeError("build de référence non monté en lecture seule")
    if any(build.iterdir()):
        raise RuntimeError("build non neuf")
    commit = git("rev-parse", expected)
    if git("rev-parse", "HEAD") != commit or git("status", "--porcelain"):
        raise RuntimeError("source incorrecte ou non propre")
    (build / "tmp").mkdir()
    os.environ["TMPDIR"] = str(build / "tmp")
    os.environ["CCACHE_DISABLE"] = "1"
    os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
    options = {
        "CMAKE_BUILD_TYPE": "Release", "BUILD_TESTS": "OFF", "BUILD_EXTENSION_TESTS": "OFF",
        "BUILD_SHELL": "OFF", "BUILD_SINGLE_FILE_HEADER": "OFF", "BUILD_EXTENSIONS:STRING": "",
        "ENABLE_RUNTIME_CHECKS": "OFF", "SINGLE_THREADED": "OFF", "ENABLE_ADDRESS_SANITIZER": "OFF",
        "ENABLE_THREAD_SANITIZER": "OFF", "ENABLE_UBSAN": "OFF", "ENABLE_MALLOC_BUFFER_MANAGER": "OFF",
        "ENABLE_LTO": "OFF", "ENABLE_BACKTRACES": "OFF", "USE_STD_FORMAT": "OFF",
        "RAG3DB_PAGE_SIZE_LOG2:STRING": "12", "RAG3DB_VECTOR_CAPACITY_LOG2:STRING": "11",
        "RAG3DB_NODE_GROUP_SIZE_LOG2:STRING": "17", "RAG3DB_MAX_SEGMENT_SIZE_LOG2:STRING": "18",
        "FETCHCONTENT_FULLY_DISCONNECTED": "ON", "AUTO_UPDATE_GRAMMAR": "OFF",
        "CCACHE_PROGRAM:FILEPATH": "",
    }
    command = ["cmake", "-S", str(source), "-B", str(build), "-G", "Ninja"]
    command += [f"-D{k}={v}" for k, v in options.items()]
    run(command, "configure.log")
    if git("status", "--porcelain"):
        raise RuntimeError("configuration a modifié la source")

    # Comparer tous les objets src/ à leurs commandes de référence, pas seulement un exemple.
    old_commands = json.loads((reference / "compile_commands.json").read_text())
    new_commands = json.loads((build / "compile_commands.json").read_text())
    old_source = reference.parent.parent

    def normalize(item, root, binary):
        file = Path(item["file"])
        if not file.is_relative_to(root / "src"):
            return None
        argv = item.get("arguments") or shlex.split(item["command"])
        clean = []
        i = 0
        while i < len(argv):
            token = argv[i]
            if token in ("-o", "-c", "-MF", "-MT"):
                i += 2
                continue
            if token == "-MD":
                i += 1
                continue
            clean.append(token.replace(str(binary), "<BUILD>").replace(str(root), "<SOURCE>"))
            i += 1
        return str(file.relative_to(root)), clean

    old = dict(v for item in old_commands if (v := normalize(item, old_source, reference)))
    new = dict(v for item in new_commands if (v := normalize(item, source, build)))
    differences = {name: {"reference": old.get(name), "private": new.get(name)}
                   for name in sorted(old.keys() | new.keys()) if old.get(name) != new.get(name)}
    # Les versions consignées par CMake doivent aussi coïncider.
    def compiler_info(directory):
        files = sorted((directory / "CMakeFiles").glob("*/CMakeCXXCompiler.cmake"))
        if not files:
            raise RuntimeError("provenance compilateur CMake absente")
        fields = ("CMAKE_CXX_COMPILER", "CMAKE_CXX_COMPILER_ID", "CMAKE_CXX_COMPILER_VERSION",
                  "CMAKE_CXX_SIZEOF_DATA_PTR", "CMAKE_CXX_COMPILER_ABI")
        records = []
        for file in files:
            text = file.read_text()
            result = {}
            for field in fields:
                match = re.search(r"set\(" + field + r'\s+"?([^"\)\n]+)', text)
                if not match:
                    raise RuntimeError("champ compilateur absent : " + field)
                result[field] = match.group(1)
            records.append(result)
        if any(record != records[0] for record in records[1:]):
            raise RuntimeError("descripteurs compilateur divergents ; arrêt")
        return records[0]

    compilers = {"reference": compiler_info(reference), "private": compiler_info(build)}
    run(["ninja", "-C", str(build), "-n", "rag3db"], "plan-sec.log")
    plan = (output / "plan-sec.log").read_text()
    forbidden = ("Linking CXX shared library", "Downloading", "Performing download step", "git clone",
                 "curl ", "wget ", "test/transaction", "Building CXX object extension/")
    incompatible = [part for part in forbidden if part in plan]
    review = {
        "source_commit": commit, "source_clean": True, "options": options,
        "runner_sha256": digest(Path(__file__)), "configure_argv": command,
        "memory_max": memory, "swap_max": swap, "scratch_bytes": fs.f_blocks * fs.f_frsize,
        "compiler": compilers, "engine_commands": len(new), "differences": differences,
        "reference_compile_commands_sha256": digest(reference / "compile_commands.json"),
        "forbidden_plan_entries": incompatible, "plan_sha256": digest(output / "plan-sec.log"),
        "compile_commands_sha256": digest(build / "compile_commands.json"),
        "configuration_elapsed_s": time.monotonic() - started,
        "gate": "ATTENTE_REVUE" if not differences and not incompatible and
                compilers["reference"] == compilers["private"] else "STOP",
    }
    write("revue.json", review)
    shutil.copyfile(build / "compile_commands.json", output / "compile_commands.json")
    shutil.copyfile(build / "CMakeCache.txt", output / "CMakeCache.txt")
    if review["gate"] != "ATTENTE_REVUE":
        raise RuntimeError("comparaison ou plan incompatible ; aucun build")
    print("CONFIGURE_PLAN_PRETS_REVOIR", flush=True)
    gate_path = output / "autoriser-build.json"
    while not gate_path.exists():
        if time.monotonic() >= deadline:
            raise RuntimeError("budget global atteint en attente de revue ; aucun build")
        time.sleep(1)
    gate = json.loads(gate_path.read_text())
    if gate != {"plan_sha256": review["plan_sha256"], "source_commit": commit}:
        raise RuntimeError("gate de revue invalide ; aucun build")
    run(["cmake", "--build", str(build), "--target", "rag3db", "-j", "8"], "build.log")
    if git("status", "--porcelain"):
        raise RuntimeError("source non propre après build")
    archive = build / "src/librag3db.a"
    if not archive.is_file():
        raise RuntimeError("archive statique absente")
    package = output / "archive-assortie-0511dece7"
    package.mkdir()
    shutil.copyfile(archive, package / "librag3db.a")
    shutil.copytree(source / "src/include", package / "headers-source", symlinks=True)
    shutil.copytree(build / "src/include", package / "headers-generes", symlinks=True)
    for library in sorted((build / "third_party").rglob("*.a")):
        target = package / library.relative_to(build)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(library, target)
    files = {}
    for file in sorted(package.rglob("*")):
        if file.is_symlink():
            raise RuntimeError("lien dans le paquet à examiner avant toute consommation")
        if file.is_file():
            files[str(file.relative_to(package))] = {"bytes": file.stat().st_size, "sha256": digest(file)}
    if time.monotonic() >= deadline:
        raise RuntimeError("budget global atteint pendant la sauvegarde")
    write("manifest.json", {"status": "ARCHIVE_PRIVEE_PRETE_A_REVOIR", "source_commit": commit,
                            "source_clean": True, "review": review, "files": files,
                            "elapsed_config_build_s": time.monotonic() - started,
                            "no_tests_pilot_measure_install": True})
    print("ARCHIVE_PRIVEE_PRETE_A_REVOIR", flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print("STOP", type(error).__name__, str(error), flush=True)
        raise SystemExit(2)
