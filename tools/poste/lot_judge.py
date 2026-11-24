#!/usr/bin/env python3
"""The lot judge, as the pusher calls it at a deposit (24 Nov. 2026, Lucie: « qu'il sorte un rapport des choses qui
violent les règles »; c7: an opinion, never a refusal; once per diff).

    lot_judge.py <id> <head> <author>

Run from the pusher's worktree (cwd), with PUSHER_DIR the pusher's folder. It reads the lot's diff against its base on
origin/master, generated files left out (tools/poste/generated_files.txt), normalized and bounded; the key of an opinion
is the digest of that diff, of the judge's rules, of its graph and of its manifest: an opinion that exists for the key is
served again without a call (a redeposit, a repose without a change of code). Otherwise, under a declared daily cap, it
calls the backend's tool `lot_judge` through `rag3weaver-backend <manifest>` (JSON lines on stdin), writes the report
next to the pusher's journals, one line in the log and one in the author's inbox. It never fails the deposit.

Environment: PUSHER_DIR; PUSHER_JUDGE_CMD (rag3weaver-backend); PUSHER_JUDGE_MANIFEST (the manifest of master's tree,
extension/rag3weaver/templates/backends/pusher/backend.json); PUSHER_JUDGE_MAX_PER_DAY (100); PUSHER_JUDGE_MAX_CHARS
(240000, about 60 000 tokens).
"""
import datetime
import hashlib
import json
import os
import subprocess
import sys

NOTE = ("an opinion of one draw of a model, not a proof: a lot it says nothing about is not shown clean "
        "(the model varies from one call to the next)")


def git(*a, check=True):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=check).stdout


def show(path):
    return git("show", f"origin/master:{path}", check=False)


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def log(d, lot, event, detail):
    with open(os.path.join(d, "log.tsv"), "a", encoding="utf-8") as f:
        f.write(f"{datetime.datetime.now().strftime('%Y-%m-%dT%H:%M:%S')}\t{lot}\t{event}\t{detail}\n")


def tell(d, author, lot, text):
    os.makedirs(os.path.join(d, "inbox"), exist_ok=True)
    with open(os.path.join(d, "inbox", f"{author}.txt"), "a", encoding="utf-8") as f:
        f.write(f"juge\t{lot}\t{text}\n")


def findings_of(value):
    """The report in the tool's result: the first object that has « findings »."""
    if isinstance(value, dict):
        if isinstance(value.get("findings"), list):
            return value
        for v in value.values():
            r = findings_of(v)
            if r is not None:
                return r
    if isinstance(value, list):
        for v in value:
            r = findings_of(v)
            if r is not None:
                return r
    if isinstance(value, str):
        try:
            return findings_of(json.loads(value))
        except ValueError:
            return None
    return None


