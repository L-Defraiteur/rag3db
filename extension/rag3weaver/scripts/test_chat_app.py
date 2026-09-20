#!/usr/bin/env python3
"""Real Rust agent + local SSE provider + backend protocol + both UI transports.
No external provider, database, embedding model or API expense.
"""
import http.server
import json
import os
from pathlib import Path
import pty
import select
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.error
import urllib.request

from chat_app import Bridge, LocalServer, ROOT, request

BINARY = ROOT / "target/debug/rag3weaver-chat"


class Provider(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.server.requests.append(body)
        assert self.path == "/v1/chat/completions"
        assert body["model"] == "fixture-model"
        names = {t["function"]["name"] for t in body["tools"]}
        assert names == {"search", "save_artifact"}, names
        messages = body["messages"]
        last = messages[-1]
        user = next(m["content"] for m in reversed(messages) if m["role"] == "user")
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()

        def chunk(delta, finish=None):
            self.wfile.write(("data: " + json.dumps({"choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}) + "\n\n").encode())
            self.wfile.flush()

        try:
            if user == "cancel":
                chunk({"content": "Début "})
                self.server.started.set()
                self.server.release.wait(10)
                chunk({"content": "Suite "})
                chunk({}, "stop")
            else:
                if last["role"] == "user":
                    name, args = "hidden", {}
                elif last["name"] == "hidden":
                    assert "error" in last["content"]
                    name, args = "search", {"query": "résurrection"}
                elif last["name"] == "search":
                    assert "Résultat vérifié" in last["content"]
                    assert '"total":1' in last["content"]
                    assert "DO_NOT_DUPLICATE_RAW" not in last["content"]
                    name, args = "save_artifact", {"name": "essai.txt", "content": "Résultat vérifié\n"}
                else:
                    assert last["name"] == "save_artifact"
                    chunk({"content": "Export enregistré : essai.txt"})
                    chunk({}, "stop")
                    self.wfile.write(b"data: [DONE]\n\n")
                    return
                chunk({"tool_calls": [{"index": 0, "id": "call_" + name, "type": "function", "function": {"name": name, "arguments": json.dumps(args)}}]})
                chunk({}, "tool_calls")
            self.wfile.write(b"data: [DONE]\n\n")
        except (BrokenPipeError, ConnectionResetError):
            pass


class ChatAppTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = Path(self.tmp.name)
        self.provider = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Provider)
        self.provider.requests = []
        self.provider.started = threading.Event()
        self.provider.release = threading.Event()
        threading.Thread(target=self.provider.serve_forever, daemon=True).start()
        backend = self.path / "backend.py"
        backend.write_text('''import sys,json
for line in sys.stdin:
    r=json.loads(line)
    if r['op']=='describe':
        result={'tools':[{'name':n,'description':'Fixture','inputSchema':{'type':'object','properties':{'query':{'type':'string'}}}} for n in ['search','hidden']]}
    elif r['op']=='shutdown':
        print(json.dumps({'ok':True,'result':{'closed':True}}),flush=True)
        break
    else:
        assert r['name']=='search', 'disabled tool must never reach backend'
        assert r['arguments']['query']=='résurrection'
        result={'presentation':'Résultat vérifié','metadata':{'total':1},'result':'DO_NOT_DUPLICATE_RAW'}
    print(json.dumps({'ok':True,'result':result}),flush=True)
''')
        self.config = self.path / "chat.json"
        self.config.write_text(json.dumps({"name": "Fixture atelier", "system_prompt": "Use tools.", "llm": {"base_url": f"http://127.0.0.1:{self.provider.server_port}/v1", "model": "fixture-model"}, "state_dir": "state", "backend_command": [sys.executable, str(backend)], "allowed_tools": ["search"]}))
        self.bridge = Bridge([str(BINARY), str(self.config)])
        self.server = LocalServer(0, self.bridge, "test-token")
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def tearDown(self):
        self.provider.release.set()
        self.server.shutdown()
        self.server.server_close()
        self.bridge.close()
        self.provider.shutdown()
        self.provider.server_close()
        self.tmp.cleanup()

    def api(self, path, value=None):
        return request(self.server.origin, "test-token", path, value)

    def test_tools_export_history_auth_and_restart(self):
        with urllib.request.urlopen(self.server.origin + "/") as page:
            self.assertIn(b'aria-label="Conversation"', page.read())
            self.assertIn("frame-ancestors 'none'", page.headers["Content-Security-Policy"])
        with self.assertRaises(urllib.error.HTTPError) as error:
            urllib.request.urlopen(self.server.origin + "/api/info")
        self.assertEqual(error.exception.code, 401)
        error.exception.close()
        cross_origin = urllib.request.Request(self.server.origin + "/api/info", headers={"Authorization": "Bearer test-token", "Origin": "https://untrusted.example"})
        with self.assertRaises(urllib.error.HTTPError) as error:
            urllib.request.urlopen(cross_origin)
        self.assertEqual(error.exception.code, 403)
        error.exception.close()
        with self.api("/api/chat", {"session": "one", "message": "Exporte le résultat"}) as stream:
            events = [json.loads(line) for line in stream]
        self.assertTrue(events[-1]["ok"], events)
        self.assertEqual(events[-1]["result"]["tool_calls"], 3)
        self.assertEqual(events[-1]["result"]["tool_errors"], 1)
        self.assertEqual([e["name"] for e in events if e["event"] == "tool_start"], ["hidden", "search", "save_artifact"])
        with self.api("/api/artifacts/essai.txt") as response:
            self.assertEqual(response.read().decode(), "Résultat vérifié\n")
        with self.api("/api/history?session=one") as response:
            turns = json.load(response)["turns"]
        self.assertFalse(any(t["role"] == "system" for t in turns))
        self.assertTrue(any(t["tool_call_id"] == "call_search" for t in turns))
        self.bridge.close()
        self.bridge = Bridge([str(BINARY), str(self.config)])
        self.server.bridge = self.bridge
        with self.api("/api/history?session=one") as response:
            self.assertEqual(json.load(response)["turns"], turns)
        with self.api("/api/sessions") as response:
            self.assertEqual(json.load(response)["sessions"][0]["id"], "one")
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.api("/api/chat", {"session": "../oops", "message": "no"})
        error.exception.close()
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.api("/api/artifacts/..%2Fchat.json")
        error.exception.close()

    def test_cancel_is_scoped_and_busy_does_not_corrupt_protocol(self):
        result = []
        def run():
            with self.api("/api/chat", {"session": "slow", "message": "cancel"}) as stream:
                result.extend(json.loads(line) for line in stream)
        thread = threading.Thread(target=run)
        thread.start()
        self.assertTrue(self.provider.started.wait(10))
        with self.api("/api/cancel", {"session": "other"}) as response:
            self.assertFalse(json.load(response)["requested"])
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.api("/api/chat", {"session": "other", "message": "hi"})
        self.assertEqual(error.exception.code, 409)
        error.exception.close()
        with self.api("/api/cancel", {"session": "slow"}) as response:
            self.assertTrue(json.load(response)["requested"])
        time.sleep(.05)
        self.provider.release.set()
        thread.join(10)
        self.assertFalse(thread.is_alive())
        self.assertTrue(result[-1]["ok"], result)
        self.assertEqual(result[-1]["result"]["stop"], "Cancelled")
        self.assertEqual(self.bridge.request({"op": "history", "session": "slow"})["turns"][-1]["role"], "assistant")

    def test_tui_pty_and_web_are_available_together(self):
        master, slave = pty.openpty()
        env = dict(os.environ, TERM="xterm-256color")
        process = subprocess.Popen([sys.executable, str(ROOT / "scripts/chat_app.py"), str(self.config), "--demo", "--port", "0"], stdin=slave, stdout=slave, stderr=slave, env=env)
        os.close(slave)
        output = b""
        try:
            deadline = time.monotonic() + 15
            while b"Fixture atelier" not in output and time.monotonic() < deadline:
                if select.select([master], [], [], .2)[0]:
                    output += os.read(master, 65536)
            self.assertIn(b"Chat : http://127.0.0.1:", output)
            self.assertIn(b"Ctrl-Q", output)
            # Wait for curses init, send a prompt, observe Rust demo output in terminal.
            os.write(master, "Bonjour\n".encode())
            while b"partagent" not in output and time.monotonic() < deadline:
                if select.select([master], [], [], .2)[0]:
                    output += os.read(master, 65536)
            self.assertIn(b"partagent", output)
            os.write(master, b"\x11")
            self.assertEqual(process.wait(timeout=10), 0, output.decode(errors="replace"))
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
            os.close(master)

    @unittest.skipUnless(shutil.which("chromedriver") and shutil.which("chromium"), "optional Chromium UI smoke test")
    def test_browser_stream_tools_and_reload(self):
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        driver = subprocess.Popen(["chromedriver", f"--port={port}"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        base = f"http://127.0.0.1:{port}"
        def wd(path, data=None, method=None):
            req = urllib.request.Request(base + path, data=None if data is None else json.dumps(data).encode(), headers={"Content-Type": "application/json"}, method=method)
            with urllib.request.urlopen(req, timeout=20) as response:
                return json.load(response)["value"]
        sid = None
        try:
            deadline = time.monotonic() + 10
            while True:
                try:
                    wd("/status")
                    break
                except urllib.error.URLError:
                    if time.monotonic() > deadline:
                        raise
                    time.sleep(.1)
            result = wd("/session", {"capabilities": {"alwaysMatch": {"browserName": "chrome", "goog:chromeOptions": {"binary": shutil.which("chromium"), "args": ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", "--window-size=1280,900", f"--user-data-dir={self.path / 'browser'}"]}}}})
            sid = result["sessionId"]
            endpoint = f"/session/{sid}"
            wd(endpoint + "/url", {"url": self.server.origin + "/#token=test-token"})
            def js(script):
                return wd(endpoint + "/execute/sync", {"script": script, "args": []})
            deadline = time.monotonic() + 10
            while js("return document.getElementById('name').textContent") != "Fixture atelier":
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.1)
            js("document.getElementById('message').value='Exporte le résultat'; document.getElementById('composer').requestSubmit();")
            while js("return document.getElementById('send').disabled"):
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.1)
            text = js("return document.getElementById('messages').textContent")
            self.assertIn("Export enregistré", text)
            self.assertIn("Résultat vérifié", text)
            self.assertEqual(js("return document.querySelectorAll('#messages details').length"), 3)
            self.assertEqual(js("return location.hash"), "")
            # Reload proves the browser consumes persisted tool calls, not just streamed text.
            wd(endpoint + "/refresh", {})
            deadline = time.monotonic() + 10
            while "Export enregistré" not in js("return document.getElementById('messages').textContent"):
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.1)
            # History and sidebar are fetched independently after reload.
            while "essai.txt" not in js("return document.getElementById('artifacts').textContent"):
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.1)
            import base64
            Path("/tmp/rag3weaver-chat-web.png").write_bytes(base64.b64decode(wd(endpoint + "/screenshot")))
        finally:
            if sid:
                wd(f"/session/{sid}", method="DELETE")
            driver.terminate()
            driver.wait(timeout=10)


if __name__ == "__main__":
    unittest.main(verbosity=2)
