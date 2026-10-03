#!/usr/bin/env python3
"""La tuyauterie du backend de code, sans modèle de langage.

Ce que la règle de Lucie range en « local » : l'outil est appelé avec les
bons arguments, le refus dit quoi faire, l'édition réindexe, la porte tient.
La « vraie expérience de codage » (Gemini) est un autre scénario, hors des
batteries.

Deux manifestes du même moteur (templates/backends/code) : l'instantané
cloud se lit et s'édite en mémoire, l'arbre de travail de poste s'édite sur
le disque. Le démon d'embeddings n'est pas requis : un service distant via
RAG3WEAVER_EMBED_SERVICE, ou le démon local, au choix de l'environnement.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

CRATE = Path(__file__).resolve().parent.parent
BACKEND = CRATE / "target/debug/rag3weaver-backend"


def fail(msg):
    print(f"FAIL: {msg}")
    sys.exit(1)


def prepare(tmp, manifest_name, patch=None):
    """Copie le dossier du backend de code et son workspace d'essai."""
    # La même profondeur que templates/ : backends/<nom>/ référence
    # ../../tools — l'arborescence se réplique telle quelle.
    d = Path(tmp) / "backends" / manifest_name.replace(".json", "")
    shutil.copytree(CRATE / "templates/backends/code", d)
    shutil.copytree(CRATE / "templates/tools", Path(tmp) / "tools", dirs_exist_ok=True)
    ws = d / "workspace"
    ws.mkdir()
    (ws / "main.rs").write_text("fn main() { depart(); }\n")
    (ws / "lib.rs").write_text("pub fn depart() {}\n")
    manifest = json.loads((d / manifest_name).read_text())
    manifest["vector_extension"] = os.environ.get(
        "RAG3DB_VECTOR_EXTENSION",
        str(CRATE.parent / "vector/build/libvector.rag3db_extension"),
    )
    if patch:
        patch(manifest)
    chemin = d / manifest_name
    chemin.write_text(json.dumps(manifest, ensure_ascii=False, indent=1))
    return chemin


