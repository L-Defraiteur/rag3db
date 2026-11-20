#!/usr/bin/env python3
"""The launcher of the campaign of past fixes (rag3weaver-archi, 19 Nov 2026; design:
docs/rag3weaver-archi/19-11-2026/04-le-lanceur-de-la-campagne-des-correctifs-passes.md).

Lot 1: the case and the tree.
- read_case(path) -> (block, report): the « lanceur » JSON block and the section « Le signalement,
  tel qu'il était », the only thing Algareth is given;
- refusals(block, report, repo, case_path) -> [reason]: what forbids playing the case;
- build_tree(block, repo, depots, dest): the whole repository at the parent (or its `paths`),
  without any docs/ folder but docs/procedures, each submodule at its recorded commit, the files of
  `witness_files_from`, then a fresh git repository with one commit « base » and no history.
"""
import json
import os
import re
import subprocess
import sys

REPORT_TITLE = "## Le signalement, tel qu'il était"


def git(d, *args, entree=None):
    r = subprocess.run(["git", "-C", d, *args], input=entree, capture_output=True)
    if r.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {r.stderr.decode(errors='replace').strip()}")
    return r.stdout


def read_case(path):
    """The « lanceur » block and the report, quoted as it was (blockquote marks removed)."""
    with open(path) as f:
        texte = f.read()
    m = re.search(r"```json\s*\n(.*?)\n```", texte, re.S)
    if not m:
        raise ValueError(f"{path}: no ```json « lanceur » block")
    block = json.loads(m.group(1))
    # Two shapes: the bare block, or the block wrapped in « lanceur » (lucivy, embarquement).
    if set(block) == {"lanceur"}:
        block = block["lanceur"]
    if REPORT_TITLE not in texte:
        raise ValueError(f"{path}: no section « {REPORT_TITLE[3:]} »")
    section = texte.split(REPORT_TITLE, 1)[1]
    fin = re.search(r"\n## ", section)
    section = section[: fin.start()] if fin else section
    lignes = [re.sub(r"^> ?", "", l) for l in section.strip("\n").splitlines()]
    return block, "\n".join(lignes).strip()


def repo_of(block, default, depots):
    """The repository of the case: the one its block names (`repo`, cloned bare among the
    campaign's own, never a working tree of Lucie's), else the default one."""
    if not block.get("repo"):
        return default
    return bare_clone(depots, block["repo"], block["parent"])


def changed_paths(repo, commit):
    """The files a commit changes."""
    out = git(repo, "diff-tree", "--no-commit-id", "--name-only", "-r", "--root", commit).decode()
    return [l for l in out.splitlines() if l]


def refusals(block, report, repo, case_path):
    """What forbids playing the case: the report leaks our fix (its hash, a path it changes), or
    the case file is in the tree at the parent."""
    raisons = []
    ours = block.get("ours", "")
    if ours and any(ours[:n] in report for n in (40, 12, 9, 7) if len(ours) >= n):
        raisons.append("the report names ours (the hash of our fix)")
    if ours:
        for chemin in changed_paths(repo, ours):
            if chemin in report:
                raisons.append(f"the report names {chemin}, a file ours changes")
    parent = block["parent"]
    if subprocess.run(["git", "-C", repo, "cat-file", "-e", f"{parent}:{case_path}"], capture_output=True).returncode == 0:
        raisons.append(f"the case file {case_path} is in the tree at the parent")
    return raisons


def excluded(chemin):
    """A docs/ folder, wherever it is, but docs/procedures: our session journals and tickets
    often tell the principle of the fix."""
    parts = chemin.split("/")
    for i, p in enumerate(parts):
        if p == "docs" and not (i + 1 < len(parts) and parts[i + 1] == "procedures"):
            return True
    return False


def archive_into(repo, commit, dest, paths=None, prefix=""):
    """`git archive` of `commit` (restricted to `paths`) extracted under dest/prefix, docs/ aside."""
    import io, tarfile
    brut = git(repo, "archive", "--format=tar", commit, *(paths or []))
    with tarfile.open(fileobj=io.BytesIO(brut)) as tar:
        membres = [m for m in tar.getmembers() if not excluded(prefix + m.name)]
        tar.extractall(os.path.join(dest, prefix) if prefix else dest, members=membres, filter="data")


