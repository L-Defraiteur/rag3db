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


if __name__ == "__main__":
    sys.exit("usage: lanceur.py <case.md> …")
