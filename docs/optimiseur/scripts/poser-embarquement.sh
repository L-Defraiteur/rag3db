#!/bin/bash
# Pose en place du binaire d'embarquement à luciepc (15 nov. 2026) : jamais d'essai à côté sur card2 (règle du 25 oct.),
# card0 est celle de seat1. Pour chaque instance demandée, dans l'ordre : un vecteur témoin noté sur l'ancien binaire,
# arrêt de l'unité, binaire neuf posé (l'ancien gardé en .avant-<hash>), redémarrage, puis le témoin d'embarquement :
# /sante (max_chars_per_text 12000, chars_per_token 3), le vecteur témoin égal au bit près, 413 sur 13 000 caractères.
# Un rouge remet l'ancien binaire et relance l'unité. Usage : poser-embarquement.sh <hash> <instance>...
#   (instances : granite-278m pour 7878, bge-m3 pour 7879)
set -u
H=${1:?hash}; shift; [ $# -gt 0 ] || { echo "usage : poser-embarquement.sh <hash> <instance>..."; exit 2; }
ssh -o BatchMode=yes luciepc "bash -s" "$H" "$@" <<'X'
set -u
H=$1; shift
B=$HOME/.cache/rag3weaver-build/nuit-$H/rag3weaver-embeddings
E=$HOME/.local/bin/rag3weaver-embeddings
[ -x "$B" ] || { echo "ROUGE : pas de binaire bâti en $B"; exit 3; }
[ -f "$E.avant-$H" ] || cp -p "$E" "$E.avant-$H"
echo "ancien $(sha256sum "$E.avant-$H" | cut -c1-16) ; neuf $(sha256sum "$B" | cut -c1-16)"
vecteur() { curl -s -m 60 -H 'Content-Type: application/json' -d '{"texts":["fn main() { println!(\"témoin de pose\"); }"]}' "http://$1/embed"; }
attendre() { for i in $(seq 1 90); do curl -s -m 2 "http://$1/sante" >/dev/null && return 0; sleep 1; done; return 1; }
R=0
for I in "$@"; do
  A=$(sed -n 's/^RAG3WEAVER_EMBEDDINGS_ADDR=//p' ~/.config/rag3weaver/embeddings/$I.env)
  [ -n "$A" ] || { echo "ROUGE : pas d'adresse pour $I"; R=1; break; }
  echo "== $(date +%T) $I ($A)"
  AVANT=$(vecteur "$A"); [ -n "$AVANT" ] || { echo "ROUGE : $I ne répond pas avant la pose"; R=1; break; }
  systemctl --user stop rag3weaver-embeddings@$I
  [ "$(sha256sum "$E" | cut -c1-64)" = "$(sha256sum "$B" | cut -c1-64)" ] || install -m 755 "$B" "$E"
  systemctl --user reset-failed rag3weaver-embeddings@$I 2>/dev/null
  systemctl --user start rag3weaver-embeddings@$I
  T0=$(date +%s)
  ok=1
  attendre "$A" || { echo "ROUGE : $I ne répond pas en 90 s"; ok=0; }
  if [ $ok = 1 ]; then
    echo "  coupure : $(( $(date +%s) - T0 )) s jusqu'au /sante"
    SANTE=$(curl -s -m 5 "http://$A/sante")
    APRES=$(vecteur "$A")
    python3 -c 'import json,os,sys,urllib.request
s=json.loads(sys.argv[1] or "{}"); a=json.loads(sys.argv[2] or "{}")["vectors"][0]; b=json.loads(sys.argv[3] or "{}")["vectors"][0]
neuf = s.get("max_chars_per_text")==12000 and s.get("chars_per_token")==3
ecart = max(abs(x-y) for x,y in zip(a,b)) if len(a)==len(b) else None
print("  sante :", s.get("modele"), "dim", s.get("dim"), "max_chars_per_text", s.get("max_chars_per_text"), "chars_per_token", s.get("chars_per_token"))
print("  vecteur témoin :", len(b), "composantes, écart absolu max", ecart)
sys.exit(0 if neuf and ecart == 0 else 1)' "$SANTE" "$AVANT" "$APRES" || { echo "ROUGE : $I, /sante ou vecteur"; ok=0; }
  fi
  if [ $ok = 1 ]; then
    python3 -c 'import json;print(json.dumps({"texts":["a"*13000]}))' > /tmp/pose-embed-long.json
    C=$(curl -s -m 30 -o /tmp/pose-embed-long.out -w '%{http_code}' -H 'Content-Type: application/json' --data @/tmp/pose-embed-long.json "http://$A/embed")
    echo "  13 000 caractères : HTTP $C, $(head -c 160 /tmp/pose-embed-long.out)"
    [ "$C" = 413 ] && grep -q '13000 caractères, plafond 12000' /tmp/pose-embed-long.out || { echo "ROUGE : $I, pas de 413 nommé"; ok=0; }
    rm -f /tmp/pose-embed-long.json /tmp/pose-embed-long.out
  fi
  systemctl --user show rag3weaver-embeddings@$I -p ActiveState -p NRestarts -p ExecMainStatus | tr '\n' ' '; echo
  if [ $ok = 0 ]; then
    echo "  retour à l'ancien binaire pour $I"
    systemctl --user stop rag3weaver-embeddings@$I; install -m 755 "$E.avant-$H" "$E"
    systemctl --user reset-failed rag3weaver-embeddings@$I 2>/dev/null; systemctl --user start rag3weaver-embeddings@$I
    attendre "$A" && echo "  $I relancé sur l'ancien" || echo "  ✗ $I ne répond plus, même sur l'ancien"
    R=1; break
  fi
done
[ $R = 0 ] && echo VERT || echo ROUGE
X
