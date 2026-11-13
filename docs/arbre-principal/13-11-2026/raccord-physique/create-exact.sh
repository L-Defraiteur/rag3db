#!/usr/bin/env bash
# Même ELF et même profil privé que la suite : seul ANCIEN est absent.
set -euo pipefail
bin="$1"; digest="$2"; preuve="$3"
python3 "$CARGO_MANIFEST_DIR/scripts/confiner_validation.py" verifier
python3 - "$bin" "$digest" "$preuve" <<'PY'
from pathlib import Path
import hashlib,sys,os,json,shutil,subprocess
b=Path(sys.argv[1]);assert hashlib.file_digest(b.open('rb'),'sha256').hexdigest()==sys.argv[2]
o=Path(sys.argv[3]);profile=json.loads((o/'profil-lectures.json').read_text())['env']
assert 'RAG3WEAVER_MOTEUR_ANCIEN' not in os.environ
for k,v in profile.items():
 if k!='RAG3WEAVER_MOTEUR_ANCIEN':assert os.environ.get(k)==v,(k,os.environ.get(k),v)
root=o/'runtime-disk';pid=os.environ['RAG3WEAVER_PASSE_E2E'].split('-')[0];assert pid.isdigit()
tmp=Path(os.environ['TMPDIR']);disk=Path(os.environ['RAG3WEAVER_E2E_DOSSIER']);state=Path(os.environ['RAG3WEAVER_STATE_DIR'])
assert tmp.is_relative_to(o/'tmp') and tmp.parent.name=='rag3weaver-e2e-'+pid and tmp.name=='e2e_lectures'
assert disk==root/('e2e-'+pid)/'e2e_lectures'
assert Path(os.environ['RAG3WEAVER_CHECKPOINT_DIR'])==disk/'checkpoints'
assert Path(os.environ['RAG3WEAVER_CACHE_DIR'])==disk/'cache'
assert state.parent==tmp and state.name.startswith('etat-temoins.')
for p in [tmp,disk,disk/'etat-temoins']:
 assert p.resolve()==p.absolute() and p.is_dir(),p
assert not state.exists() and not any(tmp.iterdir()),'TMPDIR résiduel avant complément'
archive=disk/'etat-temoins';assert (archive/'.possede-par-le-lanceur-e2e').read_text().strip()==str(state)
shutil.move(str(archive),str(state))
script=Path(os.environ['CARGO_MANIFEST_DIR'])/'run_e2e.sh'
tracked=subprocess.check_output(['git','-C',str(script.parent),'show',json.loads((o/'source.json').read_text())['source']+':extension/rag3weaver/run_e2e.sh'])
assert tracked==script.read_bytes()
(o/'profil-complement-demande.json').write_text(json.dumps({k:os.environ[k] for k in profile if k!='RAG3WEAVER_MOTEUR_ANCIEN'},indent=2))
(o/'run_e2e-primitives-source.sha256').write_text(hashlib.sha256(tracked).hexdigest()+'\n')
PY
for nom in terminer_etat_suite_e2e residus_de_la_suite; do
  definition=$(sed -n "/^$nom() {/,/^}/p" "$CARGO_MANIFEST_DIR/run_e2e.sh")
  test -n "$definition"
  eval "$definition"
done
E2E_TMP_PARENT="$(dirname "$TMPDIR")"
LISTE_DES_RESIDUS="$CARGO_MANIFEST_DIR/tests/e2e/suites_qui_laissent_des_residus.txt"
TEST_FILTER='la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel'
set +e
"$bin" --exact e2e_lectures::la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel --include-ignored --nocapture
rc=$?
set -e
if test "$rc" -ne 0; then exit "$rc"; fi
terminer_etat_suite_e2e "$RAG3WEAVER_STATE_DIR" 0 1 "$RAG3WEAVER_E2E_DOSSIER/etat-temoins"
residus_de_la_suite e2e_lectures
python3 - "$preuve" <<'PY'
from pathlib import Path
import os,sys,json
p=Path(os.environ['TMPDIR']);assert p.is_dir() and not any(p.iterdir())
(Path(sys.argv[1])/'gardes-complement.json').write_text(json.dumps(dict(verdict='VERT',autorite_verifiee=True,profil_identique_sauf_ancien=True,etat_possede_archive=True,tmpdir_propre=True,primitives_reelles='run_e2e.sh'),indent=2))
PY
