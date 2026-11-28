#!/usr/bin/env python3
"""The judge's bench (24 Nov. 2026, c7: « le banc des huit lots reste comme témoin rejouable du juge, avec les verdicts à
la main, pour toute consigne future »).

    python3 tools/poste/judge_bench.py [--ref <git ref of the judge>] [--draws N] [--model <ref>]… [--out <dir>]

For each lot of tools/poste/judge_bench_verdicts.json, the judge's context as its graph builds it (AGENTS.md, the judge's
rules, the lot's announced subject, its diff without generated files, the lines master added recently in its files), its
instruction and its schema read from templates/pusher/judge.mmd at --ref; one call per draw and per model to Vertex
(OpenAI-compatible endpoint, global), with no temperature nor reasoning imposed. Each finding is sorted by the hand
verdicts: true, false, debatable, or to judge (a person then adds it to the verdicts). When the graph has KeepQuoted,
the bench mirrors it (a finding whose excerpt is not, as it is, in an added line — a removed one for
reverted_master_change — is dropped) and says how many true and false findings it dropped. It prints, per model, the true
and false findings of each draw and their median. It calls a paid model: a measure, never a pass's witness.
Credentials: GOOGLE_APPLICATION_CREDENTIALS and GOOGLE_CLOUD_PROJECT, from the environment or from
~/.config/rag3weaver/rag3weaver-daemon.env; the access token is minted with openssl.
"""
import argparse, base64, json, os, re, statistics, subprocess, sys, tempfile, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))


def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout


def param(mmd, name):
    m = re.search(r"^%% param: " + name + r" (?:string|json) = (.*)$", mmd, re.M)
    return json.JSONDecoder().raw_decode(m.group(1).strip())[0]


def credentials():
    env = dict(os.environ)
    f = os.path.expanduser("~/.config/rag3weaver/rag3weaver-daemon.env")
    if os.path.exists(f):
        for l in open(f).read().splitlines():
            if "=" in l and not l.startswith("#"):
                k, v = l.split("=", 1); env.setdefault(k, v)
    return env["GOOGLE_APPLICATION_CREDENTIALS"], env["GOOGLE_CLOUD_PROJECT"]


def token(sa_path):
    sa = json.load(open(sa_path))
    b64 = lambda b: base64.urlsafe_b64encode(b).rstrip(b"=")
    now = int(time.time())
    head = b64(json.dumps({"alg": "RS256", "typ": "JWT"}).encode())
    claims = b64(json.dumps({"iss": sa["client_email"], "scope": "https://www.googleapis.com/auth/cloud-platform",
                             "aud": sa["token_uri"], "iat": now, "exp": now + 3600}).encode())
    with tempfile.NamedTemporaryFile("w", delete=False) as k:
        os.chmod(k.name, 0o600); k.write(sa["private_key"])
    try:
        sig = subprocess.run(["openssl", "dgst", "-sha256", "-sign", k.name], input=head + b"." + claims,
                             capture_output=True, check=True).stdout
    finally:
        os.remove(k.name)
    jwt = (head + b"." + claims + b"." + b64(sig)).decode()
    body = f"grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={jwt}".encode()
    return json.load(urllib.request.urlopen(urllib.request.Request(sa["token_uri"], body)))["access_token"]


