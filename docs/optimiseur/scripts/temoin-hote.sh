#!/bin/bash
# Témoin par le chemin de l'hôte (13 nov. 2026) : un hôte d'essai 7991 sous <lib> avec <binaire>, /sante, une copie reflink
# de code-lourd (d'un instantané) ouverte par l'hôte, et UNE recherche (un seul appel d'embarquement à 7878). Rien ne
# touche une base vivante. Usage : temoin-hote.sh <id de lib> <binaire> <instantané source, ex. avant-835d5298f>
set -u
LIB=${1:?lib}; X=${2:?binaire}; SRC=$HOME/.local/share/rag3weaver/instantanes/${3:?instantané}/code-lourd
LD=$HOME/.cache/rag3db-libs/$LIB; C=${LIB:13:9}; ET=temoin-$C
I=$HOME/.local/share/rag3weaver/instantanes/$ET; D=$HOME/.cache/rag3weaver-build/temoin-$C
dire() { echo "== $(date +%H:%M:%S) $*"; }
[ -e "$I" ] || [ -e "$D" ] && { dire "$ET existe déjà"; echo ROUGE; exit 3; }
[ -d "$SRC" ] && [ -f "$LD/librag3db.so" ] && [ -x "$X" ] || { dire "source, lib ou binaire absents"; echo ROUGE; exit 3; }
mkdir -p "$I/code-lourd" "$D/hote"
cp -a --reflink=always "$SRC"/. "$I/code-lourd/" || { echo ROUGE; exit 4; }
printf 'VECTOR\t%s/libvector.rag3db_extension\n' "$LD" > "$I/code-lourd/code-lourd.rag3db.extensions"
cp -a "$HOME/.config/rag3weaver/code-lourd/templates" "$D/templates"
python3 - "$I/code-lourd/code-lourd.rag3db" "$D/templates/backends/code/temoin.json" "$D/templates/backends/code/backend.json" <<'P'
import json, sys
db, out, src = sys.argv[1:4]
d = json.load(open(src)); d["database"] = db; d["name"] = "temoin"
for k in ("watches", "bilans"): d.pop(k, None)
json.dump(d, open(out, "w"), ensure_ascii=False, indent=2)
P
export LD_LIBRARY_PATH=$LD RAG3DB_LIBRARY_DIR=$LD
RAG3WEAVER_DAEMON_DIR=$D/hote RAG3WEAVER_DAEMON_IDLE_MIN=10 "$X" daemon --address 127.0.0.1:7991 > "$D/hote/journal" 2>&1 &
P=$!
for i in $(seq 1 60); do curl -fsS -m 2 http://127.0.0.1:7991/sante > "$D/sante" 2>/dev/null && break; sleep 1; done
dire "hôte d'essai $P ; /sante : $(cut -c1-160 $D/sante) ; lib : $(grep -o 'rag3db-build-[0-9a-f]\{9\}' /proc/$P/maps | sort -u | tr '\n' ' ')"
grep -q "$C" /proc/$P/maps || { kill -TERM $P; echo ROUGE; exit 5; }
python3 - "$D" <<'P'
import json, os, sys, urllib.request
D = sys.argv[1]; M = f"{D}/templates/backends/code/temoin.json"; J = open(f"{D}/hote/daemon.token").read().strip()
def d(m, r, c=None, s=None):
    q = urllib.request.Request("http://127.0.0.1:7991"+r, method=m, data=json.dumps(c).encode() if c is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json", **({"X-Rag3weaver-Session": s} if s else {})})
    try:
        with urllib.request.urlopen(q, timeout=300) as x: return json.loads(x.read() or b"{}")
    except urllib.error.HTTPError as e: return json.loads(e.read() or b"{}")
s = d("POST", "/session", {"client": "G-temoin", "pid": os.getpid()})["session"]
o = d("POST", "/backend/open", {"manifest": M}, s); print("ouvert :", o.get("ok"), (o.get("error") or "")[:200], flush=True)
st = d("POST", "/call", {"manifest": M, "name": "daemon_status", "arguments": {}}, s); print("daemon_status :", json.dumps(st, ensure_ascii=False)[:300], flush=True)
r = d("POST", "/call", {"manifest": M, "name": "search_code", "arguments": {"query": "LocalStorage::commit", "options": {}}}, s)
t = json.dumps(r, ensure_ascii=False); print("search_code :", "ok" if r.get("ok") else "ÉCHEC", len(t), "octets ;", t.count("commit"), "« commit » ;", t[:200].replace("\\n", " "))
d("DELETE", "/session", None, s)
sys.exit(0 if o.get("ok") and r.get("ok") else 1)
P
rc=$?
kill -TERM $P; for i in $(seq 1 60); do kill -0 $P 2>/dev/null || break; sleep 1; done
dire "hôte d'essai arrêté ; erreurs au journal : $(grep -ciE 'erreur|panic|segv|exception' $D/hote/journal)"
[ $rc = 0 ] && echo VERT || echo ROUGE
