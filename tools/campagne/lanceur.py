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
    if REPORT_TITLE not in texte:
        raise ValueError(f"{path}: no section « {REPORT_TITLE[3:]} »")
    section = texte.split(REPORT_TITLE, 1)[1]
    fin = re.search(r"\n## ", section)
    section = section[: fin.start()] if fin else section
    lignes = [re.sub(r"^> ?", "", l) for l in section.strip("\n").splitlines()]
    return block, "\n".join(lignes).strip()


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


CAMPAGNE = os.path.expanduser("~/.cache/rag3weaver-build/campagne")


def prepare(case_file, case_path, repo, poste=None, racine=CAMPAGNE):
    """One case, ready for Algareth: refused with its reasons, or a run folder holding the tree,
    the case's target, case.json, request.txt and red.json (the red proved at the parent). The
    target lives under the campaign folder, never in /tmp (RAM on these machines)."""
    import datetime, hashlib, shutil
    block, report = read_case(case_file)
    raisons = refusals(block, report, repo, case_path)
    nom = os.path.splitext(os.path.basename(case_file))[0]
    run = os.path.join(racine, nom, datetime.datetime.now().strftime("%Y%m%d-%H%M%S"))
    os.makedirs(run)
    with open(os.path.join(run, "case.json"), "w") as f:
        json.dump(dict(block, case_file=case_path, report_sha256=hashlib.sha256(report.encode()).hexdigest()), f, indent=2, ensure_ascii=False)
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


if __name__ == "__main__":
    import argparse
    a = argparse.ArgumentParser(description="The launcher of the campaign of past fixes, lot 1: prepare a case.")
    a.add_argument("verb", choices=["prepare"])
    a.add_argument("case_file", help="the case file (Markdown with its « lanceur » block)")
    a.add_argument("--case-path", required=True, help="the case file's path in its repository (refused if present at the parent)")
    a.add_argument("--repo", required=True, help="a clone of the repository that holds the parent (read only)")
    a.add_argument("--poste", default=os.path.expanduser("~/.cache/rag3weaver-build/poste"), help="poste, for the red at the parent under poste lourd")
    a.add_argument("--racine", default=CAMPAGNE)
    o = a.parse_args()
    print(json.dumps(prepare(o.case_file, o.case_path, o.repo, o.poste if os.path.exists(o.poste) else None, o.racine), ensure_ascii=False))