def keep_ignores(repo, commit, dest, paths):
    """A case restricted to `paths` keeps the .gitignore files of the root and of each ancestor
    folder of its paths: without them, what the repository ignores (.pyc, build output) came into
    the diff (the night of 21 Nov 2026)."""
    dossiers = {""}
    for p in paths:
        parties = p.strip("/").split("/")
        for i in range(len(parties)):
            dossiers.add("/".join(parties[:i]))
    for d in sorted(dossiers):
        chemin = f"{d}/.gitignore" if d else ".gitignore"
        if excluded(chemin):
            continue
        try:
            contenu = git(repo, "show", f"{commit}:{chemin}")
        except RuntimeError:
            continue
        cible = os.path.join(dest, chemin)
        os.makedirs(os.path.dirname(cible) or dest, exist_ok=True)
        with open(cible, "wb") as f:
            f.write(contenu)


def submodules(repo, commit):
    """(path, commit, url) of each submodule recorded at `commit`."""
    urls = {}
    try:
        cfg = git(repo, "show", f"{commit}:.gitmodules").decode()
    except RuntimeError:
        return []
    chemin = None
    for l in cfg.splitlines():
        l = l.strip()
        if l.startswith("path"):
            chemin = l.split("=", 1)[1].strip()
        elif l.startswith("url") and chemin:
            urls[chemin] = l.split("=", 1)[1].strip()
    out = []
    for l in git(repo, "ls-tree", "-r", commit).decode().splitlines():
        mode, _type, reste = l.split(" ", 2)
        sha, chemin = reste.split("\t", 1)
        if mode == "160000":
            out.append((chemin, sha, urls.get(chemin)))
    return out


def bare_clone(depots, url, sha):
    """A bare clone of mine (never Lucie's tree), fetched until it holds `sha`."""
    os.makedirs(depots, exist_ok=True)
    nom = re.sub(r"[^A-Za-z0-9._-]", "_", url.rstrip("/").split("/")[-1])
    d = os.path.join(depots, nom if nom.endswith(".git") else nom + ".git")
    if not os.path.isdir(d):
        subprocess.run(["git", "-c", "protocol.file.allow=always", "clone", "-q", "--bare", url, d], check=True, capture_output=True)
    if subprocess.run(["git", "-C", d, "cat-file", "-e", sha + "^{commit}"], capture_output=True).returncode:
        subprocess.run(["git", "-C", d, "-c", "protocol.file.allow=always", "fetch", "-q", "origin", "+refs/heads/*:refs/heads/*"], check=True, capture_output=True)
    return d


def build_tree(block, repo, depots, dest):
    """The tree Algareth starts from: the parent (whole, or `paths`), without docs/ but
    docs/procedures, each submodule at its commit, the files of `witness_files_from`, then a fresh
    repository with one commit « base »."""
    parent = block["parent"]
    os.makedirs(dest, exist_ok=False)
    paths = block.get("paths")
    archive_into(repo, parent, dest, paths)
    if paths:
        keep_ignores(repo, parent, dest, paths)
    for chemin, sha, url in submodules(repo, parent):
        if paths and not any(chemin == p or chemin.startswith(p.rstrip("/") + "/") or p.startswith(chemin + "/") for p in paths):
            continue
        if not url:
            raise RuntimeError(f"submodule {chemin}: no url in .gitmodules at the parent")
        source = url if os.path.isabs(url) or "://" in url or url.startswith("git@") else os.path.normpath(os.path.join(repo, url))
        d = bare_clone(depots, source, sha)
        os.makedirs(os.path.join(dest, chemin), exist_ok=True)
        archive_into(d, sha, dest, None, prefix=chemin.rstrip("/") + "/")
    wff = block.get("witness_files_from")
    if wff:
        for chemin in wff["paths"]:
            cible = os.path.join(dest, chemin)
            os.makedirs(os.path.dirname(cible), exist_ok=True)
            with open(cible, "wb") as f:
                f.write(git(repo, "show", f"{wff['commit']}:{chemin}"))
    ident = ["-c", "user.name=campagne", "-c", "user.email=campagne@localhost", "-c", "commit.gpgsign=false"]
    git(dest, "init", "-q")
    git(dest, "add", "-A", "-f")
    git(dest, *ident, "commit", "-qm", "base")


