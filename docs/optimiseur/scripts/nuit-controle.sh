#!/bin/bash
# luciepc, nuit du 8 au 9 nov. 2026 : contrôle des deux sens sous la lib neuve SANS index complet.
# 1. instantané avant-<lib> des bases de luciepc (hôte arrêté) : c'est aussi la recette de retour de la pose ;
# 2. copie reflink ouverte par un hôte d'essai 7991 (binaire de la repose contre la lib neuve), montre comprise,
#    90 s, arrêt TERM ; 3. controle-des-deux-sens.sh sur la copie ouverte, sous la lib neuve.
# Usage : nuit-controle.sh <id de lib> <binaire>
set -u
LIB=${1:?lib}; X=${2:?binaire}; LD=$HOME/.cache/rag3db-libs/$LIB; C=${LIB:13:9}
R=$HOME/.local/share/rag3weaver; I=$R/instantanes; ET=nuit-$C-ouvert; D=$HOME/.cache/rag3weaver-build/nuit-$C/controle
dire() { echo "== $(date +%H:%M:%S) $*"; }
[ "$(systemctl --user is-active rag3weaver-daemon)" = inactive ] || { dire "l'hôte de luciepc tourne : rien"; echo ROUGE; exit 3; }
[ -e $I/avant-$C ] || bash $HOME/.cache/rag3weaver-notes-A/instantanes.sh avant-$C $R/code-principal.rag3db $R/code-lourd.rag3db || { echo ROUGE; exit 4; }
[ -e $I/$ET ] && { dire "$ET existe déjà"; echo ROUGE; exit 3; }
mkdir -p $D/hote
for b in code-lourd code-principal; do
  mkdir -p $I/$ET/$b && cp -a --reflink=always $I/avant-$C/$b/. $I/$ET/$b/ || { echo ROUGE; exit 4; }
  printf 'VECTOR\t%s/libvector.rag3db_extension\n' "$LD" > $I/$ET/$b/$b.rag3db.extensions
  cp -a $HOME/.config/rag3weaver/$b/templates $D/templates-$b
  python3 - $I/$ET/$b/$b.rag3db $D/templates-$b/backends/code/nuit.json $D/templates-$b/backends/code/backend.json <<'P'
import json, sys
db, out, src = sys.argv[1:4]
d = json.load(open(src)); d["database"] = db; d["name"] = d["name"] + "-nuit"; d.pop("bilans", None)
json.dump(d, open(out, "w"), ensure_ascii=False, indent=2)
P
done
export LD_LIBRARY_PATH=$LD RAG3DB_LIBRARY_DIR=$LD
RAG3WEAVER_DAEMON_DIR=$D/hote RAG3WEAVER_DAEMON_IDLE_MIN=30 "$X" daemon --address 127.0.0.1:7991 > $D/hote/journal 2>&1 &
P=$!
for i in $(seq 1 60); do curl -fsS -m 2 http://127.0.0.1:7991/sante > /dev/null 2>&1 && break; sleep 1; done
grep -q "$C" /proc/$P/maps || { dire "la lib chargée n'est pas $C"; kill -TERM $P; echo ROUGE; exit 5; }
dire "hôte d'essai $P sur $C"
python3 - $D <<'P'
import json, os, sys, time, urllib.request
D = sys.argv[1]; J = open(f"{D}/hote/daemon.token").read().strip()
def d(m, r, c=None, s=None):
    q = urllib.request.Request("http://127.0.0.1:7991"+r, method=m, data=json.dumps(c).encode() if c is not None else None,
        headers={"Authorization": f"Bearer {J}", "Content-Type": "application/json", **({"X-Rag3weaver-Session": s} if s else {})})
    try:
        with urllib.request.urlopen(q, timeout=900) as x: return json.loads(x.read() or b"{}")
    except urllib.error.HTTPError as e: return json.loads(e.read() or b"{}")
s = d("POST", "/session", {"client": "G-nuit", "pid": os.getpid()})["session"]
ok = True
for b in ("code-lourd", "code-principal"):
    o = d("POST", "/backend/open", {"manifest": f"{D}/templates-{b}/backends/code/nuit.json"}, s)
    print(b, "ouvert :", o.get("ok"), (o.get("error") or "")[:300], flush=True); ok = ok and bool(o.get("ok"))
time.sleep(90)
d("DELETE", "/session", None, s)
sys.exit(0 if ok else 1)
P
rc=$?
kill -TERM $P; for i in $(seq 1 120); do kill -0 $P 2>/dev/null || break; sleep 1; done
kill -0 $P 2>/dev/null && { dire "l'hôte d'essai ne s'arrête pas"; echo ROUGE; exit 6; }
dire "hôte d'essai arrêté ; .wal : $(ls $I/$ET/*/*.wal 2>/dev/null | wc -l) ; erreurs au journal : $(grep -ciE 'erreur|panic|segv|exception' $D/hote/journal)"
[ $rc = 0 ] || { dire "une ouverture a échoué"; echo ROUGE; exit 7; }
bash $HOME/git_workspaces/rag3db-controle-G/docs/procedures/scripts/controle-des-deux-sens.sh $ET $LIB
