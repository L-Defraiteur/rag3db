#!/usr/bin/env python3
"""Un faux hôte rag3weaver, pour mesurer le terminal (lot tty-5).

Il sert ce que `rag3weaver-tty` lit (`/ide/etat`, `/ide/teammate`, `/ide/teammate/list`,
`/ide/describe`, `/highlight`, le flux SSE `/ide/flux`), et note chaque POST dans un journal JSONL.
À la demande, il joue un pas scripté : un appel au modèle, des tokens, un bloc de code, un outil qui
écrit, puis une demande d'accord. Il ne dépend que de la bibliothèque standard de Python.

    faux_hote.py <port> <journal.jsonl> <daemon.json à écrire>

`POST /faux/marque {"texte": "…"}` publie tout de suite un token : sert à mesurer la latence
d'affichage (le temps jusqu'à ce que le texte paraisse à l'écran).
"""
import http.server
import json
import socketserver
import sys
import threading
import time
import urllib.parse

PORT, JOURNAL, DAEMON_JSON = int(sys.argv[1]), sys.argv[2], sys.argv[3]
JETON = "jeton-de-mesure"
MANIFESTE = "/faux/depot/backend.json"
TACHE = "/faux/taches/mesure/backend.json"
evenements = []
boite = []
cond = threading.Condition()
etat = {"state": "waiting"}


def publier(sujet, ev):
    with cond:
        i = len(evenements) + 1
        evenements.append((i, {"id": i, "sujet": sujet, "backend": TACHE, "at_ms": int(time.time() * 1000), "evenement": ev}))
        cond.notify_all()


def ligne(kind, text, de="teammate", **extra):
    with cond:
        n = len(boite)
        l = {"id": f"faux/mesure:{n}", "at": time.time(), "from": de, "kind": kind, "text": text, **extra}
        boite.append(l)
    publier("mailbox", {"kind": "MailboxLine", "parent": "faux", "task": "mesure", "line": l})


def pas_scripte():
    etat["state"] = "running"
    publier("agent", {"kind": "RunStarted", "run": "r1", "run_kind": "agent", "name": "chat:mesure"})
    publier("agent", {"kind": "LlmCall", "run": "r1"})
    time.sleep(1.5)  # le Cercle tourne
    for i in range(200):
        publier("tokens", {"kind": "Token", "run": "r1", "node": "chat", "seq": i, "text": f" mot{i}", "count": i + 1})
        time.sleep(0.005)
    for t in ["\n```rust\n", "fn main() {\n", "    println!(\"bonjour\");\n", "}\n", "```\n"]:
        publier("tokens", {"kind": "Token", "run": "r1", "node": "chat", "seq": 999, "text": t, "count": 1})
    publier("agent", {"kind": "ToolCallStarted", "run": "r1", "agent": "chat", "call_id": "c1", "tool": "edit_file", "arguments": "{}"})
    time.sleep(1.0)  # le Glitch tourne
    publier("agent", {"kind": "ToolCallFinished", "run": "r1", "agent": "chat", "call_id": "c1", "tool": "edit_file", "ok": True, "error_kind": None, "ms": 1000, "bytes": 10})
    publier("agent", {"kind": "RunFinished", "run": "r1", "run_kind": "agent", "ms": 4000, "ok": True})
    ligne("approval", "commande en attente d'accord : echo mesure", command="echo mesure")
    etat["state"] = "waiting"


class Gestion(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def json(self, v, code=200):
        b = json.dumps(v).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def garde(self):
        if self.headers.get("Authorization") != f"Bearer {JETON}":
            self.json({"ok": False, "error": "jeton"}, 403)
            return False
        return True

    def do_GET(self):
        if not self.garde():
            return
        u = urllib.parse.urlparse(self.path)
        if u.path == "/ide/etat":
            self.json({"backends": [
                {"id": "faux", "manifest": MANIFESTE, "repo": "faux-depot", "branch": "master", "tache": None},
                {"id": "faux-mesure", "manifest": TACHE, "repo": "faux-depot", "branch": "tache/mesure", "tache": {"nom": "mesure", "parent": "faux", "branche": "tache/mesure"}},
            ]})
        elif u.path == "/ide/teammate":
            with cond:
                self.json({"ok": True, "result": {"mailbox": list(boite), "backend": TACHE, "running": etat["state"] == "running"}})
        elif u.path == "/ide/teammate/list":
            self.json({"ok": True, "result": {"supervisor": "supervisor", "teammates": [{"task": "mesure", "parent": "faux", "state": etat["state"]}]}})
        elif u.path == "/ide/flux":
            depuis = self.headers.get("Last-Event-ID")
            with cond:
                n = int(depuis) if depuis else len(evenements)
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.wfile.write(b": faux\n\n")
            self.wfile.flush()
            try:
                while True:
                    with cond:
                        while len(evenements) <= n:
                            if not cond.wait(15):
                                break
                        nouveaux = evenements[n:]
                    if not nouveaux:
                        self.wfile.write(b": battement\n\n")
                    for i, d in nouveaux:
                        self.wfile.write(f"id: {i}\ndata: {json.dumps(d)}\n\n".encode())
                        n = i
                    self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError):
                pass
        else:
            self.json({"ok": False, "error": "inconnu"}, 404)

    def do_POST(self):
        if not self.garde():
            return
        corps = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
        with open(JOURNAL, "a") as j:
            j.write(json.dumps({"t": time.time(), "chemin": self.path, "corps": corps}) + "\n")
        if self.path == "/ide/teammate/mailbox":
            kind = corps.get("kind")
            ligne(kind, corps.get("text", ""), de="supervisor", **({"reply_to": corps["reply_to"]} if corps.get("reply_to") else {}))
            if kind == "request":
                threading.Thread(target=pas_scripte, daemon=True).start()
            if kind == "answer" and corps.get("approved") is True:
                ligne("ended", "fait", de="host")
                etat["state"] = "ended"
            self.json({"ok": True, "result": {"started": kind == "request"}})
        elif self.path == "/ide/describe":
            self.json({"tools": [{"name": "edit_file", "policy": {"write_files": True}}, {"name": "read_file", "policy": {"read_files": True}}]})
        elif self.path == "/highlight":
            self.json({"ok": True, "language": None, "spans": None})
        elif self.path == "/faux/marque":
            publier("tokens", {"kind": "Token", "run": "r1", "node": "chat", "seq": 5000, "text": corps.get("texte", ""), "count": 1})
            self.json({"ok": True, "result": {}})
        else:
            self.json({"ok": False, "error": "inconnu"}, 404)


class Serveur(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


import os

# Le pid écrit par le faux hôte lui-même : celui qui l'arrête vise ce processus, pas un lanceur.
with open(os.path.join(os.path.dirname(DAEMON_JSON), "faux.pid"), "w") as f:
    f.write(str(os.getpid()))
with open(DAEMON_JSON, "w") as f:
    json.dump({"address": f"127.0.0.1:{PORT}", "pid": 0, "token": JETON, "build": "faux"}, f)
Serveur(("127.0.0.1", PORT), Gestion).serve_forever()