def run_test(block, dest, target, poste, commande):
    """`commande` from dest/tree, with the case's own CARGO_TARGET_DIR (two trees on one target
    replayed each other's binary, 19 Nov 2026), under `poste lourd` when given."""
    cwd = os.path.join(dest, block.get("tree") or ".")
    env = dict(os.environ, CARGO_TARGET_DIR=target)
    # A case is built once, on its own target: no incremental cache, which is never reused (19 Nov 2026, c7: 20 of
    # the 31 Go of archi's target were incremental/). The environment keeps the last word.
    env.setdefault("CARGO_INCREMENTAL", "0")
    argv = ["sh", "-c", commande]
    if poste:
        argv = [poste, "lourd", *argv]
        env.setdefault("POSTE_MEM_MAX", "32G")
    r = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def failed_tests(sortie):
    """The tests libtest says failed: « test X ... FAILED » lines, and the closing « failures: »
    list (the only form under -q)."""
    noms = set(re.findall(r"^test (\S+) \.\.\. FAILED", sortie, re.M))
    for bloc in re.findall(r"^failures:\n((?:    \S+\n)+)", sortie, re.M):
        noms.update(l.strip() for l in bloc.splitlines() if l.strip())
    return sorted(noms)


def prove_red(block, dest, target, poste):
    """The witness is red at the parent, on its assertion: the test command fails, libtest says
    FAILED for the witness. A compile error, a missing test or a green are not a red."""
    code, sortie = run_test(block, dest, target, poste, block["test"])
    # A witness that is a script (.sh by bash, .py by python3, as the pusher plays them): judged by
    # its exit code; a last line that says VERT is never a red (recherche, case 03).
    if block["witness"].strip().endswith((".sh", ".py")):
        derniere = next((l.strip() for l in reversed(sortie.splitlines()) if l.strip()), "")
        if code == 0 or derniere == "VERT":
            return {"red": False, "why": f"the witness script ends green (code {code}, last line « {derniere} »)", "failed": [], "code": code, "tail": sortie[-2000:]}
        return {"red": True, "why": f"the witness script ends red (code {code})", "failed": [block["witness"].strip()], "code": code, "tail": sortie[-2000:]}
    temoins = block["witness"].split()
    failed = [t for t in temoins if any(n == t or n.endswith("::" + t) or t.endswith("::" + n) for n in failed_tests(sortie))]
    if re.search(r"^error(\[E\d+\])?: ", sortie, re.M) and "test result" not in sortie:
        return {"red": False, "why": "the witness does not compile at the parent", "failed": [], "code": code, "tail": sortie[-2000:]}
    if code == 0:
        return {"red": False, "why": "the witness is green at the parent", "failed": [], "code": code, "tail": sortie[-2000:]}
    if not failed:
        return {"red": False, "why": "the command failed, but no witness is FAILED (absent, or another failure)", "failed": [], "code": code, "tail": sortie[-2000:]}
    return {"red": True, "why": "red on the assertion", "failed": failed, "code": code, "tail": sortie[-2000:]}


def answer_for(command):
    """The launcher answers the teammate's approvals: git is refused (the tree has no history,
    and the history of the real repository is out of the case), the rest is granted."""
    import shlex
    try:
        mots = shlex.split(command)
    except ValueError:
        mots = command.split()
    for mot in mots:
        if os.path.basename(mot) == "git":
            return {"approved": False, "text": "git is not available in this campaign: work on the files, and run the tests."}
    return {"approved": True, "text": "granted"}


