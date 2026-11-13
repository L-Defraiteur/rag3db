#!/bin/bash
# Exécution seule : mêmes primitives de fixture que run_e2e, aucun Cargo.
set -euo pipefail
bin="$1"; digest="$2"; preuve="$3"
WEAVER="$CARGO_MANIFEST_DIR"
python3 "$WEAVER/scripts/confiner_validation.py" verifier
python3 - "$bin" "$digest" "$preuve" <<'PY'
import hashlib,json,pathlib,subprocess,sys
b=pathlib.Path(sys.argv[1]);assert hashlib.file_digest(b.open('rb'),'sha256').hexdigest()==sys.argv[2]
o=pathlib.Path(sys.argv[3]);source=json.loads((o/'debut.json').read_text())['source']
s=pathlib.Path(__import__('os').environ['CARGO_MANIFEST_DIR'])/'run_e2e.sh'
tracked=subprocess.check_output(['git','-C',str(s.parent),'show',source+':extension/rag3weaver/run_e2e.sh']);assert tracked==s.read_bytes()
(o/'hote-primitives-source.sha256').write_text(hashlib.sha256(tracked).hexdigest()+'\n')
PY
for nom in creer_etat_suite_e2e terminer_etat_suite_e2e residus_de_la_suite preparer_journaux_des_demons; do
  definition=$(sed -n "/^$nom() {/,/^}/p" "$WEAVER/run_e2e.sh")
  test -n "$definition"
  eval "$definition"
done
export RAG3WEAVER_PASSE_E2E="$$-$(date +%s%N)"
E2E_TMP_PARENT="$TMPDIR/rag3weaver-e2e-$$"
E2E_DISQUE_PARENT="$HOME/.cache/rag3weaver-build/e2e-$$"
LISTE_DES_RESIDUS="$WEAVER/tests/e2e/suites_qui_laissent_des_residus.txt"
TEST_FILTER=''
test ! -e "$E2E_TMP_PARENT"
test ! -e "$E2E_DISQUE_PARENT"
mkdir -p "$E2E_TMP_PARENT/e2e_demon_hote" "$E2E_DISQUE_PARENT/e2e_demon_hote/checkpoints"
preparer_journaux_des_demons
"$bin" --exact chaque_suite_est_montee aucune_suite_ne_se_relance_par_son_nom_court --include-ignored
export RAG3WEAVER_E2E_DOSSIER="$E2E_DISQUE_PARENT/e2e_demon_hote"
export RAG3WEAVER_CHECKPOINT_DIR="$RAG3WEAVER_E2E_DOSSIER/checkpoints"
export RAG3WEAVER_CACHE_DIR="$RAG3WEAVER_E2E_DOSSIER/cache"
etat=$(creer_etat_suite_e2e "$E2E_TMP_PARENT/e2e_demon_hote")
set +e
TMPDIR="$E2E_TMP_PARENT/e2e_demon_hote" RAG3WEAVER_STATE_DIR="$etat" "$bin" e2e_demon_hote:: --include-ignored --nocapture
rc=$?
set -e
if [ "$rc" -ne 0 ]; then exit "$rc"; fi
terminer_etat_suite_e2e "$etat" 0 1 "$RAG3WEAVER_E2E_DOSSIER/etat-temoins"
residus_de_la_suite e2e_demon_hote
python3 - "$preuve" "$E2E_TMP_PARENT" "$E2E_DISQUE_PARENT" <<'PY'
import json,pathlib,sys
p=pathlib.Path(sys.argv[2])/'e2e_demon_hote';assert p.is_dir() and not any(p.iterdir())
(pathlib.Path(sys.argv[1])/'gardes-hote13.json').write_text(json.dumps({'verdict':'VERT','tmp_parent':sys.argv[2],'disque_parent':sys.argv[3],'etat_possede_archive':True,'tmp_propre':True,'primitives_reelles':'run_e2e.sh'},indent=2))
PY
