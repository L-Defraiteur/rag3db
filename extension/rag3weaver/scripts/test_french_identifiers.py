#!/usr/bin/env python3
"""Witness of scripts/french_identifiers.py: the function on single identifiers, then a throwaway repository with one
lot per case, read by the script.

Usage: python3 scripts/test_french_identifiers.py [script] — without it, the crate's own. Exit 0 when every case is green.
"""
import os
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SCRIPT = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "french_identifiers.py"
os.environ.setdefault("FRENCH_IDENTIFIERS_WORDLISTS", str(HERE.parent / "data" / "wordlists"))
failures, count = [], 0


def case(name, ok, detail=""):
    global count
    count += 1
    print(("vert   " if ok else "ROUGE  ") + name + ("" if ok else f" — {detail}"))
    if not ok:
        failures.append(name)


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout.strip()


def lot(repo, files, remove=()):
    """One commit on top of master's base: files {path: content}; returns (base, head)."""
    git(repo, "checkout", "-q", "-f", "master")
    base = git(repo, "rev-parse", "HEAD")
    git(repo, "checkout", "-q", "-B", "lot")
    for path, content in files.items():
        f = repo / path; f.parent.mkdir(parents=True, exist_ok=True); f.write_text(content, encoding="utf-8")
    for path in remove:
        (repo / path).unlink()
    git(repo, "add", "-A"); git(repo, "commit", "-qm", "lot")
    return base, git(repo, "rev-parse", "HEAD")


def report(repo, files, remove=()):
    if not SCRIPT.is_file():
        return None, f"script absent : {SCRIPT}"
    base, head = lot(repo, files, remove)
    r = subprocess.run([sys.executable, str(SCRIPT), str(repo), base, head], capture_output=True, text=True)
    return r, r.stdout + r.stderr


# The function alone, for the product (21 Nov., Lucie: reusable).
if SCRIPT.is_file():
    import importlib.util
    spec = importlib.util.spec_from_file_location("french_identifiers", SCRIPT)
    fi = importlib.util.module_from_spec(spec); spec.loader.exec_module(fi)
    lex = fi.Lexicon.load()
    for ident, want in [("ouvrir_un_dossier", True), ("openFolder", False), ("RacineDuDepot", True), ("écrire", True),
                        ("vue", True), ("vues_diff", True), ("parse_route", False), ("index_state", False), ("table", False)]:
        r = fi.french_reason(ident, lex)
        case(f"french_reason({ident}) {'dit français' if want else 'se tait'}", bool(r) == want, str(r))
else:
    case("le script existe", False, str(SCRIPT))

