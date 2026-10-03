#!/usr/bin/env python3
"""rag3weaver-chat when its backend must reopen its base (exit 75).

Real Rust chat host, local SSE provider, fixture backend — no database, no
embedding model. The backend answers a tool call with mustReopen and exits
with 75, as rag3weaver-backend does after a failed checkpoint. The chat
restarts it once and says so in the tool result, without replaying the call;
if the restarted backend fails the same way, it is not restarted again.
"""
import http.server
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/debug/rag3weaver-chat"


class Provider(http.server.BaseHTTPRequestHandler):
    """Each user turn calls `search` once, then answers with the tool result."""
    def log_message(self, *args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        last = body["messages"][-1]
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()

        def chunk(delta, finish=None):
            self.wfile.write(("data: " + json.dumps({"choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}) + "\n\n").encode())
            self.wfile.flush()

        if last["role"] == "user":
            chunk({"tool_calls": [{"index": 0, "id": "call_search", "type": "function",
                                   "function": {"name": "search", "arguments": json.dumps({"query": "q"})}}]})
            chunk({}, "tool_calls")
        else:
            self.server.tool_results.append(last["content"])
            chunk({"content": "vu"})
            chunk({}, "stop")
        self.wfile.write(b"data: [DONE]\n\n")


BACKEND = '''import sys,json
from pathlib import Path
launches=Path(%r); n=int(launches.read_text() or 0)+1 if launches.exists() else 1; launches.write_text(str(n))
fail_all=%r
for line in sys.stdin:
    r=json.loads(line)
    if r['op']=='describe':
        print(json.dumps({'ok':True,'result':{'tools':[{'name':'search','description':'Fixture','inputSchema':{'type':'object','properties':{'query':{'type':'string'}}}}]}}),flush=True)
    elif r['op']=='shutdown':
        print(json.dumps({'ok':True,'result':{'closed':True}}),flush=True); break
    elif n==1 or fail_all:
        print(json.dumps({'ok':False,'error':'la base doit être rouverte : A checkpoint of this database failed, so it must be closed and reopened before it is used again','mustReopen':True}),flush=True)
        sys.exit(75)
    else:
        print(json.dumps({'ok':True,'result':{'presentation':'Résultat après relance','metadata':{}}}),flush=True)
'''


def run(fail_all):
    """Two chat turns; the backend's first launch fails, or every launch."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        provider = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Provider)
        provider.tool_results = []
        threading.Thread(target=provider.serve_forever, daemon=True).start()
        launches = tmp / "launches"
        backend = tmp / "backend.py"
        backend.write_text(BACKEND % (str(launches), fail_all))
        config = tmp / "chat.json"
        config.write_text(json.dumps({"name": "Fixture", "system_prompt": "Use tools.",
                                      "llm": {"base_url": f"http://127.0.0.1:{provider.server_port}/v1", "model": "fixture-model"},
                                      "state_dir": "state", "backend_command": [sys.executable, str(backend)],
                                      "allowed_tools": ["search"]}))
        chat = subprocess.Popen([str(BINARY), str(config)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, text=True, cwd=tmp)

        def turn(message):
            chat.stdin.write(json.dumps({"op": "chat", "session": "s", "message": message}) + "\n")
            chat.stdin.flush()
            for line in chat.stdout:
                event = json.loads(line)
                if event.get("event") == "done":
                    return event

        turn("premier")
        turn("second")
        chat.stdin.write(json.dumps({"op": "shutdown"}) + "\n")
        chat.stdin.flush()
        chat.stdin.close()
        chat.wait(timeout=30)
        stderr = chat.stderr.read()
        provider.shutdown()
        provider.server_close()
        return int(launches.read_text()), provider.tool_results, stderr


# Fails once: restarted once, said, the call is not replayed; the next turn works.
launches, results, stderr = run(fail_all=False)
assert launches == 2, launches
assert "relancé sur la base rouverte" in results[0] and "pas rejoué" in results[0], results
assert "Résultat après relance" in results[1], results
assert "le backend est relancé (une fois)" in stderr, stderr

# Fails every time: one restart only, then the failure is named and left alone.
launches, results, stderr = run(fail_all=True)
assert launches == 2, f"une seule relance, pas une boucle : {launches}"
assert "relancé sur la base rouverte" in results[0], results
assert "n'est pas relancé une seconde fois" in results[1], results
print("PASS: rag3weaver-chat restarts a backend that must reopen its base once, says so, never replays the call, and does not loop.")
