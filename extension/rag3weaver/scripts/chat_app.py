#!/usr/bin/env python3
"""Local web + terminal clients for rag3weaver-chat. Python standard library only."""
import argparse
import curses
import http.server
import json
import os
from pathlib import Path
import queue
import secrets
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
import textwrap

ROOT = Path(__file__).resolve().parents[1]


class Busy(Exception):
    pass


class Bridge:
    """One child owns the backend; both UIs serialize requests through it."""
    def __init__(self, command):
        self.child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      text=True, bufsize=1)
        self.turn = threading.Lock()
        self.writer = threading.Lock()
        self.closing = False
        self.active_session = None

    def send(self, value):
        with self.writer:
            self.child.stdin.write(json.dumps(value, ensure_ascii=False) + "\n")
            self.child.stdin.flush()

    def events(self, value):
        if not self.turn.acquire(blocking=False):
            raise Busy("Un tour est déjà en cours. Réessayez après sa fin.")
        try:
            if self.closing:
                raise RuntimeError("Le service s'arrête.")
            self.active_session = value.get("session") if value.get("op") == "chat" else None
            self.send(value)
            while True:
                line = self.child.stdout.readline()
                if not line:
                    raise RuntimeError("Le processus agent s'est arrêté ; voir le terminal.")
                event = json.loads(line)
                yield event
                if event.get("event") == "done":
                    break
        finally:
            self.active_session = None
            self.turn.release()

    def request(self, value):
        result = list(self.events(value))[-1]
        if not result.get("ok"):
            raise RuntimeError(result.get("error", "Agent error"))
        return result["result"]

    def cancel(self, session):
        # Cancellation is scoped to the turn, so another tab cannot cancel by accident.
        if session and session == self.active_session:
            self.send({"op": "cancel"})
            return True
        return False

    def close(self):
        self.closing = True
        if self.child.poll() is None:
            self.send({"op": "cancel"})
            with self.turn:
                self.send({"op": "shutdown"})
                # Graceful checkpoint: do not SIGKILL a database on a short UI timeout.
                for line in self.child.stdout:
                    if json.loads(line).get("result", {}).get("closed"):
                        break
                self.child.stdin.close()
                self.child.wait()
        self.child.stdout.close()


class LocalServer(http.server.ThreadingHTTPServer):
    daemon_threads = True
    def __init__(self, port, bridge, token):
        self.bridge, self.token = bridge, token
        self.info = bridge.request({"op": "describe"})
        self.state = Path(self.info.pop("state_dir")).resolve()
        super().__init__(("127.0.0.1", port), Handler)
        self.origin = f"http://127.0.0.1:{self.server_port}"