with tempfile.TemporaryDirectory() as t:
    repo = pathlib.Path(t) / "r"
    repo.mkdir()
    git(repo, "init", "-q", "-b", "master")
    git(repo, "config", "user.email", "luciedefraiteur@gmail.com"); git(repo, "config", "user.name", "t")
    (repo / "src").mkdir()
    (repo / "src" / "lib.rs").write_text("fn ouvrir_un_dossier_ancien() {}\npub fn open() {}\n", encoding="utf-8")
    git(repo, "add", "-A"); git(repo, "commit", "-qm", "base")

    # c7's first witness: a function and a route in French.
    r, out = report(repo, {"src/api.rs": 'pub fn ouvrir_un_dossier(x: u32) -> u32 { x }\n'
                                          'fn routes() { route("/dossiers/x", get(h)); }\n'})
    case("fn ouvrir_un_dossier signalée", r is not None and "ouvrir_un_dossier" in out, out)
    case("route /dossiers/x signalée", r is not None and "/dossiers/x" in out, out)
    case("la sortie se termine par 0 (un avertissement, jamais un refus)", r is not None and r.returncode == 0, out)

    # c7's second witness: French comments only, in every language.
    r, out = report(repo, {"src/lib.rs": "fn ouvrir_un_dossier_ancien() {}\npub fn open() {}\n// ouvrir le dossier racine\n/// fermer la tâche\n",
                           "tools/a.py": "# ouvrir le dossier\nx = 1  # fermer la tâche\n",
                           "tools/b.sh": "#!/bin/bash\n# retirer le dépôt\necho ok  # poser la vue\n",
                           "ui/c.ts": "// ouvrir le dossier\n/* fermer\n * la tâche */\nexport const x = 1;\n"})
    case("des commentaires français seuls : rien de signalé", r is not None and r.returncode == 0 and out.strip() == "", out)

    # Displayed strings and prose are not identifiers.
    r, out = report(repo, {"src/msg.rs": 'fn say() { println!("ouvrir le dossier : la tâche est fermée"); }\n',
                           "tools/d.sh": '#!/bin/bash\necho "le dossier est ouvert"\n',
                           "docs/note.md": "# Ouvrir un dossier\n\nfn ouvrir_un_dossier\n"})
    case("chaînes affichées et prose : rien de signalé", r is not None and out.strip() == "", out)

    # English identifiers stay silent.
    r, out = report(repo, {"src/en.rs": 'pub struct FolderView;\nfn open_folder() {}\nconst ROOT_DIR: &str = "/folders/{id}";\n'})
    case("identifiants anglais : rien de signalé", r is not None and out.strip() == "", out)

    # Types, tests, JSON template keys, serde names, accents, shell functions and subcommands, added files.
    r, out = report(repo, {"src/t.rs": 'pub struct VueDuDepot;\n#[test]\nfn fermer_la_tache() {}\n#[serde(rename = "etat")]\nfield: u8,\n',
                           "templates/x.json": '{\n  "racine": "/a",\n  "kind": "code"\n}\n',
                           "tools/e.py": "def écrire(x):\n    return x\n",
                           "tools/fermer_tout.sh": "#!/bin/bash\nretirer_coffre() {\n  :\n}\ncase $1 in\n  poser) : ;;\n  list) : ;;\nesac\n"})
    for ident in ("VueDuDepot", "fermer_la_tache", "etat", "racine", "écrire", "retirer_coffre", "poser", "fermer_tout"):
        case(f"{ident} signalé", r is not None and ident in out, out)
    case("la clé JSON anglaise « kind » n'est pas signalée", r is not None and "  kind  " not in out, out)
    case("la sous-commande anglaise « list » n'est pas signalée", r is not None and "  list  " not in out, out)
    case("un identifiant accentué dit « accent »", r is not None and "accent" in out, out)

    # An identifier already there (a line moved or edited) is not new.
    r, out = report(repo, {"src/lib.rs": "fn ouvrir_un_dossier_ancien() { let _ = 1; }\npub fn open() {}\n"})
    case("un identifiant déjà présent, ligne modifiée : rien de signalé", r is not None and out.strip() == "", out)

    # Lucie (21 Nov.): text never counts, identifiers written as strings do. A French prompt in a template is not
    # reported; a JSON key « dossier » and a node named in French are.
    r, out = report(repo, {"templates/agent.json": '{\n  "prompt": "Tu ouvres le dossier racine et tu fermes la tâche.",\n'
                                                   '  "dossier": "x",\n  "name": "ouvrir_le_coffre",\n  "description": "Le témoin de la passe"\n}\n',
                           "src/p.rs": 'const PROMPT: &str = "Ouvre le dossier racine";\nfn e() -> String { format!("dossier introuvable : {}", 1) }\n'})
    case("une invite française neuve dans un gabarit : non signalée", r is not None and "Tu ouvres" not in out and "Témoin" not in out and "introuvable" not in out, out)
    case("une clé JSON « dossier » : signalée", r is not None and "  dossier  " in out, out)
    case("un nœud nommé en français (\"name\": \"ouvrir_le_coffre\") : signalé", r is not None and "ouvrir_le_coffre" in out, out)
    case("une constante d'invite au nom anglais (PROMPT) : non signalée", r is not None and "PROMPT" not in out, out)

    # c7 (21 Nov.): an identifier already in the base's tree is not new (arbre-open-command called the existing route
    # /dossiers/ouvrir); only what the lot introduces is reported.
    git(repo, "checkout", "-q", "-f", "master")
    (repo / "src" / "routes.rs").write_text('fn r() { route("/dossiers/ouvrir", get(o)); }\n', encoding="utf-8")
    git(repo, "add", "-A"); git(repo, "commit", "-qm", "route existante")
    r, out = report(repo, {"src/open.rs": 'fn open() { call("/dossiers/ouvrir"); }\nfn close() { route("/dossiers/fermer", get(c)); }\n'})
    case("une route française déjà dans la base, citée par le lot : non signalée", r is not None and "/dossiers/ouvrir" not in out, out)
    case("une route française que le lot crée : signalée", r is not None and "/dossiers/fermer" in out, out)

    # c7 (21 Nov.): the strings of test files are data (test_*, tests/, a Rust file's mod tests); the names of their
    # functions and tests are still judged.
    r, out = report(repo, {"tests/e2e_x.rs": 'fn ouvrir_le_tiroir() { call("/tiroirs/ouvrir"); let k = json!({"tiroir": 1}); }\n',
                           "tools/test_y.py": 'def test_rangement():\n    route("/rangements/fermer")\n',
                           "src/m.rs": 'pub fn open() { route("/coffres/a", get(h)); }\n#[cfg(test)]\nmod tests {\n    fn t() { call("/placards/fermer"); }\n    fn fermer_le_placard() {}\n}\n'})
    for quiet in ("/tiroirs/ouvrir", "  tiroir  ", "/rangements/fermer", "/placards/fermer"):
        case(f"chaîne d'un fichier ou d'un module de test « {quiet.strip()} » : non signalée", r is not None and quiet not in out, out)
    for loud in ("ouvrir_le_tiroir", "test_rangement", "fermer_le_placard", "/coffres/a"):
        case(f"« {loud} » (nom de test, ou chaîne hors du module de test) : signalé", r is not None and loud in out, out)

    # Each identifier once, with its file and line.
    r, out = report(repo, {"src/twice.rs": "fn fermer_x() {}\nfn other() { fermer_x(); }\nmod m { fn fermer_x() {} }\n"})
    case("un identifiant dit une fois, avec fichier:ligne", r is not None and out.count("fermer_x") == 1 and "src/twice.rs:1" in out, out)

print(f"{count - len(failures)}/{count} verts")
sys.exit(1 if failures else 0)