def freeze_declarations(repo, rev, dest):
    """The templates of `rev`, frozen for the campaign host, with the command gate in `auto`: no
    human answers in a campaign, and what still asks is answered by `answer_for`."""
    import io, tarfile
    os.makedirs(dest, exist_ok=True)
    brut = git(repo, "archive", "--format=tar", rev, "extension/rag3weaver/templates")
    with tarfile.open(fileobj=io.BytesIO(brut)) as tar:
        tar.extractall(dest, filter="data")
    racine = os.path.join(dest, "extension/rag3weaver")
    manifeste = os.path.join(racine, "templates/backends/code/backend.json")
    with open(manifeste) as f:
        m = json.load(f)
    m.setdefault("workspace", {})["commands"] = "auto"
    # The campaign host has a HOME of its own, where « ~ » is a link: the sandbox gets the real
    # paths (cargo could not read ~/.rustup/settings.toml through the link, case tty 09).
    bac = m["workspace"].get("sandbox") or {}
    if "extra_read" in bac:
        bac["extra_read"] = [os.path.join(os.path.expanduser("~"), c[2:]) if c.startswith("~/") else c for c in bac["extra_read"]]
    # And the real homes of the rust chain: a tree without its own toolchain file falls back on
    # rustup's default, read in RUSTUP_HOME (case lucivy 03).
    if "sandbox" in m["workspace"]:
        env = m["workspace"]["sandbox"].setdefault("env", {})
        env["RUSTUP_HOME"] = os.path.join(os.path.expanduser("~"), ".rustup")
        env["CARGO_HOME"] = os.path.join(os.path.expanduser("~"), ".cargo")
    with open(manifeste, "w") as f:
        json.dump(m, f, indent=2, ensure_ascii=False)
    return racine


def collect_diff(worktree, base):
    """base..the worktree as it is, new files included, through a temporary index: the
    worktree and its index are left untouched."""
    import tempfile
    with tempfile.TemporaryDirectory() as t:
        env = dict(os.environ, GIT_INDEX_FILE=os.path.join(t, "index"))
        def g(*args):
            r = subprocess.run(["git", "-C", worktree, *args], env=env, capture_output=True)
            if r.returncode:
                raise RuntimeError(r.stderr.decode(errors="replace"))
            return r.stdout.decode(errors="replace")
        g("read-tree", "HEAD")
        g("add", "-A")
        return g("diff", "--cached", "--binary", base), g("diff", "--cached", "--stat", base)


def build_trace(runs_root, backend):
    """trace.json for the bench: the agent runs of the task opened on `backend` (one run per
    step), summed — steps, model calls and time, tokens, tool calls and errors, retries — with
    the slowest model calls named, so a slow case says where its time went."""
    import glob
    n = lambda v: int(v) if str(v).lstrip("-").isdigit() else 0
    pas, lents = [], []
    for chemin in sorted(glob.glob(os.path.join(runs_root, "*", "agent-*.json.zst"))):
        r = subprocess.run(["zstd", "-dc", chemin], capture_output=True)
        if r.returncode:
            continue
        d = json.loads(r.stdout)
        if (d.get("tache") or {}).get("parent") != backend:
            continue
        ev = d.get("evenements") or []
        appels = [e for e in ev if e.get("kind") == "LlmCall"]
        fins = [e for e in ev if e.get("kind") == "ToolCallFinished"]
        outils = {}
        for e in fins:
            outils[e.get("tool")] = outils.get(e.get("tool"), 0) + 1
        u = d.get("usage_du_chat") or {}
        un = {"step": d.get("pas"), "run": d.get("run"), "wall_ms": n((d.get("fin") or {}).get("ms")),
              "model_calls": len(appels), "model_ms": sum(n(e.get("ms")) for e in appels),
              "retries": sum(n(e.get("retries")) for e in appels),
              "input_tokens": n(u.get("input_tokens")), "cached_tokens": n(u.get("cached_input_tokens")),
              "output_tokens": n(u.get("output_tokens")), "reasoning_tokens": n(u.get("reasoning_tokens")),
              "tool_calls": outils, "tool_ms": sum(n(e.get("ms")) for e in fins),
              "tool_errors": sum(1 for e in fins if str(e.get("ok")) != "True")}
        pas.append(un)
        for e in appels:
            lents.append({"step": d.get("pas"), "ms": n(e.get("ms")), "prompt_tokens": n(e.get("prompt_tokens")),
                          "cached_tokens": n(e.get("cached_prompt_tokens")), "output_tokens": n(e.get("completion_tokens")),
                          "retries": n(e.get("retries"))})
    pas.sort(key=lambda p: (n(p["step"]), p["run"] or ""))
    somme = lambda k: sum(p[k] for p in pas)
    outils = {}
    for p in pas:
        for k, v in p["tool_calls"].items():
            outils[k] = outils.get(k, 0) + v
    return {"steps": len(pas), "model_calls": somme("model_calls"), "model_ms": somme("model_ms"), "wall_ms": somme("wall_ms"),
            "retries": somme("retries"), "input_tokens": somme("input_tokens"), "cached_tokens": somme("cached_tokens"),
            "output_tokens": somme("output_tokens"), "reasoning_tokens": somme("reasoning_tokens"),
            "tool_calls": outils, "tool_ms": somme("tool_ms"), "tool_errors": somme("tool_errors"),
            "slowest_model_calls": sorted(lents, key=lambda c: -c["ms"])[:10], "per_step": pas}