def simple_name(name):
    return isinstance(name, str) and bool(name) and len(name) <= 120 and not name.startswith(".") and all(
        c.isascii() and (c.isalnum() or c in "._-") for c in name)


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass  # Never log token-bearing request URLs.

    def response(self, status, value, content_type="application/json; charset=utf-8"):
        data = value if isinstance(value, bytes) else json.dumps(value, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'")
        self.end_headers()
        self.wfile.write(data)

    def allowed(self, auth=True):
        authority = urllib.parse.urlsplit(self.server.origin).netloc
        if self.headers.get("Host") != authority or self.headers.get("Origin", self.server.origin) != self.server.origin:
            self.response(403, {"error": "Origine refusée"})
            return False
        if auth and not secrets.compare_digest(self.headers.get("Authorization", ""), "Bearer " + self.server.token):
            self.response(401, {"error": "Ouvrez le lien du terminal pour autoriser ce navigateur."})
            return False
        return True

    def do_GET(self):
        parsed = urllib.parse.urlsplit(self.path)
        static = {"/": "index.html", "/app.js": "app.js", "/style.css": "style.css"}
        if not self.allowed(auth=parsed.path not in static):
            return
        try:
            if parsed.path in static:
                name = static[parsed.path]
                mime = {"html": "text/html", "js": "text/javascript", "css": "text/css"}[name.rsplit(".", 1)[1]]
                return self.response(200, (ROOT / "ui/chat" / name).read_bytes(), mime + "; charset=utf-8")
            if parsed.path == "/api/info":
                return self.response(200, self.server.info)
            if parsed.path == "/api/sessions":
                files = sorted((self.server.state / "sessions").glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)
                return self.response(200, {"sessions": [{"id": p.stem, "modified": p.stat().st_mtime} for p in files if simple_name(p.stem)]})
            if parsed.path == "/api/history":
                session = urllib.parse.parse_qs(parsed.query).get("session", [""])[0]
                result = self.server.bridge.request({"op": "history", "session": session})
                # System prompts stay server-side, including after a reload.
                result["turns"] = [t for t in result["turns"] if t["role"] != "system"]
                return self.response(200, result)
            if parsed.path == "/api/artifacts":
                root = self.server.state / "artifacts"
                return self.response(200, {"artifacts": [{"name": p.name, "bytes": p.stat().st_size} for p in sorted(root.glob("*")) if simple_name(p.name) and p.is_file() and not p.is_symlink()]})
            if parsed.path.startswith("/api/artifacts/"):
                name = urllib.parse.unquote(parsed.path[len("/api/artifacts/"):])
                path = self.server.state / "artifacts" / name
                if not simple_name(name) or path.is_symlink() or not path.is_file():
                    return self.response(404, {"error": "Export introuvable"})
                return self.response(200, path.read_bytes(), "application/octet-stream")
            self.response(404, {"error": "Route inconnue"})
        except Busy as exc:
            self.response(409, {"error": str(exc)})
        except (OSError, RuntimeError, ValueError) as exc:
            self.response(400, {"error": str(exc)})

    def do_POST(self):
        if not self.allowed():
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 256 * 1024:
                return self.response(413, {"error": "Requête vide ou trop grande"})
            value = json.loads(self.rfile.read(length))
            if not isinstance(value, dict) or not simple_name(value.get("session", "")):
                return self.response(400, {"error": "Identifiant de conversation invalide"})
            if self.path == "/api/cancel":
                return self.response(200, {"requested": self.server.bridge.cancel(value["session"])})
            if self.path != "/api/chat":
                return self.response(404, {"error": "Route inconnue"})
            if not isinstance(value.get("message"), str) or not value["message"].strip():
                return self.response(400, {"error": "Message vide"})
            events = self.server.bridge.events({"op": "chat", "session": value["session"], "message": value["message"]})
            # Acquire the single-turn lock before sending headers (409 on contention).
            first = next(events)
        except Busy as exc:
            return self.response(409, {"error": str(exc)})
        except (OSError, RuntimeError, ValueError, TypeError) as exc:
            return self.response(400, {"error": str(exc)})
        self.send_response(200)
        self.send_header("Content-Type", "application/x-ndjson; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        connected = True
        try:
            import itertools
            for event in itertools.chain([first], events):
                if connected:
                    try:
                        self.wfile.write((json.dumps(event, ensure_ascii=False) + "\n").encode())
                        self.wfile.flush()
                    except (BrokenPipeError, ConnectionResetError):
                        connected = False
                        self.server.bridge.cancel(value["session"])
                        # Drain through done before releasing the lock; no protocol desync.
        except (OSError, RuntimeError, ValueError) as exc:
            if connected:
                try:
                    self.wfile.write((json.dumps({"event": "done", "ok": False, "error": str(exc)}) + "\n").encode())
                except OSError:
                    pass
        finally:
            events.close()


def request(url, token, route, value=None):
    data = None if value is None else json.dumps(value).encode()
    req = urllib.request.Request(url + route, data=data, headers={"Authorization": "Bearer " + token, "Content-Type": "application/json"})
    return urllib.request.urlopen(req, timeout=300)


def terminal(url, token, session_id=None):
    """Curses is a client only: it never owns the database or an agent loop."""
    info = json.load(request(url, token, "/api/info"))
    inbox = queue.Queue()
    def ui(screen):
        curses.raw()  # Ctrl-Q must reach us instead of terminal flow control.
        screen.timeout(100)
        session = session_id or uuid.uuid4().hex
        history = json.load(request(url, token, "/api/history?session=" + urllib.parse.quote(session)))["turns"]
        transcript = [[t.get("tool_name") or t["role"], t["content"]] for t in history if t["content"]]
        compose, status = "", "Prêt"
        busy, scroll = False, 0
        def send(message, current_session):
            try:
                with request(url, token, "/api/chat", {"session": current_session, "message": message}) as response:
                    for line in response:
                        inbox.put(json.loads(line))
            except Exception as exc:
                inbox.put({"event": "done", "ok": False, "error": str(exc)})
        def put(y, text):
            try:
                screen.addnstr(y, 0, text, max(1, screen.getmaxyx()[1] - 1))
            except curses.error:
                pass
        while True:
            while not inbox.empty():
                event = inbox.get()
                kind = event.get("event")
                if kind == "token":
                    if not transcript or transcript[-1][0] != "Agent":
                        transcript.append(["Agent", ""])
                    transcript[-1][1] += event["text"]
                elif kind == "tool_start":
                    transcript.append(["Outil", event["name"] + " " + event["arguments"]])
                    status = "Exécution : " + event["name"]
                elif kind == "tool_end":
                    transcript.append(["Résultat", event["content"]])
                elif kind == "status":
                    status = event["text"]
                elif kind == "done":
                    busy = False
                    status = event.get("result", {}).get("stop", "Erreur")
                    accepted = event.get("result", {}).get("task_accepted")
                    if accepted is not None:
                        status = "Résultat accepté" if accepted else "Aucun résultat accepté"
                        transcript.append(["Validation", status])
                    if not event.get("ok"):
                        transcript.append(["Erreur", event.get("error", "Erreur")])
            height, width = screen.getmaxyx()
            screen.erase()
            put(0, f"{info['name']} · {info['model']}" + (" · DÉMO" if info["demo"] else ""))
            put(1, "Entrée envoyer · Échap arrêter · F2 nouveau · F3 exports · Pg↑↓ défiler · Ctrl-Q quitter")
            lines = []
            for role, content in transcript:
                for line in (role + " › " + content).splitlines():
                    lines.extend(textwrap.wrap(line, max(1, width - 2)) or [""])
                lines.append("")
            room = max(0, height - 6)
            scroll = min(scroll, max(0, len(lines) - room))
            end = len(lines) - scroll
            for i, line in enumerate(lines[max(0, end - room):end]):
                put(i + 3, line)
            put(max(2, height - 2), f"{status} · {session[:8]}")
            put(max(3, height - 1), "> " + compose[-max(1, width - 4):])
            screen.refresh()
            try:
                key = screen.get_wch()
            except curses.error:
                continue
            if key in ("\x11", "\x03"):
                if busy:
                    request(url, token, "/api/cancel", {"session": session}).close()
                return
            if key == "\x1b" and busy:
                request(url, token, "/api/cancel", {"session": session}).close()
                status = "Annulation demandée…"
            elif key == curses.KEY_F2 and not busy:
                session, transcript, compose, scroll = uuid.uuid4().hex, [], "", 0
            elif key == curses.KEY_F3:
                files = json.load(request(url, token, "/api/artifacts"))["artifacts"]
                transcript.append(["Exports", "\n".join(f"{v['name']} ({v['bytes']} octets)" for v in files) or "Aucun export"])
            elif key == curses.KEY_PPAGE:
                scroll += max(1, room // 2)
            elif key == curses.KEY_NPAGE:
                scroll = max(0, scroll - max(1, room // 2))
            elif key in ("\n", "\r") and compose.strip() and not busy:
                transcript.append(["Vous", compose])
                threading.Thread(target=send, args=(compose, session), daemon=True).start()
                busy, compose, status, scroll = True, "", "Réflexion…", 0
            elif key in (curses.KEY_BACKSPACE, "\x7f", "\b"):
                compose = compose[:-1]
            elif isinstance(key, str) and key.isprintable():
                compose += key
    curses.wrapper(ui)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", nargs="?", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/rag3weaver-chat")
    parser.add_argument("--port", type=int, default=8740)
    parser.add_argument("--web-only", action="store_true")
    parser.add_argument("--session", help="Resume this session ID in the TUI")
    parser.add_argument("--demo", action="store_true")
    parser.add_argument("--attach", help="Attach the TUI to an existing local URL; token from RAG3WEAVER_CHAT_TOKEN")
    args = parser.parse_args()
    if args.attach:
        parsed = urllib.parse.urlsplit(args.attach)
        if parsed.scheme != "http" or parsed.hostname != "127.0.0.1" or parsed.path not in ("", "/"):
            parser.error("--attach requires http://127.0.0.1:PORT")
        return terminal(args.attach.rstrip("/"), os.environ["RAG3WEAVER_CHAT_TOKEN"], args.session)
    if args.config is None:
        parser.error("config is required when starting the service")
    if not args.web_only and not sys.stdin.isatty():
        parser.error("TUI requires a terminal; use --web-only otherwise")
    bridge = Bridge([str(args.binary.resolve()), str(args.config.resolve())] + (["--demo"] if args.demo else []))
    server = None
    try:
        token = secrets.token_urlsafe(32)
        server = LocalServer(args.port, bridge, token)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        print(f"Chat : {server.origin}/#token={token}", flush=True)
        print("Fermer avec Ctrl-C (web seul) ou Ctrl-Q (TUI). Les conversations et exports sont conservés.", flush=True)
        if args.web_only:
            while True:
                time.sleep(0.5)
        else:
            terminal(server.origin, token, args.session)
    except KeyboardInterrupt:
        pass
    finally:
        if server:
            server.shutdown()
            server.server_close()
        bridge.close()


if __name__ == "__main__":
    main()
