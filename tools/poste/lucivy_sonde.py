#!/usr/bin/env python3
"""The night probe of the lucivy dictionaries (20 November 2026, after the lost words of rag3weaver's code index).

For each full-text directory of this poste (one `<base>.rag3db.fts` per line in the bases file), each `Lucivy_*` index
is copied by reflink under `~/.cache/rag3weaver-build/mesures/lucivy-sonde-<stamp>/` (marked jetable: the housekeeping
keeps the three most recent), then checked by the `lucivy-sonde` binary (`check_dictionary`: ids cited by a live
segment with no text in any generation). A copy taken while the host writes may not open: it is taken again, three
times, two minutes apart.

The verdict, one line at the end of `~/.cache/rag3weaver-build/lucivy-sonde/journal.log`:
  - ROUGE: a field has more lost ids than the known losses (`lucivy_sonde_connus.txt`, words lost before 4.3.12 that
    only a rebuild gives back);
  - INCOMPLET: an index never opened, or the bases file names nothing;
  - VERT otherwise. Ids in reprieve are said, never red: they are known and read.

Usage: lucivy_sonde.py [--bases <file>] [--connus <file>] [--bin <lucivy-sonde>] [--racine <folder of the copies>]
       [--journal <file>] [--pause <seconds>]
"""
import argparse
import datetime as dt
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

HOME = pathlib.Path.home()
CACHE = HOME / ".cache/rag3weaver-build"


def known_losses(path):
    """`<base> <index> <field> <count>` per line; `#` starts a comment."""
    known = {}
    if path and pathlib.Path(path).exists():
        for line in pathlib.Path(path).read_text().splitlines():
            line = line.split("#", 1)[0].strip()
            if line:
                base, index, field, count = line.split()
                known[(base, index, field)] = int(count)
    return known


def check(binary, copy):
    out = subprocess.run([binary, str(copy)], capture_output=True, text=True)
    return [json.loads(l) for l in out.stdout.splitlines() if l.strip().startswith("{")]


def main(argv=None):
    p = argparse.ArgumentParser()
    p.add_argument("--bases", default=str(CACHE / "lucivy-sonde/bases.txt"))
    p.add_argument("--connus", default=str(pathlib.Path(__file__).with_name("lucivy_sonde_connus.txt")))
    p.add_argument("--bin", default=str(CACHE / "lucivy-sonde/bin/lucivy-sonde"))
    p.add_argument("--racine", default=str(CACHE / "mesures"))
    p.add_argument("--journal", default=str(CACHE / "lucivy-sonde/journal.log"))
    p.add_argument("--pause", type=float, default=120.0)
    a = p.parse_args(argv)

    now = dt.datetime.now()
    stamp = now.strftime("%Y%m%d-%H%M%S")
    dest = pathlib.Path(a.racine) / f"lucivy-sonde-{stamp}"
    dest.mkdir(parents=True, exist_ok=True)
    (dest / ".jetable.json").write_text(json.dumps({
        "famille": "lucivy-sonde", "par": "lucivy", "cree": now.isoformat(timespec="seconds"),
        "garder": 3, "age_max_h": 72, "pourquoi": "copies de la sonde nocturne des dictionnaires lucivy",
    }))
    known = known_losses(a.connus)
    bases = []
    if pathlib.Path(a.bases).exists():
        bases = [l.strip() for l in pathlib.Path(a.bases).read_text().splitlines() if l.strip() and not l.startswith("#")]
    lines, red, incomplete = [f"== {now:%Y-%m-%d %H:%M:%S} sonde des dictionnaires"], False, not bases
    if not bases:
        lines.append(f"aucune base dans {a.bases}")
    for fts in bases:
        fts = pathlib.Path(os.path.expanduser(fts))
        base = fts.name.removesuffix(".rag3db.fts")
        for index in sorted(fts.glob("Lucivy_*")):
            copy = dest / base / index.name
            rows = []
            for attempt in range(3):
                shutil.rmtree(copy, ignore_errors=True)
                copy.parent.mkdir(parents=True, exist_ok=True)
                subprocess.run(["cp", "-a", "--reflink=auto", str(index), str(copy)], check=True)
                rows = check(a.bin, copy)
                if rows and not any("error" in r for r in rows):
                    break
                if attempt < 2:
                    time.sleep(a.pause)
            if not rows or any("error" in r for r in rows):
                incomplete = True
                err = next((r["error"] for r in rows if "error" in r), "aucune ligne")
                lines.append(f"INCOHÉRENTE {base} {index.name} : {err}")
                continue
            for r in rows:
                k = known.get((base, index.name, r["field"]), 0)
                if r["unknown"] > k:
                    red = True
                    lines.append(f"ROUGE {base} {index.name} {r['field']} : {r['unknown']} ids perdus (connus : {k}), sur {r['referenced']} cités")
                elif r["unknown"]:
                    lines.append(f"connu {base} {index.name} {r['field']} : {r['unknown']} perdus d'avant (rendus au rebâti)")
                if r["in_reprieve"]:
                    lines.append(f"sursis {base} {index.name} {r['field']} : {r['in_reprieve']} ids cités tiennent par le sursis")
    verdict = "ROUGE" if red else ("INCOMPLET" if incomplete else "VERT")
    lines.append(f"{verdict} sonde des dictionnaires {now:%Y-%m-%d %H:%M}")
    journal = pathlib.Path(a.journal)
    journal.parent.mkdir(parents=True, exist_ok=True)
    with journal.open("a") as f:
        f.write("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0 if verdict == "VERT" else 1


if __name__ == "__main__":
    sys.exit(main())