def main(lot, head, author):
    d = os.environ["PUSHER_DIR"]
    jd = os.path.join(d, "judge")
    os.makedirs(jd, exist_ok=True)
    base = git("merge-base", "origin/master", head).strip()
    generated = [l.split("\t")[0] for l in show("tools/poste/generated_files.txt").splitlines()
                 if l.strip() and not l.startswith("#") and "\t" in l]
    spec = ["--", "."] + [f":!{p}" for p in generated]
    raw = git("diff", "--no-color", base, head, *spec)
    diff = "\n".join(l for l in raw.splitlines() if not l.startswith("index "))
    if not diff.strip():
        log(d, lot, "juge", "rien à juger (le diff, sans les fichiers générés, est vide)")
        return
    bound = int(os.environ.get("PUSHER_JUDGE_MAX_CHARS", "240000"))
    cut = len(diff) > bound
    if cut:
        diff = diff[:bound]
    subject = git("log", "--reverse", "--format=%s", f"{base}..{head}")
    files = [f for f in git("diff", "--name-only", base, head, *spec).split() if f]
    recent = ""
    for f in files:
        added = [l[1:] for l in git("log", "-p", "--format=", f"{base}~30..{base}", "--", f, check=False).splitlines()
                 if l.startswith("+") and not l.startswith("+++")]
        if added:
            recent += f"### {f}\n" + "\n".join(added[-200:]) + "\n"
    manifest = os.environ.get("PUSHER_JUDGE_MANIFEST") or os.path.abspath(
        "extension/rag3weaver/templates/backends/pusher/backend.json")
    rules_text = show("extension/rag3weaver/templates/pusher/judge_rules.json")
    graph = show("extension/rag3weaver/templates/pusher/judge.mmd")
    try:
        man = open(manifest, encoding="utf-8").read()
    except OSError:
        man = ""
    key = digest("\n".join([digest(diff), digest(rules_text), digest(graph), digest(man)]))[:24]
    report = os.path.join(jd, f"{key}.json")
    link = os.path.join(jd, f"{lot}.json")
    if os.path.exists(report):
        n = len(json.load(open(report, encoding="utf-8")).get("findings", []))
        if os.path.lexists(link):
            os.remove(link)
        os.symlink(report, link)
        log(d, lot, "juge", f"avis repris ({n} constat{'s' if n != 1 else ''}) — {report}")
        tell(d, author, lot, f"avis repris : {n} constat{'s' if n != 1 else ''} — {report}")
        return
    day = datetime.date.today().isoformat()
    calls = os.path.join(jd, f"calls-{day}")
    done = sum(1 for _ in open(calls)) if os.path.exists(calls) else 0
    cap = int(os.environ.get("PUSHER_JUDGE_MAX_PER_DAY", "100"))
    if done >= cap:
        log(d, lot, "juge", f"plafond du jour atteint ({done} appels sur {cap}) : pas d'avis")
        return
    try:
        rules = json.loads(rules_text).get("rules", []) if rules_text.strip() else []
    except ValueError:
        rules = []
    call = {"op": "call", "name": "lot_judge", "arguments": {"lot": {
        "diff": diff, "agents_md": show("AGENTS.md"), "rules": rules, "subject": subject,
        "recent_master_added": recent}}}
    with open(calls, "a") as f:
        f.write(f"{lot}\t{key}\n")
    cmd = os.environ.get("PUSHER_JUDGE_CMD", "rag3weaver-backend")
    try:
        r = subprocess.run([cmd, manifest], input=json.dumps(call) + "\n" + '{"op":"shutdown"}\n',
                           capture_output=True, text=True, timeout=600)
        first = next((l for l in r.stdout.splitlines() if l.strip().startswith("{")), "")
        answer = json.loads(first) if first else {"ok": False, "error": (r.stderr.strip().splitlines() or ["rien"])[-1]}
    except Exception as e:  # noqa: BLE001 — an opinion never fails the deposit
        answer = {"ok": False, "error": f"{type(e).__name__}: {e}"}
    if not answer.get("ok"):
        log(d, lot, "juge", f"pas d'avis : {str(answer.get('error'))[:200]}")
        return
    rep = findings_of(answer.get("result")) or {"findings": [], "summary": ""}
    out = {"lot": lot, "head": head, "base": base, "key": key, "note": NOTE,
           "diff_chars": len(diff), "diff_cut_at": bound if cut else None,
           "findings": rep.get("findings", []), "summary": rep.get("summary", "")}
    with open(report + ".neuf", "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=1)
    os.replace(report + ".neuf", report)
    if os.path.lexists(link):
        os.remove(link)
    os.symlink(report, link)
    n = len(out["findings"])
    said = f"{n} constat{'s' if n != 1 else ''}{' (diff coupé à ' + str(bound) + ' caractères)' if cut else ''}"
    log(d, lot, "juge", f"{said} — {report}")
    tell(d, author, lot, f"{said} — {report} ({NOTE})")


if __name__ == "__main__":
    try:
        main(*sys.argv[1:4])
    except Exception as e:  # noqa: BLE001
        d = os.environ.get("PUSHER_DIR")
        if d and len(sys.argv) > 1:
            log(d, sys.argv[1], "juge", f"pas d'avis : {type(e).__name__}: {str(e)[:200]}")
