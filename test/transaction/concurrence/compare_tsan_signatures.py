#!/usr/bin/env python3
"""La passe ThreadSanitizer du banc de concurrence (forme proposée par la session cœur
C++ à la relecture du 3 octobre).

Elle lance des cas du banc plusieurs fois sous TSan, réunit les signatures des courses
vues — fonction et ligne du premier cadre du moteur, des deux côtés de l'accès — et les
compare à tsan_signatures.txt, dans un seul sens :
- une signature absente de la liste fait échouer la passe ;
- une signature de la liste qui n'est pas revue n'est pas un échec, puisqu'une course
  ne se montre que si les deux accès ont eu lieu sans relation d'ordre.
Le témoin C0 doit rester sans aucun avertissement : sinon c'est la ligne de base qu'il
faut trier d'abord. Le jour où la liste est vide (marche A5 et suivantes), tout
avertissement est rouge.

C7 est lancé et rapporté, jamais comparé. Mesuré le 3 octobre : sur six passes de dix
répétitions, C7 faisait encore apparaître de nouvelles signatures à chaque passe, à la
ligne comme à la fonction (17, puis 4, 3, 0, 1 et 2 de plus) — un mélange aléatoire
explore sans fin. C5 et C6 seuls : cinq signatures, toutes dès la première passe, les
mêmes sur les cinq passes ; une sixième (delete_ contre isSelected) depuis que C5 valide
par événements, la même sur cinq passes. Comparer C7 ferait une passe qui rougit au
hasard.

Usage : compare_tsan_signatures.py <banc> <tsan_signatures.txt> <dossier de travail>
        [--repeat N] [--print]
--print écrit les signatures vues, pour établir ou relire la liste.
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile

WITNESS_FILTER = "*C0_DisjointKeysWitness/Thread_Hot:*C0_DisjointKeysWitness/Thread_Reopen"
RACE_FILTER = ("*C5_DoubleDelete/Thread_Hot:*C5_DoubleDelete/Thread_Reopen:"
               "*C6_*/Thread_Hot:*C6_*/Thread_Reopen")
EXPLORE_FILTER = "*C7_RandomMix/Thread_Hot:*C7_RandomMix/Thread_Reopen"

FRAME = re.compile(r"^\s+#\d+ (.+?) (/\S+/src/(\S+?):(\d+))(?::\d+)?(?: \(|$)")
ACCESS = re.compile(r"^\s+(?:Previous )?(?:[Aa]tomic )?(?:[Rr]ead|[Ww]rite) of size")


def short_function(name):
    """Le nom de la fonction sans ses paramètres ni son type de retour."""
    depth = 0
    for i, c in enumerate(name):
        if c == "(" and depth == 0 and i > 0:
            return name[:i]
        if c == "<":
            depth += 1
        elif c == ">":
            depth -= 1
    return name


def signatures_of(report_text):
    """Une signature par rapport TSan : les deux accès, chacun par son premier cadre
    dans src/, triés pour ne pas dépendre de l'ordre dans lequel TSan les écrit."""
    signatures = set()
    for report in report_text.split("=================="):
        kind = re.search(r"WARNING: ThreadSanitizer: ([a-z -]+)", report)
        if not kind:
            continue
        accesses = []
        current = None
        for line in report.splitlines():
            if ACCESS.match(line):
                current = []
                accesses.append(current)
            elif current is not None and not current:
                frame = FRAME.match(line)
                if frame:
                    current.append(f"{short_function(frame.group(1))}@{frame.group(3)}:"
                                   f"{frame.group(4)}")
        sides = sorted(a[0] for a in accesses if a) or ["<no engine frame>"]
        signatures.add(f"{kind.group(1).strip()}: " + " | ".join(sides))
    return signatures


def run(bench, test_filter, logs, repeat):
    os.makedirs(logs, exist_ok=True)
    env = dict(os.environ)
    env["TSAN_OPTIONS"] = f"halt_on_error=0 exitcode=0 log_path={logs}/tsan"
    env.setdefault("CONCURRENCE_ITERATIONS", "20")
    result = subprocess.run([bench, f"--gtest_filter={test_filter}", f"--gtest_repeat={repeat}"],
                            env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    text = ""
    for name in sorted(os.listdir(logs)):
        with open(os.path.join(logs, name), encoding="utf-8", errors="replace") as f:
            text += f.read()
    return result.returncode, result.stdout, signatures_of(text), text.count("WARNING:")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("bench")
    parser.add_argument("signatures")
    parser.add_argument("workdir")
    parser.add_argument("--repeat", type=int, default=10)
    parser.add_argument("--print", action="store_true")
    args = parser.parse_args()

    with open(args.signatures, encoding="utf-8") as f:
        known = {line.strip() for line in f if line.strip() and not line.startswith("#")}
    os.makedirs(args.workdir, exist_ok=True)
    workdir = tempfile.mkdtemp(prefix="tsan-", dir=args.workdir)
    problems = []

    status, output, witness, count = run(args.bench, WITNESS_FILTER,
                                         os.path.join(workdir, "witness"), args.repeat)
    if count:
        problems.append(f"the witness C0 is not clean: {count} warnings")
        problems += [f"  witness: {s}" for s in sorted(witness)]
    if "FAILED" in output and "[  FAILED  ]" in output:
        problems.append("the witness C0 failed as a test, see " + workdir)

    status, output, seen, count = run(args.bench, RACE_FILTER, os.path.join(workdir, "races"),
                                      args.repeat)
    for signature in sorted(seen - known):
        problems.append(f"unknown race signature: {signature}")

    print(f"tsan pass (C5, C6): {args.repeat} repetitions, {count} warnings, {len(seen)} "
          f"signatures ({len(seen & known)} known, {len(seen - known)} unknown, "
          f"{len(known - seen)} known not seen this time); reports in {workdir}")
    if args.print:
        for signature in sorted(seen):
            print(signature)

    status, output, explored, count = run(args.bench, EXPLORE_FILTER,
                                          os.path.join(workdir, "explore"), args.repeat)
    print(f"tsan exploration (C7, reported, not compared): {count} warnings, "
          f"{len(explored)} signatures, {len(explored - known)} outside the list")
    for signature in sorted(explored - known):
        print(f"  C7 only: {signature}")
    if problems:
        print("\n".join(problems))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
