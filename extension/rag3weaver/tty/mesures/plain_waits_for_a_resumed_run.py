#!/usr/bin/env python3
"""**`--plain --demande` waits for a run the host resumes after its end** (ticket
`docs/tty/tickets/2026-11-16-le-mode-plain-et-la-fin-d-un-pas-relance.md`, 18 nov. 2026).

The host writes `ended` once, at the end of the teammate's loop; but a supervisor line that came in during the last
step makes it start the teammate again right after (the restart of `start_teammate`), with the task's state back to
`running`. The plain mode stopped at the first `ended` and missed the answer of the resumed run. On the fake host:
1. launch `rag3weaver-tty --plain --depot faux --tache mesure --demande go --journal …`;
2. once the scripted step has run, `POST /faux/end-then-resume`: an `ended`, then 3 s of a running task, a report,
   and the final `ended`;
3. require the journal to carry that report, and the terminal to exit with code 0 within 15 s.

    plain_waits_for_a_resumed_run.py <binary> <faux_hote.py> <work dir>

Says VERT or ROUGE on its last line. Standard library only.
"""
import json
import os
import subprocess
import sys
import time
import urllib.request

BINARY, FAKE, DIR = sys.argv[1:4]
PORT = 17996
os.makedirs(DIR, exist_ok=True)
DJ, JOURNAL, BOX = os.path.join(DIR, "daemon.json"), os.path.join(DIR, "plain.journal"), os.path.join(DIR, "fake.jsonl")
for f in (DJ, JOURNAL, BOX):
    if os.path.exists(f):
        os.remove(f)
reds = []
ANSWER = "la réponse de la reprise, 18 nov."


def post(path, body):
    r = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", data=json.dumps(body).encode(), headers={"Authorization": "Bearer jeton-de-mesure", "Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(r, timeout=5))


fake = subprocess.Popen([sys.executable, FAKE, str(PORT), BOX, DJ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(50):
        if os.path.exists(DJ) and os.path.getsize(DJ) > 0:
            break
        time.sleep(0.1)
    tty = subprocess.Popen([BINARY, "--plain", "--depot", "faux", "--tache", "mesure", "--demande", "go", "--daemon-json", DJ, "--journal", JOURNAL], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    time.sleep(6)
    post("/faux/end-then-resume", {"text": ANSWER})
    try:
        code = tty.wait(timeout=15)
        if code != 0:
            reds.append(f"it exits with code {code}")
    except subprocess.TimeoutExpired:
        reds.append("the resumed run ended, and it does not give the hand back within 15 s")
        tty.kill()
    journal = open(JOURNAL).read() if os.path.exists(JOURNAL) else ""
    if ANSWER not in journal:
        reds.append("the journal does not carry the answer of the resumed run (it stopped at the first end)")
finally:
    fake.terminate()
for r in reds:
    print("ROUGE", r)
print("ROUGE" if reds else "VERT")
