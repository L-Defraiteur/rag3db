#!/bin/bash
# A/B default-export navigation (11 novembre 2026) : code-lourd (rag3db-lourd, a72acfeef) réindexé deux
# fois : A = aece4b9f1 / c660893, B = d96ed3264 / a730a9b ; puis la
# sonde, et l'oracle TypeScript (juge strict) sur les mêmes mentions que les 300 arêtes du rejugement.
# Toute la séquence sous un seul poste lourd (l-index-d-une-mesure-sous-poste-lourd).
set -euo pipefail
unset RAG3WEAVER_MOTEUR_ANCIEN
if ss -ltn | awk '{print $4}' | grep -q ":7978$"; then echo "ROUGE : port hôte 7978 occupé"; exit 1; fi
N=$HOME/.cache/codeparsers-notes
M=$N/require-default-v2-d96ed3264
# Never overwrite historical evidence. Reserve 7882 with optimiseur first.
[ ! -e "$M" ] || { echo "ROUGE : dossier de preuve déjà présent"; exit 1; }
mkdir -p "$M"
O=$M/resume.txt
LIB=rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
L=$HOME/.cache/rag3db-libs/$LIB
W=$HOME/git_workspaces/cx-wt-codeparsers-mesure-v2
R=$HOME/git_workspaces/cx-wt-codeparsers-require
LOT=d96ed3264
BASE=aece4b9f1
[ ! -e "$W" ] || { echo "ROUGE : worktree de mesure déjà présent"; exit 1; }
git -C "$R" worktree add --detach "$W" "$BASE" || exit 1
H=""
cleanup_host() {
  if [ -n "$H" ]; then
    kill -TERM "$H" 2>/dev/null || true
    wait "$H" 2>/dev/null || true
    H=""
  fi
}
trap cleanup_host EXIT INT TERM
echo "base $BASE, lib $LIB (sans MOTEUR_ANCIEN), $(date +%T)" > $O
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-codeparsers-clic CARGO_INCREMENTAL=0 RAG3DB_ROOT=$W RAG3DB_LIB=$LIB
git -C "$HOME/git_workspaces/rag3db-lourd" rev-parse HEAD > "$M/source-git.txt"
git -C "$HOME/git_workspaces/rag3db-lourd" status --porcelain --untracked-files=no > "$M/source-status.txt"
[ ! -s "$M/source-status.txt" ] || { echo "ROUGE : source suivie modifiée" >> "$O"; exit 1; }
for X in A B; do
  {
    date -Is
    cat /proc/pressure/io
    if [ -r /sys/class/drm/card2/device/gpu_busy_percent ]; then cat /sys/class/drm/card2/device/gpu_busy_percent; fi
  } > "$M/charge-avant-$X.txt"
  cd $W && git checkout -q --detach $BASE && git submodule update -q --init extension/rag3weaver/codeparsers || { echo "ROUGE arbre $X" >> $O; exit 1; }
  if [ $X = B ]; then git checkout -q --detach $LOT && git submodule update -q --init extension/rag3weaver/codeparsers || { echo "ROUGE arbre B" >> $O; exit 1; }; fi
  echo "$X : arbre $(git rev-parse --short HEAD), codeparsers $(git -C extension/rag3weaver/codeparsers rev-parse --short HEAD)" >> $O
  cd extension/rag3weaver
  RUSTFLAGS="-C link-arg=-Wl,-rpath,$L" bash scripts/cargo_magasin.sh build -j8 --release --bin rag3weaver-backend --features rag3db-native,code,daemon,openai-llm > $M/bati-$X.log 2>&1 || { echo "ROUGE bâti $X" >> $O; exit 1; }
  cp $CARGO_TARGET_DIR/release/rag3weaver-backend $M/backend-$X
  echo "$X : binaire $(md5sum $M/backend-$X | cut -c1-12), RUNPATH $(readelf -d $M/backend-$X | grep -o 'rag3db-build-[0-9a-f]\{9\}')" >> $O
  D=$M/$X; mkdir -p $D/daemon $D/data
  ( trap cleanup_host EXIT INT TERM
    export LD_LIBRARY_PATH= RAG3DB_LIBRARY_DIR=$L RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7882 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
    export RAG3DB_ROOT=$HOME/git_workspaces/rag3db-lourd RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:7978 RAG3WEAVER_DAEMON_DIR=$D/daemon
    $M/backend-$X daemon --address 127.0.0.1:7978 > $D/hote.log 2>&1 &
    H=$!; for i in $(seq 1 50); do curl -s -m 1 http://127.0.0.1:7978/sante >/dev/null && break; sleep 0.2; done
    mkdir -p $D/lourd; cp -a ~/.config/rag3weaver/code-lourd/templates $D/lourd/
    MF=$D/lourd/templates/backends/code/backend.json
    python3 -I -c "import json;m='$MF';d=json.load(open(m));d['database']='$D/data/code-lourd.rag3db';d['embeddings']['address']='127.0.0.1:7882';json.dump(d,open(m,'w'),indent=2,ensure_ascii=False)"
    # Le disque de luciepc peut tenir l'ouverture d'une base neuve plus de 30 s (8 nov.) : on attend
    # qu'il se calme (PSI io avg60 sous 8), puis on retente l'appel jusqu'à trois fois.
    for k in $(seq 1 120); do awk '/^some/{split($3,x,"=");exit !(x[2]<8)}' /proc/pressure/io && break; sleep 30; done
    t0=$(date +%s); J=""
    for essai in 1 2 3; do
      Rp=$(cd $N && python3 mcp_appel.py $MF index '{"confirm":true}' 2>&1) || true
      J=$(printf '%s' "$Rp" | grep -o '/[^ "\\]*commandes/index/[^ "\\]*\.out' | head -1) || true
      [ -n "$J" ] && break
      echo "$X : appel index, essai $essai sans journal ($(head -1 /proc/pressure/io | cut -c1-40))" >> $O; sleep 30
    done
    [ -z "$J" ] && { echo "ROUGE $X : pas de journal : $(printf '%s' "$Rp" | head -c 400)" >> $O; kill -TERM $H; exit 1; }
    while ! grep -qE "indexation (terminée|échouée)" "$J" 2>/dev/null; do kill -0 $H 2>/dev/null || { echo "ROUGE $X : hôte mort" >> $O; exit 1; }; sleep 5; done
    grep -q "indexation terminée" "$J" || { echo "ROUGE index $X : $J" >> "$O"; exit 1; }
    echo "$X : index en $(( $(date +%s) - t0 )) s ; $(tail -n 1 "$J" | cut -c1-120)" >> $O
    cleanup_host ) || exit 1
  {
    date -Is
    cat /proc/pressure/io
    if [ -r /sys/class/drm/card2/device/gpu_busy_percent ]; then cat /sys/class/drm/card2/device/gpu_busy_percent; fi
  } > "$M/charge-apres-$X.txt"