CAMPAGNE = os.path.expanduser("~/.cache/rag3weaver-build/campagne")


def prepare(case_file, case_path, repo, poste=None, racine=CAMPAGNE):
    """One case, ready for Algareth: refused with its reasons, or a run folder holding the tree,
    the case's target, case.json, request.txt and red.json (the red proved at the parent). The
    target lives under the campaign folder, never in /tmp (RAM on these machines)."""
    import datetime, hashlib, shutil
    block, report = read_case(case_file)
    repo = repo_of(block, repo, os.path.join(racine, "depots"))
    raisons = refusals(block, report, repo, case_path)
    nom = os.path.splitext(os.path.basename(case_file))[0]
    run = os.path.join(racine, nom, datetime.datetime.now().strftime("%Y%m%d-%H%M%S"))
    os.makedirs(run)
    with open(os.path.join(run, "case.json"), "w") as f:
        json.dump(dict(block, case_file=case_path, repo_used=repo, report_sha256=hashlib.sha256(report.encode()).hexdigest()), f, indent=2, ensure_ascii=False)
    if raisons:
        with open(os.path.join(run, "refused.json"), "w") as f:
            json.dump(raisons, f, indent=2, ensure_ascii=False)
        return {"run": run, "refused": raisons}
    with open(os.path.join(run, "request.txt"), "w") as f:
        f.write(report + "\n")
    build_tree(block, repo, os.path.join(racine, "depots"), os.path.join(run, "arbre"))
    rouge = prove_red(block, os.path.join(run, "arbre"), os.path.join(run, "target"), poste)
    with open(os.path.join(run, "red.json"), "w") as f:
        json.dump(rouge, f, indent=2, ensure_ascii=False)
    if not rouge["red"]:
        with open(os.path.join(run, "refused.json"), "w") as f:
            json.dump([rouge["why"]], f, indent=2, ensure_ascii=False)
        return {"run": run, "refused": [rouge["why"]]}
    return {"run": run, "refused": [], "red": rouge["failed"]}


HOTE_SH = """#!/bin/bash
# The campaign host's environment: an instance of its own (HOME of its own), never a service host.
set -a; . "$HOME/.config/rag3weaver/rag3weaver-daemon.env"; set +a
export GOOGLE_CLOUD_LOCATION=global RAG3WEAVER_EXIGER_GEMINI=1
export RAG3WEAVER_POSTE="$HOME/.cache/rag3weaver-build/poste" POSTE_MEM_MAX=32G
export PATH="$HOME/.cargo/bin:$PATH"
export RAG3WEAVER_DAEMON_DIR="{maison}/.local/share/rag3weaver"
export HOME="{maison}"
exec "$@"
"""