def quoted(diff):
    """The added and the removed lines of a diff, as KeepQuoted reads them."""
    added = [l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++")]
    removed = [l[1:] for l in diff.splitlines() if l.startswith("-") and not l.startswith("---")]
    return added, removed


def kept(finding, added, removed):
    """A mirror of the graph's KeepQuoted (24 Nov. 2026): a finding is kept when its excerpt is, as it is, in an added
    line of the diff, or in a removed line for reverted_master_change."""
    e = (finding.get("excerpt") or "").strip()
    lines = removed if finding.get("rule") == "reverted_master_change" else added
    return bool(e) and any(e in l for l in lines)


def context(lot, ref, rules, agents, generated):
    base = lot["base"] or git("merge-base", "origin/master", lot["tip"]).strip()
    spec = ["--", "."] + [f":!{p}" for p in generated]
    diff = "\n".join(l for l in git("diff", "--no-color", base, lot["tip"], *spec).splitlines() if not l.startswith("index "))
    subject = git("log", "--reverse", "--format=%s", f"{base}..{lot['tip']}")
    recent = ""
    for f in [x for x in git("diff", "--name-only", base, lot["tip"], *spec).split() if x]:
        added = [l[1:] for l in git("log", "-p", "--format=", f"{base}~30..{base}", "--", f).splitlines()
                 if l.startswith("+") and not l.startswith("+++")]
        if added:
            recent += f"### {f}\n" + "\n".join(added[-200:]) + "\n"
    lot["_quoted"] = quoted(diff)
    return (f"## AGENTS.md, the rules of the repository\n\n{agents}\n\n## The rules of the judge\n\n"
            f"{json.dumps(rules, ensure_ascii=False, indent=1)}\n\n## The announced subject of the lot\n\n{subject}\n\n"
            f"## The diff of the lot\n\n{diff}\n\n## Lines master added recently in these files\n\n{recent}")


def matches(finding, entry):
    if entry.get("rule", "*") not in ("*", finding.get("rule")):
        return False
    if "file" in entry and not (finding.get("file") or "").endswith(entry["file"]):
        return False
    return entry.get("excerpt", "") in (finding.get("excerpt") or "")


def sort(finding, lot):
    for kind in ("true", "false", "debatable"):
        if any(matches(finding, e) for e in lot.get(kind, [])):
            return kind
    return "to judge"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="origin/master"); ap.add_argument("--draws", type=int, default=3)
    ap.add_argument("--model", action="append"); ap.add_argument("--out", default=tempfile.mkdtemp(prefix="judge-bench-"))
    a = ap.parse_args()
    models = a.model or ["google/gemini-3.1-flash-lite"]
    verdicts = json.load(open(os.path.join(HERE, "judge_bench_verdicts.json")))["lots"]
    mmd = git("show", f"{a.ref}:extension/rag3weaver/templates/pusher/judge.mmd")
    instruction, schema = param(mmd, "instruction"), param(mmd, "schema")
    rules = json.loads(git("show", f"{a.ref}:extension/rag3weaver/templates/pusher/judge_rules.json"))["rules"]
    agents = git("show", "origin/master:AGENTS.md")
    generated = [l.split("\t")[0] for l in git("show", "origin/master:tools/poste/generated_files.txt").splitlines()
                 if "\t" in l and not l.startswith("#")]
    sa, project = credentials(); tok = token(sa)
    url = f"https://aiplatform.googleapis.com/v1/projects/{project}/locations/global/endpoints/openapi/chat/completions"
    contexts = {lot["id"]: context(lot, a.ref, rules, agents, generated) for lot in verdicts}
    keep = "KeepQuoted" in mmd
    os.makedirs(a.out, exist_ok=True)
    for model in models:
        tallies = []
        for draw in range(1, a.draws + 1):
            t = {"true": 0, "false": 0, "debatable": 0, "to judge": 0, "dropped true": 0, "dropped false": 0}
            for lot in verdicts:
                body = {"model": model, "max_tokens": 16384,
                        "messages": [{"role": "system", "content": instruction}, {"role": "user", "content": contexts[lot["id"]]}],
                        "response_format": {"type": "json_schema", "json_schema": {"name": "report", "schema": schema}}}
                req = urllib.request.Request(url, json.dumps(body).encode(),
                                             {"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
                try:
                    found = json.loads(json.load(urllib.request.urlopen(req, timeout=300))["choices"][0]["message"]["content"]).get("findings", [])
                except Exception as e:  # noqa: BLE001
                    print(f"  {lot['id']}: pas de réponse ({str(e)[:120]})"); found = []
                for f in found:
                    k = sort(f, lot)
                    if keep and not kept(f, *lot["_quoted"]):
                        if k in ("true", "false"):
                            t[f"dropped {k}"] += 1
                        continue
                    t[k] += 1
                    if k == "to judge":
                        print(f"  à juger — {model.split('/')[-1]} tirage {draw} {lot['id']} : {f.get('rule')} {f.get('file')}:{f.get('line')} « {(f.get('excerpt') or '')[:80]} »")
                with open(os.path.join(a.out, f"{model.split('/')[-1]}-{draw}-{lot['id']}.json"), "w") as o:
                    json.dump(found, o, ensure_ascii=False, indent=1)
            tallies.append(t)
            print(f"{model.split('/')[-1]} tirage {draw} : {t['true']} vrais, {t['false']} faux, {t['debatable']} discutables, {t['to judge']} à juger"
                  + (f" ; ôtés par KeepQuoted : {t['dropped true']} vrais, {t['dropped false']} faux" if keep else ""))
        print(f"{model.split('/')[-1]} médiane : {statistics.median(x['true'] for x in tallies)} vrais, "
              f"{statistics.median(x['false'] for x in tallies)} faux — rapports dans {a.out}")


if __name__ == "__main__":
    sys.exit(main())