done
echo "index faits $(date +%T)" >> $O
# La sonde, bâtie avec codeparsers B (le dernier état de l'arbre) : elle ne lit que la base.
bash scripts/cargo_magasin.sh test -j8 --features rag3db-native,code --test sonde_aretes_par_relation --no-run > $M/sonde-bati.log 2>&1 || { echo "ROUGE bâti de la sonde" >> $O; exit 1; }
T=$(grep -o '/[^ ]*deps/sonde_aretes_par_relation-[0-9a-f]*' $M/sonde-bati.log | tail -1)
Q='MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.file_path, r.line, b.name, b.file_path, b.start_line, b.end_line, r.resolution, a.name, b.parent_name'
for X in A B; do
  c=$M/copie-$X.rag3db; [ ! -e "$c" ] || exit 1; cp -a --reflink=auto $M/$X/data/code-lourd.rag3db "$c"
  for suffix in .wal .extensions .fts; do
    if [ -e "$M/$X/data/code-lourd.rag3db$suffix" ]; then
      cp -a --reflink=auto "$M/$X/data/code-lourd.rag3db$suffix" "$c$suffix"
    fi
  done
  LD_LIBRARY_PATH=$L SONDE_BASE=$c SONDE_REQUETE="$Q" timeout 1200 "$T" --ignored --nocapture --exact compter_les_aretes > $M/aretes-$X.txt 2>&1
  LD_LIBRARY_PATH=$L SONDE_BASE=$c SONDE_REQUETE='MATCH (s:Scope) RETURN s.name, s.parent_name, s.file_path, s.start_line, s.end_line' timeout 1200 "$T" --ignored --nocapture --exact compter_les_aretes > $M/scopes-$X.txt 2>&1
  # Retain the disposable read copy with this evidence.
  echo "$X : $(grep -c '^ligne' $M/aretes-$X.txt) CONSUMES" >> $O
