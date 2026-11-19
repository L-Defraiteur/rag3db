#!/usr/bin/env python3
"""The housekeeping of the postes (19 Nov 2026, Lucie: « ces instantanés sont marqués tests car ils vous servent à vous,
plutôt que l'inverse »). **Nothing is touched by default.** Only a folder that carries a jetable mark can go: a file
`.jetable.json` inside it, written by the script that created it (snapshot of a bascule, measurement copy, base of a
test, case of a judge):

    {"famille": "instantane-bascule", "par": "G", "cree": "2026-11-19T17:50:14",
     "garder": 3, "age_max_h": 72, "pourquoi": "retour de la bascule vers les versions"}

Within one family of marked folders under one root, the `garder` most recent (3 by default) always stay, whatever their
age; the others go once older than `age_max_h` (72 by default). A control copy whose check has passed is marked
{"garder": 0, "age_max_h": 0}. A folder that a process holds open stays. The age is `cree`, or the folder's mtime.

The build targets are swept apart: --balayer <folder of the targets> <project> runs `cargo sweep --time 2 <project>`
with CARGO_TARGET_DIR on each `target*` of the folder that cargo tagged (CACHEDIR.TAG), skipping a target whose
.cargo-lock a process holds (a build is running). The bytes freed are read on the file system before and after.

Usage: menage.py [--racine <dossier>]… [--balayer <dossier des targets> <projet>] [--a-blanc]
  without --racine: ~/.local/share/rag3weaver/instantanes, ~/.local/share/rag3weaver, ~/.cache/rag3weaver-build/mesures,
  ~/.cache/rag3weaver-build. Each root's direct children are looked at, never deeper. --a-blanc says what would go.
"""
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
import time

MARQUE = ".jetable.json"
RACINES = ["~/.local/share/rag3weaver/instantanes", "~/.local/share/rag3weaver", "~/.cache/rag3weaver-build/mesures",
           "~/.cache/rag3weaver-build"]


def taille(d):
    n = 0
    for base, _, fichiers in os.walk(d):
        for f in fichiers:
            try: n += os.lstat(os.path.join(base, f)).st_blocks * 512
            except OSError: pass
    return n


def tenu(d):
    """A process has a file or its cwd under d (lsof +D, as the windows check it)."""
    r = subprocess.run(["lsof", "-t", "+D", d], capture_output=True, text=True)
    return bool(r.stdout.strip())


def age_h(d, m):
    try: t = dt.datetime.fromisoformat(m["cree"]).timestamp()
    except (KeyError, ValueError, TypeError): t = os.path.getmtime(d)
    return (time.time() - t) / 3600, t


def candidats(racine):
    familles = {}
    try: noms = sorted(os.listdir(racine))
    except OSError: return []
    for nom in noms:
        d = os.path.join(racine, nom)
        if os.path.islink(d) or not os.path.isdir(d): continue
        try: m = json.load(open(os.path.join(d, MARQUE)))
        except (OSError, ValueError): continue
        if not isinstance(m, dict) or not m.get("famille"): continue
        a, t = age_h(d, m)
        familles.setdefault(m["famille"], []).append((t, a, d, m))
    out = []
    for fam, membres in familles.items():
        membres.sort(key=lambda x: -x[0])  # most recent first
        for rang, (t, a, d, m) in enumerate(membres):
            garder = int(m.get("garder", 3)); age_max = float(m.get("age_max_h", 72))
            if rang < garder or a <= age_max and garder > 0: continue
            if garder == 0 and a < age_max: continue
            out.append((fam, rang + 1, a, d))
    return out


def libre(d):
    s = os.statvfs(d); return s.f_bavail * s.f_frsize


def verrou_tenu(target):
    for base, dirs, fichiers in os.walk(target):
        if base.count(os.sep) - target.count(os.sep) > 2: dirs[:] = []; continue
        if ".cargo-lock" in fichiers:
            r = subprocess.run(["flock", "-n", os.path.join(base, ".cargo-lock"), "true"], capture_output=True)
            if r.returncode != 0: return True
    return False


def balayer(dossier, projet, a_blanc):
    avant = libre(dossier)
    for nom in sorted(os.listdir(dossier)):
        t = os.path.join(dossier, nom)
        if not nom.startswith("target") or not os.path.isfile(os.path.join(t, "CACHEDIR.TAG")): continue
        if verrou_tenu(t):
            print(f"target sauté (un bâti le tient) : {t}"); continue
        args = ["cargo", "sweep", "--time", "2", projet] if not a_blanc else ["cargo", "sweep", "--dry-run", "--time", "2", projet]
        r = subprocess.run(args, env=dict(os.environ, CARGO_TARGET_DIR=t), capture_output=True, text=True)
        print(f"balayé : {t} (rc {r.returncode}) {(r.stdout + r.stderr).strip().splitlines()[-1:] or ''}")
    print(f"balayage : rendus {(libre(dossier) - avant) / 2**30:.2f} Gio")


def main():
    args = sys.argv[1:]
    a_blanc = "--a-blanc" in args
    if "--balayer" in args:
        i = args.index("--balayer"); balai = (args[i + 1], args[i + 2]); del args[i:i + 3]
    else:
        balai = None
    racines = [args[i + 1] for i, x in enumerate(args) if x == "--racine"] or [os.path.expanduser(r) for r in RACINES]
    rendus = 0; partis = 0
    print(f"== ménage {time.strftime('%Y-%m-%d %H:%M:%S')}{' (à blanc)' if a_blanc else ''}")
    for racine in racines:
        for fam, rang, a, d in candidats(racine):
            if tenu(d):
                print(f"gardé (tenu par un processus) : {d}"); continue
            n = taille(d)
            print(f"{'partirait' if a_blanc else 'effacé'} : {d} (famille {fam}, rang {rang}, {a:.0f} h, {n / 2**20:.1f} Mio)")
            if not a_blanc:
                shutil.rmtree(d)
                rendus += n; partis += 1
    print(f"{partis} dossier(s) effacé(s) ; rendus : {rendus / 2**20:.1f} Mio ({rendus / 2**30:.2f} Gio)")
    if balai: balayer(balai[0], balai[1], a_blanc)


if __name__ == "__main__":
    main()
