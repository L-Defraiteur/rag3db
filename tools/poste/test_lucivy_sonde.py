#!/usr/bin/env python3
"""Witness of tools/poste/lucivy_sonde.py: a fake lucivy-sonde binary answers chosen lines, per index name."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SONDE = HERE / "lucivy_sonde.py"
failures = []
count = 0


def case(name, ok, detail=""):
    global count
    count += 1
    print(("OK    " if ok else "ÉCHEC ") + name + ("" if ok else f" — {detail}"))
    if not ok:
        failures.append(name)


def run(t, answers, connus="", bases=None):
    """answers: index name → list of JSON rows (or a list of lists, one per attempt)."""
    fts = t / "code-x.rag3db.fts"
    for name in answers:
        (fts / name).mkdir(parents=True, exist_ok=True)
        (fts / name / "meta.json").write_text("{}")
    state = t / "state.json"
    state.write_text(json.dumps({k: 0 for k in answers}))
    fake = t / "fake"
    fake.write_text(f"""#!/usr/bin/env python3
import json, pathlib, sys
answers = {json.dumps(answers)}
state = pathlib.Path({str(state)!r}); seen = json.loads(state.read_text())
name = pathlib.Path(sys.argv[1]).name; a = answers[name]
attempt = seen[name]; seen[name] += 1; state.write_text(json.dumps(seen))
rows = a[min(attempt, len(a) - 1)] if a and isinstance(a[0], list) else a
for r in rows: print(json.dumps(dict(index=name, **r)))
""")
    fake.chmod(0o755)
    (t / "bases.txt").write_text("\n".join(bases if bases is not None else [str(fts)]) + "\n")
    (t / "connus.txt").write_text(connus)
    out = subprocess.run([sys.executable, str(SONDE), "--bases", str(t / "bases.txt"), "--connus", str(t / "connus.txt"),
                          "--bin", str(fake), "--racine", str(t / "mesures"), "--journal", str(t / "journal.log"),
                          "--pause", "0"], capture_output=True, text=True)
    return out.returncode, (t / "journal.log").read_text() if (t / "journal.log").exists() else "", state


def row(field, unknown, in_reprieve=0, referenced=100):
    return {"field": field, "referenced": referenced, "unknown": unknown, "in_reprieve": in_reprieve}


with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, _ = run(t, {"Lucivy_Scope": [row("content", 532), row("name", 0)]}, "code-x Lucivy_Scope content 532\n")
    case("une perte connue reste verte", rc == 0 and "VERT sonde" in j and "connu code-x Lucivy_Scope content : 532" in j, j)
    marks = list((t / "mesures").glob("lucivy-sonde-*/.jetable.json"))
    case("la copie est marquée jetable (famille lucivy-sonde)", len(marks) == 1 and json.loads(marks[0].read_text())["famille"] == "lucivy-sonde", str(marks))

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, _ = run(t, {"Lucivy_Scope": [row("content", 533)], "Lucivy_File": [row("path", 0)]}, "code-x Lucivy_Scope content 532\n")
    case("une perte neuve est rouge, nommée", rc == 1 and "ROUGE code-x Lucivy_Scope content : 533 ids perdus (connus : 532)" in j and j.rstrip().splitlines()[-1].startswith("ROUGE"), j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, state = run(t, {"Lucivy_Scope": [[{"error": "meta.json: missing file"}], [row("content", 0)]]})
    case("un index qui n'ouvre pas est repris, puis jugé", rc == 0 and "VERT sonde" in j and json.loads(state.read_text())["Lucivy_Scope"] == 2, j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, state = run(t, {"Lucivy_Scope": [{"error": "no meta"}]})
    case("trois échecs d'ouverture : INCOMPLET, jamais vert", rc == 1 and "INCOHÉRENTE code-x Lucivy_Scope : no meta" in j and "INCOMPLET sonde" in j and json.loads(state.read_text())["Lucivy_Scope"] == 3, j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, _ = run(t, {"Lucivy_Scope": [row("content", 0, in_reprieve=4)]})
    case("le sursis est dit, pas rouge", rc == 0 and "sursis code-x Lucivy_Scope content : 4" in j and "VERT sonde" in j, j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, _ = run(t, {"Lucivy_Scope": [row("content", 0)]}, bases=[])
    case("aucune base : INCOMPLET", rc == 1 and "INCOMPLET sonde" in j, j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    (t / "vide.rag3db.fts").mkdir()
    rc, j, _ = run(t, {}, bases=[str(t / "vide.rag3db.fts")])
    case("une base sans index Lucivy_* : INCOMPLET, jamais vert", rc == 1 and "aucun index Lucivy_* vu" in j and "INCOMPLET sonde" in j, j)

with tempfile.TemporaryDirectory() as d:
    t = pathlib.Path(d)
    rc, j, _ = run(t, {"Lucivy_Scope": [row("content", 0, referenced=40), row("name", 0, referenced=2)]})
    case("chaque index vu se dit, avec ses comptes", rc == 0 and "vu code-x Lucivy_Scope : 2 champs, 42 ids cités, 0 perdus, 0 en sursis" in j and "1 index vus" in j, j)

print(f"{count - len(failures)}/{count}")
sys.exit(1 if failures else 0)