done
# Les mêmes mentions que le rejugement : (fichier, ligne, nom) des 300 verdicts TypeScript.
python3 -I - "$M" "$N/rejuge-lsp/verdicts-typescript-liaison.tsv" >> $O <<'P'
import ast, re, sys
from collections import Counter
M, V = sys.argv[1], sys.argv[2]
mentions = set()
for l in open(V).read().splitlines()[1:]:
    c = l.split("\t"); f, ligne = c[3].rsplit(":", 1); mentions.add((f, int(ligne), c[4]))
JS = (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts")
def lire(p):
    out = {}
    for l in open(p, errors="replace"):
        if not l.startswith("ligne\t"): continue
        b = re.sub(r'String\((".*?")\)', r"\1", l[6:].strip()); b = re.sub(r"Int\((-?\d+)\)", r"\1", b).replace("Null", "None")
        try: v = ast.literal_eval(b)
        except Exception: continue
        if v and isinstance(v[0], str) and v[0].endswith(JS): out[(v[0], v[1], v[2], v[3], v[4], v[7])] = (l, v)
    return out
for X in "AB":
    d = lire(f"{M}/aretes-{X}.txt")
    garde = [l for k, (l, v) in d.items() if (v[0], v[1], v[2]) in mentions]
    open(f"{M}/memes-{X}.txt", "w").writelines(garde)
    print(f"{X} : {len(d)} CONSUMES d'un mentionneur TS/JS ; {len(garde)} sur les {len(mentions)} mentions du rejugement")
a = lire(f"{M}/aretes-A.txt"); b = lire(f"{M}/aretes-B.txt")
print(f"TS/JS : ajoutées {len(b.keys() - a.keys())}, ôtées {len(a.keys() - b.keys())}")
P
O2=$N/oracle; A2=$O2/arbre
for X in A B; do
  cd $O2 && ORACLE_STRICT=1 ORACLE_PLAFOND=600 ORACLE_TSSERVER=$O2/ts5/node_modules/typescript/lib python3 -I oracle_lsp.py typescript $M/memes-$X.txt $M/scopes-$X.txt $A2 $A2 1000 $M/verdicts-$X.tsv > $M/oracle-$X.out 2>&1
  echo "== oracle $X" >> $O; cat $M/oracle-$X.out >> $O
done
python3 -I - "$M" <<'Q'
import ast, re, sys
M=sys.argv[1]; JS=(".ts",".tsx",".js",".jsx",".mjs",".cjs",".mts",".cts")
def lire(p):
    out={}
    for l in open(p, errors="replace"):
        if not l.startswith("ligne\t"): continue
        b=re.sub(r'String\((".*?")\)', r"\1", l[6:].strip()); b=re.sub(r"Int\((-?\d+)\)", r"\1", b).replace("Null","None")
        try: v=ast.literal_eval(b)
        except Exception: continue
        if v and isinstance(v[0],str) and v[0].endswith(JS): out[(v[0],v[1],v[2],v[3],v[4])]=l
    return out
a=lire(f"{M}/aretes-A.txt"); b=lire(f"{M}/aretes-B.txt")
open(f"{M}/ajoutees-js.txt","w").writelines(b[k] for k in b.keys()-a.keys())
open(f"{M}/otees-js.txt","w").writelines(a[k] for k in a.keys()-b.keys())
Q
for X in ajoutees otees; do S=B; [ $X = otees ] && S=A
  cd $O2 && ORACLE_STRICT=1 ORACLE_PLAFOND=600 ORACLE_TSSERVER=$O2/ts5/node_modules/typescript/lib python3 -I oracle_lsp.py typescript $M/$X-js.txt $M/scopes-$S.txt $A2 $A2 1000 $M/verdicts-$X.tsv > $M/oracle-$X.out 2>&1
  echo "== oracle $X" >> $O; cat $M/oracle-$X.out >> $O
done
echo fini >> $O