class Hote:
    """The campaign host: its own HOME (the rust chain and the model cache linked in, an identity
    « campagne »), its own port, the installed binary."""

    def __init__(self, racine=CAMPAGNE, port=7990, binaire=os.path.expanduser("~/.local/bin/rag3weaver-backend")):
        self.racine, self.port, self.binaire = racine, port, binaire
        self.maison = os.path.join(racine, "maison")
        self.sh = os.path.join(racine, "hote.sh")

    def preparer(self):
        reel = os.path.expanduser("~")
        os.makedirs(os.path.join(self.maison, ".cache"), exist_ok=True)
        for lien, cible in ((".rustup", ".rustup"), (".cargo", ".cargo"), (".cache/rag3weaver", ".cache/rag3weaver")):
            chemin = os.path.join(self.maison, lien)
            if not os.path.lexists(chemin):
                os.symlink(os.path.join(reel, cible), chemin)
        with open(os.path.join(self.maison, ".gitconfig"), "w") as f:
            f.write("[user]\n\tname = campagne\n\temail = campagne@localhost\n[commit]\n\tgpgsign = false\n")
        with open(self.sh, "w") as f:
            f.write(HOTE_SH.replace("{maison}", self.maison))
        os.chmod(self.sh, 0o755)

    def ecoute(self):
        import socket
        with socket.socket() as s:
            s.settimeout(1)
            return s.connect_ex(("127.0.0.1", self.port)) == 0

    def demarrer(self):
        import time
        self.preparer()
        if self.ecoute():
            return
        with open(os.path.join(self.racine, "hote.log"), "ab") as log:
            subprocess.Popen([self.sh, self.binaire, "daemon", "--address", f"127.0.0.1:{self.port}"], stdout=log, stderr=log, start_new_session=True)
        for _ in range(60):
            if self.ecoute():
                return
            time.sleep(1)
        raise RuntimeError("the campaign host does not listen")

    def jeton(self):
        with open(os.path.join(self.maison, ".local/share/rag3weaver/daemon.token")) as f:
            return f.read().strip()

    def http(self, methode, chemin, corps=None):
        import urllib.request
        req = urllib.request.Request(f"http://127.0.0.1:{self.port}{chemin}", method=methode,
                                     data=None if corps is None else json.dumps(corps).encode(),
                                     headers={"Authorization": f"Bearer {self.jeton()}", "content-type": "application/json"})
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.loads(r.read())

    def lancer(self, *argv):
        return subprocess.run([self.sh, *argv], capture_output=True, text=True)


