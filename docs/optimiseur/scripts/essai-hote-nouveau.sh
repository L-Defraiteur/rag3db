#!/bin/bash
# Ce poste : prouve que le binaire bâti à luciepc démarre ici, avant toute bascule. Ne touche ni l'hôte de Lucie
# ni sa socket : un hôte d'essai sur le port 7991, son propre dossier de démon, arrêté par son pid ($!).
# Usage : essai-hote-nouveau.sh <id de lib>
set -u
B=$HOME/.local/bin
X=${X:-$B/rag3weaver-backend.nouveau}; C=${C:-$B/rag3db-check-lib.nouveau}
L=$HOME/.cache/rag3db-libs/$1
rouge=0
echo "md5 : $(md5sum $X $C | cut -c1-8 | tr '\n' ' ')"
echo "RUNPATH : $(readelf -d $X | grep -io 'runpath.*')"
m=$(ldd $X | grep -c 'not found'); echo "ldd, manquants : $m"; [ "$m" = 0 ] || { ldd $X | grep 'not found'; rouge=1; }
g=$(objdump -T $X | grep -oE 'GLIBC_[0-9.]+' | sort -V | tail -1); echo "glibc demandée au plus : $g ; ici : $(ldd --version | head -1 | grep -oE '[0-9.]+$')"
$C $X $L/librag3db.so; c=$?; echo "check-lib : $c"; [ $c = 0 ] || rouge=1
o=$(nm -C $X | grep -c 'rag3weaver::openai_llm::')
# lucivy : depuis 37904f103 (7 nov. 2026), la version publiée du registre, et aucune révision git
# (un chemin `lucivy-<16hex>/<7hex>` dans le binaire voudrait dire un patch ou une dépendance git revenus).
f=$(strings $X | grep -oE "lucivy-core-[0-9]+\.[0-9]+\.[0-9]+" | sort -u | tr '\n' ' ')
g=$(strings $X | grep -oE "lucivy-[0-9a-f]{16}/[0-9a-f]{7}" | sort -u | tr '\n' ' ')
echo "openai_llm : $o ; lucivy : $f${g:+ ; révisions git : $g}"
[ "$f" = "lucivy-core-${LUCIVY_ATTENDU:-4.3.1} " ] || { echo "lucivy n'est pas ${LUCIVY_ATTENDU:-4.3.1} (lu : $f)"; rouge=1; }
[ -z "$g" ] || { echo "une révision git de lucivy est encore dans le binaire"; rouge=1; }
mkdir -p $HOME/.cache/rag3weaver-build/essais
D=$(mktemp -d -p $HOME/.cache/rag3weaver-build/essais essai-hote.XXXX) && [ -n "$D" ] || { echo "pas de dossier d'essai : rien lancé"; echo ROUGE; exit 3; }
RAG3WEAVER_DAEMON_DIR=$D RAG3WEAVER_DAEMON_IDLE_MIN=2 $X daemon --address 127.0.0.1:7991 > $D/journal 2>&1 &
P=$!
ok=0
for i in $(seq 1 30); do sleep 1; curl -fsS -m 2 http://127.0.0.1:7991/sante > $D/sante 2>/dev/null && { ok=1; break; }; kill -0 $P 2>/dev/null || break; done
echo "/sante : $([ $ok = 1 ] && cut -c1-200 $D/sante || echo 'pas de réponse')"
echo "lib chargée : $(grep -o 'rag3db-build-[0-9a-f]\{9\}' /proc/$P/maps 2>/dev/null | sort -u | tr '\n' ' ')"
[ $ok = 1 ] || { rouge=1; tail -5 $D/journal; }
kill -TERM $P 2>/dev/null; for i in $(seq 1 20); do kill -0 $P 2>/dev/null || break; sleep 1; done
kill -0 $P 2>/dev/null && { echo "l'hôte d'essai ne s'arrête pas"; rouge=1; }
echo "dossier d'essai : $D"
[ $rouge = 0 ] && echo VERT || echo ROUGE