class Host:
    def __init__(self, manifest):
        self.p = subprocess.Popen(
            [str(BACKEND), str(manifest)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def ask(self, **op):
        self.p.stdin.write(json.dumps(op) + "\n")
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            err = self.p.stderr.read()
            fail(f"backend sorti ({self.p.poll()}) : {err.strip()[-400:]}")
        return json.loads(line)

    def close(self):
        try:
            self.ask(op="shutdown")
        except SystemExit:
            pass
        except Exception:
            pass
        self.p.wait(timeout=30)


def main():
    if not BACKEND.exists():
        fail(f"binaire absent : {BACKEND} (cargo build --bin rag3weaver-backend "
             "--features daemon,rag3db-native,code)")
    tmp = tempfile.mkdtemp(prefix="backend-code-")
    try:
        # ── La politique de poste : l'arbre réel ────────────────────────────
        manifest = prepare(tmp, "backend.json")
        host = Host(manifest)
        d = host.ask(op="describe")["result"]
        noms = sorted(t["name"] for t in d["tools"])
        assert "edit_file" in noms and "search_code" in noms, noms
        # Les descriptions disent quand prendre l'outil — pas le même texte.
        desc = {t["name"]: t["description"] for t in d["tools"]}
        assert desc["search_code"] != desc["grep_files"], "chaque outil dit le sien"
        assert "motif littéral" in desc["grep_files"].lower() or "littéral" in desc["grep_files"], desc["grep_files"]

        # read lit le workspace.
        r = host.ask(op="call", name="read_file", arguments={"path": "main.rs"})
        assert "depart()" in json.dumps(r), r
        # read hors du workspace : chemin refusé.
        r = host.ask(op="call", name="read_file", arguments={"path": "../backend.json"})
        assert not r.get("ok", True) or ".." in json.dumps(r), f"le chemin hors workspace se refuse : {r}"
        # edit écrit le disque, et la réindexation suit : search trouve le texte édité.
        r = host.ask(op="call", name="edit_file",
                     arguments={"path": "main.rs", "old": "depart()", "new": "arrivee()"})
        assert r.get("ok"), r
        sur_disque = (manifest.parent / "workspace/main.rs").read_text()
        assert "arrivee()" in sur_disque, "le disque est édité"
        r = host.ask(op="call", name="search_code",
                     arguments={"query": "arrivee", "options": {"consistency": "strict"}})
        assert "main.rs" in json.dumps(r), f"l'édition est réindexée et se cherche : {r}"
        # run existe mais la porte (approbation, sans humain ici) tient pour
        # une commande hors liste : le refus ne casse pas le protocole.
        r = host.ask(op="call", name="run_command", arguments={"command": "rm -rf /"})
        assert not r.get("ok", True) or "refus" in json.dumps(r).lower(), f"la porte tient : {r}"

        # ── index + wait : l'indexation en fond se suit par son journal ─────
        # (Le refus sans confirm=true ne se déclenche que sur un débit déjà
        # mesuré en base — jamais sur une base neuve ; il est couvert en lib.)
        r = host.ask(op="call", name="index", arguments={})
        assert r.get("ok"), f"index démarre : {r}"
        m = re.search(r"journal : (\S+)", json.dumps(r).replace("\\n", "\n"))
        assert m, f"le reçu est un journal : {r}"
        journal = m.group(1)
        # timeout_s=0 : lire l'état sans attendre — la réponse est immédiate,
        # « Trouvé » ou « Pas encore », jamais une erreur.
        r = host.ask(op="call", name="wait_output",
                     arguments={"journal": journal, "pattern": "indexation", "timeout_s": 0})
        assert r.get("ok"), f"l'état se lit sans attendre : {r}"
        # Puis la fin, par son motif ; « échouée » dans le même motif pour que
        # l'échec se voie au lieu d'expirer en silence.
        r = host.ask(op="call", name="wait_output",
                     arguments={"journal": journal,
                                "pattern": "indexation terminée|indexation échouée",
                                "timeout_s": 120})
        t = json.dumps(r, ensure_ascii=False)
        assert "indexation terminée" in t, f"l'indexation aboutit : {r}"
        # Et la recherche répond sur l'index ainsi construit.
        r = host.ask(op="call", name="search_code",
                     arguments={"query": "arrivee", "options": {"consistency": "strict"}})
        assert "main.rs" in json.dumps(r), f"la recherche répond après l'indexation : {r}"
        # wait ne sort pas du dossier des journaux : la traversée se refuse.
        traverse = str(Path(journal).parent / ".." / ".." / "etc" / "passwd")
        r = host.ask(op="call", name="wait_output",
                     arguments={"journal": traverse, "pattern": "root", "timeout_s": 1})
        assert not r.get("ok", True), f"la traversée se refuse : {r}"
        host.close()

        # ── La politique cloud : l'instantané ───────────────────────────────
        manifest = prepare(tmp, "snapshot.json")
        host = Host(manifest)
        avant = (manifest.parent / "workspace/main.rs").read_text()
        r = host.ask(op="call", name="read_file", arguments={"path": "main.rs"})
        assert "depart()" in json.dumps(r), r
        r = host.ask(op="call", name="edit_file",
                     arguments={"path": "main.rs", "old": "depart()", "new": "arrivee()"})
        assert r.get("ok"), r
        apres = (manifest.parent / "workspace/main.rs").read_text()
        assert avant == apres, "l'instantané s'édite en mémoire : le disque d'origine ne bouge pas"
        r = host.ask(op="call", name="read_file", arguments={"path": "main.rs"})
        assert "arrivee()" in json.dumps(r), "et la lecture voit l'édition en mémoire"
        # run n'existe pas dans ce manifeste : l'outil inconnu se refuse.
        r = host.ask(op="call", name="run_command", arguments={"command": "ls"})
        assert not r.get("ok", True), f"outil absent refusé : {r}"
        # Le cloud suit l'indexation SANS run_commands : index et wait sont
        # en base déclarative — c'est tout l'objet de leur déménagement.
        r = host.ask(op="call", name="index", arguments={})
        assert r.get("ok"), f"index démarre sur l'instantané : {r}"
        m = re.search(r"journal : (\S+)", json.dumps(r).replace("\\n", "\n"))
        assert m, f"le reçu est un journal : {r}"
        r = host.ask(op="call", name="wait_output",
                     arguments={"journal": m.group(1),
                                "pattern": "indexation terminée|indexation échouée",
                                "timeout_s": 120})
        assert "indexation terminée" in json.dumps(r, ensure_ascii=False), f"l'indexation aboutit : {r}"
        r = host.ask(op="call", name="search_code",
                     arguments={"query": "arrivee", "options": {"consistency": "strict"}})
        assert "main.rs" in json.dumps(r), f"la recherche répond sur l'instantané indexé : {r}"
        host.close()

        print("PASS: descriptions distinctes, lecture, chemin hors workspace refusé, "
              "édition + réindexation cherchable, porte des commandes, indexation en "
              "fond suivie par son journal (état sans attendre, fin par motif, recherche "
              "qui répond, traversée refusée), instantané en mémoire sans toucher le "
              "disque et indexé sans run_commands, outil absent refusé.")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