def play(run, repo, hote, rev="origin/master", task="campagne", wall_s=1800):
    # `repo` is rag3weaver's repository: the declarations come from it, whatever the case's
    # repository (lucivy, codeparsers).
    """One prepared case played: the tree registered at the campaign host, the report alone as
    the request, approvals answered by answer_for, a stop at the wall budget, then the output for
    the bench (diff.patch, diffstat.txt, mailbox.jsonl, budget.json). No verdict."""
    import time, shutil
    hote.demarrer()
    rev9 = git(repo, "rev-parse", "--short=9", rev).decode().strip()
    decl = os.path.join(hote.racine, "declarations", rev9)
    racine_decl = os.path.join(decl, "extension/rag3weaver")
    if not os.path.isdir(racine_decl):
        freeze_declarations(repo, rev9, decl)
    arbre = os.path.join(run, "arbre")
    base = git(arbre, "rev-parse", "HEAD").decode().strip()
    nom = "cas-" + os.path.basename(os.path.dirname(run))[:2] + "-" + os.path.basename(run)
    r = hote.lancer(hote.binaire, "code-tree", "add", arbre, "--from", racine_decl, "--as", nom, "--daemon", f"127.0.0.1:{hote.port}")
    if r.returncode:
        raise RuntimeError(f"code-tree add: {r.stdout}{r.stderr}")
    # By its manifest, which carries the unique name: the host lists one repository per folder
    # name, and every case tree is called « arbre ».
    etat = hote.http("GET", "/ide/etat")
    marque = os.path.join(hote.maison, ".config/rag3weaver", nom) + "/"
    backend = next(b["id"] for b in etat["backends"] if b["manifest"].startswith(marque))
    with open(os.path.join(run, "request.txt")) as f:
        demande = f.read().strip()
    t0 = time.time()
    v = hote.http("POST", "/ide/teammate/mailbox", {"backend": backend, "task": task, "kind": "request", "text": demande})
    if not v.get("ok"):
        raise RuntimeError(f"request: {v}")
    boite = os.path.join(hote.maison, ".local/share/rag3weaver/taches", backend, task, "mailbox.jsonl")
    repondues, accords, stopped_by = set(), [], None
    while True:
        lignes = []
        if os.path.exists(boite):
            with open(boite) as f:
                lignes = [json.loads(l) for l in f if l.strip()]
        for l in lignes:
            if l.get("kind") == "answer" and l.get("reply_to"):
                repondues.add(l["reply_to"])
        for l in lignes:
            if l.get("kind") == "approval" and l.get("id") not in repondues:
                rep = answer_for(l.get("command") or "")
                hote.http("POST", "/ide/teammate/mailbox", {"backend": backend, "task": task, "kind": "answer", "reply_to": l["id"], "approved": rep["approved"], "text": rep["text"]})
                repondues.add(l["id"])
                accords.append({"command": l.get("command"), "approved": rep["approved"]})
        if any(l.get("kind") == "ended" for l in lignes):
            break
        if time.time() - t0 > wall_s and stopped_by is None:
            hote.http("POST", "/ide/teammate/mailbox", {"backend": backend, "task": task, "kind": "stop", "text": f"campaign budget reached: {wall_s} s"})
            stopped_by = "wall"
        if stopped_by and time.time() - t0 > wall_s + 120:
            break
        time.sleep(5)
    code = os.path.join(hote.maison, ".local/share/rag3weaver/taches", backend, task, "code")
    diff, stat = collect_diff(code, base)
    with open(os.path.join(run, "diff.patch"), "w") as f:
        f.write(diff)
    with open(os.path.join(run, "diffstat.txt"), "w") as f:
        f.write(stat)
    shutil.copy(boite, os.path.join(run, "mailbox.jsonl"))
    trace = build_trace(os.path.join(hote.maison, ".local/share/rag3weaver/runs"), backend)
    with open(boite) as f:
        trace["ended"] = any(json.loads(l).get("kind") == "ended" for l in f if l.strip())
    trace["stopped_by"] = stopped_by
    with open(os.path.join(run, "trace.json"), "w") as f:
        json.dump(trace, f, indent=2, ensure_ascii=False)
    with open(os.path.join(run, "budget.json"), "w") as f:
        json.dump({"wall_s": wall_s, "elapsed_s": round(time.time() - t0), "stopped_by": stopped_by, "approvals": accords,
                   "backend": backend, "task": task, "declarations": rev9, "base": base}, f, indent=2, ensure_ascii=False)
    return {"run": run, "stopped_by": stopped_by, "elapsed_s": round(time.time() - t0), "diffstat": stat.strip().splitlines()[-1:] }


if __name__ == "__main__":
    import argparse
    a = argparse.ArgumentParser(description="The launcher of the campaign of past fixes, lot 1: prepare a case.")
    a.add_argument("verb", choices=["prepare", "play", "trace"])
    a.add_argument("case_file", help="prepare: the case file (Markdown with its « lanceur » block); play: the prepared run folder")
    a.add_argument("--case-path", help="the case file's path in its repository (refused if present at the parent)")
    a.add_argument("--wall-s", type=int, default=1800)
    a.add_argument("--repo", required=True, help="a clone of the repository that holds the parent (read only)")
    a.add_argument("--poste", default=os.path.expanduser("~/.cache/rag3weaver-build/poste"), help="poste, for the red at the parent under poste lourd")
    a.add_argument("--racine", default=CAMPAGNE)
    o = a.parse_args()
    if o.verb == "prepare":
        print(json.dumps(prepare(o.case_file, o.case_path, o.repo, o.poste if os.path.exists(o.poste) else None, o.racine), ensure_ascii=False))
    elif o.verb == "trace":
        # A trace rebuilt after the fact, for a run played before lot 3.
        with open(os.path.join(o.case_file, "budget.json")) as f:
            b = json.load(f)
        t = build_trace(os.path.join(o.racine, "maison/.local/share/rag3weaver/runs"), b["backend"])
        t["stopped_by"] = b.get("stopped_by")
        with open(os.path.join(o.case_file, "trace.json"), "w") as f:
            json.dump(t, f, indent=2, ensure_ascii=False)
        print(json.dumps({k: t[k] for k in ("steps", "model_calls", "model_ms", "wall_ms", "retries")}))
    else:
        print(json.dumps(play(o.case_file, o.repo, Hote(o.racine), wall_s=o.wall_s), ensure_ascii=False))
