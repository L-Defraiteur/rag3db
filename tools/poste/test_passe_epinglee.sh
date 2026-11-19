#!/bin/bash
# Témoin de passe_epinglee.sh, sur un dépôt jouet : run_e2e.sh, check_features.sh, cargo_magasin.sh et `hostname`
# sont des faux, qui écrivent ce qu'on leur demande et notent leur environnement. Chaque cas dit vert ou ROUGE.
# Usage : bash tools/poste/test_passe_epinglee.sh [chemin de passe_epinglee.sh] — sortie 0 si tout est vert.
set -u
X=$(realpath "${1:-$(dirname "$0")/passe_epinglee.sh}")
D=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}"); trap 'rm -rf "$D"' EXIT
H=$D/home; W=$H/git_workspaces/wt/extension/rag3weaver; mkdir -p $W/scripts $W/src $D/bin
git -C $H/git_workspaces/wt init -q 2>/dev/null || { mkdir -p $H/git_workspaces/wt && git -C $H/git_workspaces/wt init -q; }
echo 'fn f() {}' > $W/src/lib.rs
# La table des suites exigées se prend à côté du témoin, dans le dépôt, et non à côté de la passe jugée : la copie du
# cache n'a pas de voisin (16 novembre 2026, ticket de G). Absente, le témoin s'arrête au lieu de rougir plus loin.
ICI_TEMOIN=$(dirname "$(realpath "$0")")
mkdir -p $H/git_workspaces/wt/tools/poste && cp "$ICI_TEMOIN/required_suites.sh" "$ICI_TEMOIN/required_suites.txt" $H/git_workspaces/wt/tools/poste/ || { echo "ROUGE  la table des suites exigées manque à côté du témoin ($ICI_TEMOIN)"; exit 1; }
git -C $H/git_workspaces/wt -c user.email=t@t -c user.name=t add -A > /dev/null && git -C $H/git_workspaces/wt -c user.email=t@t -c user.name=t commit -qm base
# master sous la tête : la base, puis un commit du lot par-dessus
git -C $H/git_workspaces/wt update-ref refs/remotes/origin/master HEAD; BASE_JOUET=$(git -C $H/git_workspaces/wt rev-parse HEAD)
echo '// lot' >> $W/src/lib.rs; git -C $H/git_workspaces/wt -c user.email=t@t -c user.name=t commit -qam lot
# Les faux : FAUX_ROUGE=<suite> fait échouer cette suite ; FAUX_NON_JOUEES=<suite> y écrit « NON JOUÉES » ;
# FAUX_CHECK=rouge rend check_features sans sa dernière ligne verte ; FAUX_PREUVE=muette ne recompile rien.
cat > $W/run_e2e.sh <<'F'
#!/bin/bash
s=""; dernier=""; while [ $# -gt 0 ]; do [ "$1" = --test ] && s=$2; [ "$1" = --lib ] && s=lib; dernier=$1; shift; done
echo "$s EMBED=$RAG3WEAVER_EMBED_SERVICE LIB=$RAG3DB_LIB REPLI=${RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE:-absent}" >> "$FAUX_TRACE"
[ "${FAUX_NON_JOUEES:-}" = "$s" ] && echo "▸ ⚠ 1 suites NON JOUÉES faute de feature"
[ "${FAUX_NON_JUGE:-}" = "$s" ] && echo "non jugé : la lib n'a pas la colonne state (format 43 requis), bascule à venir"
[ "${FAUX_ROUGE:-}" = "$s" ] && { echo "test result: FAILED. 0 passed; 1 failed"; exit 101; }
# FAUX_CAS_VERTS=<suite>:<cas>,… et FAUX_CAS_ROUGES=<suite>:<cas>,… nomment des cas, comme cargo les dit.
if [ "${FAUX_CAS_VERTS%%:*}" = "$s" ]; then for c in $(echo "${FAUX_CAS_VERTS#*:}" | tr , ' '); do echo "test $c ... ok"; done; fi
if [ "${FAUX_CAS_ROUGES%%:*}" = "$s" ]; then
  n=0; for c in $(echo "${FAUX_CAS_ROUGES#*:}" | tr , ' '); do echo "test $c ... FAILED"; n=$((n + 1)); done
  echo "test result: FAILED. 2 passed; $n failed"; exit 101
fi
# Le témoin (le filtre « filtre ») : cargo écrit la ligne du test joué ; FAUX_TEMOIN_MUET l'ignore (0 passed).
if [ "$dernier" = filtre ]; then
  [ -n "${FAUX_TEMOIN_MUET:-}" ] && { echo "test result: ok. 0 passed; 0 failed; 1 ignored"; exit 0; }
  echo "test tests::filtre_du_lot ... ok"
fi
echo "test result: ok. 1 passed; 0 failed"
F
cat > $W/scripts/check_features.sh <<'F'
#!/bin/bash
echo "VERT  sans-feature"; [ "${FAUX_CHECK:-}" = rouge ] && { echo "ROUGE daemon"; exit 1; }; echo "CHECK_FEATURES VERT 8/8"
F
cat > $W/scripts/cargo_magasin.sh <<'F'
#!/bin/bash
case "$*" in *--lib*) echo "test result: ok. 3 passed; 0 failed" ;; *) [ "${FAUX_PREUVE:-}" = muette ] || echo "    Checking rag3weaver v0.1.0" ;; esac
F
printf '#!/bin/sh\necho "${FAUX_HOTE:-lucie-tablette}"\n' > $D/bin/hostname
# L'étape des témoins navigateur : FAUX_NAV_OK=<nom>,… dit « ok » pour ces tests (sinon un seul test, « autre »).
cat > $W/scripts/temoins_navigateur.sh <<'F'
#!/bin/bash
for c in $(echo "${FAUX_NAV_OK:-autre}" | tr , ' '); do echo "test daemon_host::chat::tests::$c ... ok"; done
echo "test result: ok. 1 passed; 0 failed"
F
chmod +x $W/run_e2e.sh $W/scripts/*.sh $D/bin/hostname
rouge=0
passe() { # passe <attendu: VERT|ROUGE …|TEMOINS…> <description> [variables…] : joue la passe, juge la dernière ligne
  local attendu=$1 desc=$2; shift 2
  : > $D/trace; rm -rf $H/.cache/rag3weaver-build/j
  env "$@" HOME=$H PATH=$D/bin:$PATH FAUX_TRACE=$D/trace bash "$X" wt j filtre > /dev/null 2>&1
  local fin; fin=$(tail -1 $H/.cache/rag3weaver-build/j/resume.log 2>/dev/null)
  case "$fin" in "$attendu"*) echo "vert   $desc ($fin)" ;; *) echo "ROUGE  $desc : attendu « $attendu », lu « $fin »"; rouge=1 ;; esac
}
R=$H/.cache/rag3weaver-build/j/resume.log
passe VERT "la passe entière, tout vert"
grep -q '^commit [0-9a-f]\{7,\} — .* — lib 7c2b506ec' $R && echo "vert   la première ligne dit le commit et la lib" || { echo "ROUGE  première ligne : $(head -1 $R)"; rouge=1; }
grep -q 'LIB=rag3db-build-7c2b506ec74e79812df7960b220fc470d8449f14-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF ' $D/trace && echo "vert   les suites reçoivent la lib complète de la bascule" || { echo "ROUGE  la lib complète de la bascule manque aux suites"; rouge=1; }
[ "$(cut -d' ' -f1 $D/trace | tr '\n' ' ')" = "e2e_lectures e2e_lectures e2e_code " ] && echo "vert   le témoin, puis e2e_lectures et e2e_code" || { echo "ROUGE  suites jouées : $(cut -d' ' -f1 $D/trace | tr '\n' ' ')"; rouge=1; }
[ "$(cat $H/.cache/rag3weaver-build/j/base 2>/dev/null)" = "$BASE_JOUET" ] && echo "vert   la base testée est écrite pour la garde (<journaux>/base = master sous la tête)" || { echo "ROUGE  base testée : « $(cat $H/.cache/rag3weaver-build/j/base 2>/dev/null) », attendu $BASE_JOUET"; rouge=1; }
passe VERT "sous RAG3WEAVER_MOTEUR_ANCIEN=1" RAG3WEAVER_MOTEUR_ANCIEN=1
grep -q '^RAG3WEAVER_MOTEUR_ANCIEN=1 : le C++ du lot est plus neuf que la lib 7c2b506ec' $R && echo "vert   RAG3WEAVER_MOTEUR_ANCIEN est dit au résumé" || { echo "ROUGE  RAG3WEAVER_MOTEUR_ANCIEN n'est pas dit au résumé"; rouge=1; }
: > $D/trace; HOME=$H PATH=$D/bin:$PATH FAUX_TRACE=$D/trace bash "$X" wt j filtre temoins > /dev/null 2>&1
[ "$(tail -1 $R)" = "TEMOINS code=0" ] && [ "$(wc -l < $D/trace)" = 1 ] && echo "vert   « temoins » ne joue que le témoin" || { echo "ROUGE  témoin seul : $(tail -1 $R), $(wc -l < $D/trace) suite(s)"; rouge=1; }
passe "ROUGE e2e_code" "une suite rouge arrête la passe" FAUX_ROUGE=e2e_code
passe "ROUGE e2e_code : des suites non jouées" "une suite non jouée faute de feature est rouge" FAUX_NON_JOUEES=e2e_code
passe "ROUGE check_features" "check_features rouge" FAUX_CHECK=rouge
passe "ROUGE pas de Checking" "une preuve qui ne recompile rien est rouge" FAUX_PREUVE=muette
passe VERT "SUITES=- ne joue que le témoin, puis le reste" SUITES=-
[ "$(wc -l < $D/trace)" = 1 ] && echo "vert   SUITES=- : aucune suite après le témoin" || { echo "ROUGE  SUITES=- : $(wc -l < $D/trace) suites"; rouge=1; }
# Les services d'embarquement : jamais 7880 ni 7981, arrêtés depuis le 12 novembre.
for h in LuciePC lucie-tablette; do
  passe VERT "la passe sur $h" FAUX_HOTE=$h
  # Ports, never bare digits: the lib of 19 Nov. (7c2b506ec74e79812df…) holds « 7981 » in its hash.
  grep -qE ':(7880|7981)([^0-9]|$)' $D/trace && { echo "ROUGE  $h cite un service arrêté : $(head -1 $D/trace | grep -o 'EMBED=[^ ]*')"; rouge=1; } || echo "vert   $h ne cite ni 7880 ni 7981 ($(head -1 $D/trace | grep -o 'EMBED=[^ ]*'))"
done
# **Les rouges connus** (19 novembre 2026, forme de c7) : tools/poste/known_red_e2e.txt, dans l'arbre, une ligne par cas
# (suite, cas, ticket). La passe joue la suite entière ; un cas listé rouge est « rouge connu » avec son ticket, un cas listé
# vert est « à lever », tout autre rouge juge ; un ticket absent, fermé ou dans closed/ refuse la passe.
K=$H/git_workspaces/wt/tools/poste/known_red_e2e.txt; T=$H/git_workspaces/wt/docs/x/tickets
mkdir -p $T $H/git_workspaces/wt/docs/x/tickets/closed
printf -- '# t\n\n- **État** : ouvert (19 novembre 2026)\n' > $T/rouge.md
printf 'e2e_x\tcas_un\tdocs/x/tickets/rouge.md\n' > $K
passe VERT "un cas listé rouge ne rougit pas la passe" SUITES_EN_PLUS=e2e_x FAUX_CAS_ROUGES=e2e_x:cas_un
grep -q 'rouge connu (docs/x/tickets/rouge.md) : e2e_x::cas_un' $R && echo "vert   le rouge connu est dit avec son ticket" || { echo "ROUGE  le rouge connu n'est pas dit : $(grep -i connu $R)"; rouge=1; }
passe "ROUGE e2e_x" "un cas non listé rouge, à côté d'un listé, rougit la passe" SUITES_EN_PLUS=e2e_x FAUX_CAS_ROUGES=e2e_x:cas_un,cas_deux
passe VERT "un cas listé qui passe" SUITES_EN_PLUS=e2e_x FAUX_CAS_VERTS=e2e_x:cas_un
grep -q 'à lever : e2e_x::cas_un' $R && echo "vert   un cas listé vert est dit « à lever »" || { echo "ROUGE  le cas listé vert n'est pas dit à lever"; rouge=1; }
printf 'e2e_x\tcas_un\tdocs/x/tickets/absent.md\n' > $K
passe "ROUGE rouges connus" "un rouge connu au ticket absent refuse la passe" SUITES_EN_PLUS=e2e_x
printf 'e2e_x\tcas_un\tdocs/x/tickets/closed/rouge.md\n' > $K; cp $T/rouge.md $T/closed/rouge.md
passe "ROUGE rouges connus" "un rouge connu au ticket dans closed/ refuse la passe" SUITES_EN_PLUS=e2e_x
printf -- '# t\n\n- **État** : fermé (corrigé par abc)\n' > $T/rouge.md; printf 'e2e_x\tcas_un\tdocs/x/tickets/rouge.md\n' > $K
passe "ROUGE rouges connus" "un rouge connu au ticket fermé refuse la passe" SUITES_EN_PLUS=e2e_x
rm -f $K

# **Un témoin qui ne joue rien** (19 novembre 2026, c7) : un filtre qui ne désigne aucun test, ou un test ignoré, dit
# « 0 passed » avec le code 0. Le nom du témoin doit sortir « ok », sinon la passe rougit.
passe "ROUGE témoin" "un témoin qui ne joue aucun test (0 passed) rougit la passe" FAUX_TEMOIN_MUET=1
passe "ROUGE témoin" "sous SUITE_DU_TEMOIN=lib aussi" FAUX_TEMOIN_MUET=1 SUITE_DU_TEMOIN=lib
# **Le témoin navigateur** (19 novembre 2026, c7) : SUITE_DU_TEMOIN=navigateur, pour un lot dont le seul témoin est un test
# sous la feature `navigateur` (Chat.tsx) ; l'étape « témoin » ne le joue pas dans la lib (il y serait ignoré, donc
# « 0 passed » et vert sans avoir jugé), il se juge à l'étape des témoins navigateur, où son nom doit sortir « ok ».
passe VERT "SUITE_DU_TEMOIN=navigateur : le témoin se juge à l'étape navigateur" SUITE_DU_TEMOIN=navigateur FAUX_NAV_OK=filtre
grep -q 'témoin navigateur filtre : ok' $R && [ "$(cut -d' ' -f1 $D/trace | tr '\n' ' ')" = "e2e_lectures e2e_code " ] \
  && echo "vert   le témoin navigateur est dit ok, et l'étape lib ne le joue pas" || { echo "ROUGE  témoin navigateur : $(grep -i 'navigateur' $R | head -2 | tr '\n' ' ') ; suites : $(cut -d' ' -f1 $D/trace | tr '\n' ' ')"; rouge=1; }
passe "ROUGE témoin navigateur" "SUITE_DU_TEMOIN=navigateur : un témoin qui ne sort pas ok rougit" SUITE_DU_TEMOIN=navigateur FAUX_NAV_OK=autre

# Le repli des tests sur l'iGPU : à luciepc seulement, jamais au portable (son seul GPU porte l'écran), même posé par l'appelant.
passe VERT "le repli à luciepc" FAUX_HOTE=LuciePC
grep -q 'REPLI=igpu:0' $D/trace && echo "vert   LuciePC : RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=igpu:0" || { echo "ROUGE  LuciePC sans repli : $(head -1 $D/trace | grep -o 'REPLI=[^ ]*')"; rouge=1; }
passe VERT "pas de repli au portable" FAUX_HOTE=lucie-tablette RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=igpu:0
grep -q 'REPLI=absent' $D/trace && echo "vert   lucie-tablette : aucun repli, même posé par l'appelant" || { echo "ROUGE  le portable a un repli : $(head -1 $D/trace | grep -o 'REPLI=[^ ]*')"; rouge=1; }
# **Les suites qu'exige le lot** (16 novembre 2026, ticket de c7) : un lot qui touche code_sync.rs joue e2e_code_sync,
# même sous SUITES=-, et le résumé dit les suites exigées puis toutes celles jouées.
G="git -C $H/git_workspaces/wt -c user.email=t@t -c user.name=t"
echo '// sync' > $W/src/code_sync.rs; $G add $W/src/code_sync.rs > /dev/null; $G commit -qm sync
passe VERT "un lot qui touche code_sync.rs" SUITES=-
grep -q '^e2e_code_sync ' $D/trace && grep -q '^suites exigées par le lot : e2e_code_sync (extension/rag3weaver/src/code_sync.rs)' $R && grep -q '^suites jouées : .*e2e_code_sync' $R \
  && echo "vert   code_sync.rs touché : e2e_code_sync joué même sous SUITES=-, exigé et joué dits au résumé" \
  || { echo "ROUGE  code_sync.rs touché : suites $(cut -d' ' -f1 $D/trace | tr '\n' ' '), résumé « $(grep -E '^suites' $R | tr '\n' ' ') »"; rouge=1; }
$G reset -q --hard HEAD~1
passe VERT "un lot qui ne touche que lib.rs"
grep -q '^e2e_code_sync ' $D/trace && { echo "ROUGE  lib.rs seul : e2e_code_sync joué sans être exigé"; rouge=1; } || echo "vert   lib.rs seul : e2e_code_sync n'est pas exigé"
# **Un chemin absolu de journaux** (15 novembre 2026, ticket d'hotfixs) : `J=$HOME/.cache/rag3weaver-build/$2` l'emboîtait
# en silence ; le verdict s'écrivait là où personne (ni la garde du push) ne le lisait. Il est refusé, et la passe dit
# où elle écrit dès son premier pas.
A=$D/ailleurs/journaux-x
sortie=$(HOME=$H PATH=$D/bin:$PATH FAUX_TRACE=$D/trace bash "$X" wt "$A" filtre 2>&1); rc=$?
[ $rc != 0 ] && [ ! -e "$A" ] && [ ! -e "$H/.cache/rag3weaver-build$A" ] && echo "$sortie" | grep -q 'chemin absolu' \
  && echo "vert   un chemin absolu de journaux est refusé, nommé, sans dossier créé ni emboîté (code $rc)" \
  || { echo "ROUGE  chemin absolu : code $rc, emboîté $([ -e "$H/.cache/rag3weaver-build$A" ] && echo oui || echo non), sortie « $(echo "$sortie" | head -2 | tr '\n' ' ') »"; rouge=1; }
sortie=$(HOME=$H PATH=$D/bin:$PATH FAUX_TRACE=$D/trace bash "$X" wt j filtre temoins 2>&1)
[ "$(echo "$sortie" | head -1)" = "journaux : $H/.cache/rag3weaver-build/j" ] \
  && echo "vert   la passe dit où elle écrit dès son premier pas" \
  || { echo "ROUGE  premier pas : « $(echo "$sortie" | head -1) »"; rouge=1; }
# **Un test de la lib comme témoin** (15 novembre 2026, ticket d'hotfixs) : SUITE_DU_TEMOIN=lib joue le témoin par
# `run_e2e.sh --lib <filtre>`, et la lib n'est pas rejouée comme une suite e2e ensuite.
passe VERT "SUITE_DU_TEMOIN=lib" SUITE_DU_TEMOIN=lib
[ "$(cut -d' ' -f1 $D/trace | tr '\n' ' ')" = "lib e2e_lectures e2e_code " ] && echo "vert   SUITE_DU_TEMOIN=lib : le témoin par --lib, puis e2e_lectures et e2e_code" || { echo "ROUGE  SUITE_DU_TEMOIN=lib, suites jouées : $(cut -d' ' -f1 $D/trace | tr '\n' ' ')"; rouge=1; }
# **Un test non jugé n'est pas un test passé** (18 novembre 2026, ticket de c7) : un test qui dit « non jugé : … » et rend
# la main compte parmi les passés de cargo. Le résumé les liste à part, et un non-jugé du témoin ou d'une suite exigée
# rend la passe rouge.
passe VERT "un non-jugé dans e2e_code, suite non exigée" FAUX_NON_JUGE=e2e_code
grep -q "^non jugés dans e2e_code : 1 — non jugé : la lib n'a pas la colonne state" $R && grep -q '^tests non jugés : 1$' $R \
  && echo "vert   un non-jugé est listé à part au résumé, avec sa raison, et compté" \
  || { echo "ROUGE  non-jugé d'e2e_code : résumé « $(grep -iE 'non jug' $R | tr '\n' ' ') »"; rouge=1; }
passe "ROUGE témoin : 1 test non jugé" "un non-jugé dans le témoin rend la passe rouge" FAUX_NON_JUGE=e2e_lectures
passe VERT "aucun non-jugé"
grep -q '^tests non jugés : 0$' $R && echo "vert   sans non-jugé, le résumé dit 0" || { echo "ROUGE  sans non-jugé : « $(grep -iE 'non jug' $R | tr '\n' ' ') »"; rouge=1; }
exit $rouge
