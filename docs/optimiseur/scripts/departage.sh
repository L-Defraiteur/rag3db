#!/bin/bash
# luciepc : un run du départage des écarts aux deux sens (8 nov. 2026). Une copie reflink neuve de
# coupure-12h06/<base> (hotfixs, 0 écart), un hôte d'essai 7991 avec <binaire> sur <lib> (LD_LIBRARY_PATH et
# RAG3DB_LIBRARY_DIR), un manifeste copié sans montre ni bilans, `index confirm` complet, arrêt TERM, puis
# controle-des-deux-sens.sh sous <lib>. Rien ne touche une base vivante.
# Usage : departage.sh <étiquette> <binaire> <id de lib> [base, défaut code-lourd]
set -u
ET=${1:?étiquette}; X=${2:?binaire}; LIB=${3:?lib}; BASE=${4:-code-lourd}
LD=$HOME/.cache/rag3db-libs/$LIB
SRC=${SRC:-$HOME/.cache/rag3db-locks/coupure-12h06/$BASE}
# c7, 15 h 45 : le run 3 se fait sous 04063ee06 (format 42, il ne lit pas la copie de 12 h 06), lancé à part
[ "$ET" = departage-3b-cbc ] && { echo "== departage-3b-cbc remplacé par le run 3c sous 04063ee06 : rien lancé"; exit 9; }
I=$HOME/.local/share/rag3weaver/instantanes/$ET
D=$HOME/.cache/rag3weaver-build/departage/$ET
dire() { echo "== $(date +%H:%M:%S) $*"; }
[ -e "$I" ] || [ -e "$D" ] && { dire "$ET existe déjà"; echo ROUGE; exit 3; }
[ -x "$X" ] && [ -f "$LD/librag3db.so" ] && [ -d "$SRC" ] || { dire "binaire, lib ou copie source absents"; echo ROUGE; exit 3; }
mkdir -p "$I/$BASE" "$D/hote"
cp -a --reflink=always "$SRC"/. "$I/$BASE/" || { echo ROUGE; exit 4; }
# la copie de 12 h 06 charge l'extension vectorielle de dadbf0041 : chaque run prend celle de sa lib
printf 'VECTOR\t%s/libvector.rag3db_extension\n' "$LD" > "$I/$BASE/$BASE.rag3db.extensions"
ls "$I/$BASE" | tr '\n' ' '; echo; cat "$I/$BASE/$BASE.rag3db.extensions"
dire "arbre indexé : $(git -C $HOME/git_workspaces/rag3db-lourd log -1 --format=%h) ; $(git -C $HOME/git_workspaces/rag3db-lourd status --short | wc -l) modifiés"
# le manifeste a des chemins relatifs (../../tools/*.mmd) : on copie l'arbre des gabarits entier et on écrit
# le manifeste à la même place que l'original
cp -a "$HOME/.config/rag3weaver/$BASE/templates" "$D/templates" || { echo ROUGE; exit 4; }
# EMBED=<adresse> : un service d'embarquement prêté (9 nov. : jamais 7878, où passent les worktrees de Lucie)
python3 - "$I/$BASE/$BASE.rag3db" "$D/templates/backends/code/departage.json" "$HOME/.config/rag3weaver/$BASE/templates/backends/code/backend.json" "${EMBED:-}" <<'EOF'
import json, sys
db, out, src, embed = sys.argv[1:5]
d = json.load(open(src)); d["database"] = db; d["name"] = "departage"
if embed: d["embeddings"]["address"] = embed
print("embarquement :", d["embeddings"].get("address"))
for k in ("watches", "bilans"): d.pop(k, None)
json.dump(d, open(out, "w"), ensure_ascii=False, indent=2)
EOF
export LD_LIBRARY_PATH=$LD RAG3DB_LIBRARY_DIR=$LD
RAG3WEAVER_DAEMON_DIR=$D/hote RAG3WEAVER_DAEMON_IDLE_MIN=120 "$X" daemon --address 127.0.0.1:7991 > "$D/hote/journal" 2>&1 &
P=$!
for i in $(seq 1 60); do curl -fsS -m 2 http://127.0.0.1:7991/sante > /dev/null 2>&1 && break; sleep 1; done
dire "hôte d'essai pid $P ; lib chargée : $(grep -o 'rag3db-build-[0-9a-f]\{9\}' /proc/$P/maps | sort -u | tr '\n' ' ')"
grep -q "${LIB:13:9}" /proc/$P/maps || { dire "la lib chargée n'est pas $LIB"; kill -TERM $P; wait $P; echo ROUGE; exit 5; }
python3 - "$D" <<'EOF' | tee "$D/index.txt"
import json, os, re, sys, time, urllib.request
D = sys.argv[1]; M = f"{D}/templates/backends/code/departage.json"
J = open(f"{D}/hote/daemon.token").read().strip()
def d(m, r, c=None, s=None, t=900):
    q = urllib.request.Request("http://127.0.0.1:7991"+r, method=m, data=json.dumps(c).encode() if c is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json", **({"X-Rag3weaver-Session": s} if s else {})})
    try:
        with urllib.request.urlopen(q, timeout=t) as x: return json.loads(x.read() or b"{}")
    except urllib.error.HTTPError as e: return json.loads(e.read() or b"{}")
s = d("POST", "/session", {"client": "G-departage", "pid": os.getpid()})["session"]
try:
    o = d("POST", "/backend/open", {"manifest": M}, s); print("ouvert :", o.get("ok"), (o.get("error") or "")[:300], flush=True)
    if not o.get("ok"): sys.exit(1)
    r = json.dumps(d("POST", "/call", {"manifest": M, "name": "index", "arguments": {"confirm": True}}, s), ensure_ascii=False)
    j = re.search(r'journal : ([^\\"]+\.out)', r)
    if not j: print("pas de journal :", r[:400]); sys.exit(1)
    j = j.group(1); print("journal :", j, flush=True)
    while True:
        time.sleep(15)
        ls = open(j).read().splitlines()
        if ls and re.match(r"^(indexation terminée|indexation échouée|indexation arrêtée)", ls[-1]): break
    for l in ls:
        if re.search(r"écart|terminée|échouée|arrêtée|vecteurs prêts|fichiers sur", l): print("  " + l[:200])
finally:
    d("DELETE", "/session", None, s)
EOF
rc=${PIPESTATUS[0]}
kill -TERM $P; for i in $(seq 1 120); do kill -0 $P 2>/dev/null || break; sleep 1; done
kill -0 $P 2>/dev/null && { dire "l'hôte d'essai ne s'arrête pas"; echo ROUGE; exit 6; }
dire "hôte d'essai arrêté ; .wal restant : $([ -e "$I/$BASE/$BASE.rag3db.wal" ] && stat -c %s "$I/$BASE/$BASE.rag3db.wal" || echo absent)"
[ $rc = 0 ] || { dire "l'index n'a pas tourné : pas de contrôle"; echo ROUGE; exit 7; }
J=$(sed -n 's/^journal : //p' "$D/index.txt"); [ -n "$J" ] && tail -1 "$J" | grep -q '^indexation terminée' || { dire "dernier journal d'index sans « indexation terminée »"; echo ROUGE; exit 7; }
bash $HOME/git_workspaces/rag3db-controle-G/docs/procedures/scripts/controle-des-deux-sens.sh "$ET" "$LIB"
echo "détail : $HOME/.local/share/rag3weaver/instantanes/$ET-controle/$BASE.log"
