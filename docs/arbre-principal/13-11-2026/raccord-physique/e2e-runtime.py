#!/usr/bin/env python3
"""Tracked run_e2e runtime primitives on an already attested ELF, without Cargo.

No compiler artifacts are simulated. Only the compilation stanza is removed;
state/residue/filets/suite execution keep the actual tracked implementation.
"""
from pathlib import Path
import hashlib,json,os,re,shlex,subprocess,sys


def functions(crate,source,binary,proof,suite):
    assert re.fullmatch(r'e2e_[a-z0-9_]+',suite)
    raw=(crate/'run_e2e.sh').read_bytes()
    tracked=subprocess.check_output(['git','-C',str(crate),'show',source+':extension/rag3weaver/run_e2e.sh'])
    assert tracked==raw
    text=raw.decode();parts=[];records={}
    names=['preparer_journaux_des_demons','pids_des_demons','arreter_les_demons_de_la_passe','archiver_journaux_des_demons','creer_etat_suite_e2e','terminer_etat_suite_e2e','residus_de_la_suite','jouer_les_suites']
    for name in names:
        found=re.search(r'(?ms)^'+name+r'\(\) \{\n.*?^\}',text)
        assert found,name
        original=found.group();body=original
        if name=='jouer_les_suites':
            start=body.index('  # 1. Bâtir une fois :')
            end=body.index('  # 2. Les filets',start)
            body=body[:start]+'''  # Canonical ELF already built and rehashed. No Cargo is executed here.
  [ "${#autres[@]}" -eq 0 ] && [ "${#e2e[@]}" -eq 1 ] || return 101
  bin="$CANONICAL_E2E"
  [ -x "$bin" ] || return 101
  echo "RUNTIME_ELF_CANONIQUE $bin (compilation antérieure, aucun Cargo)"

'''+body[end:]
        parts.append(body)
        records[name]=dict(original_sha256=hashlib.sha256(original.encode()).hexdigest(),runtime_sha256=hashlib.sha256(body.encode()).hexdigest(),changed=(body!=original))
    q=shlex.quote
    prelude='\n'.join([
        '#!/bin/bash','set -u','WEAVER='+q(str(crate)),
        'CANONICAL_E2E='+q(str(binary)),
        'FEATURES='+q(os.environ.get('RAG3WEAVER_E2E_FEATURES','')),
        'CARGO_ARGS=(--test '+q(suite)+')','EXTRA_ARGS=(--test-threads=8)','TEST_FILTER=""',
        'LISTE_DES_RESIDUS="$WEAVER/tests/e2e/suites_qui_laissent_des_residus.txt"',
        'E2E_DISQUE_RACINE='+q(str(proof/'runtime-disk')),
        'E2E_TMP_PARENT="${TMPDIR:?}/rag3weaver-e2e-$$"',
        'E2E_DISQUE_PARENT="$E2E_DISQUE_RACINE/e2e-$$"',
        'export RAG3WEAVER_PASSE_E2E="$$-$(date +%s%N)"',
        'export RAG3WEAVER_REGIME="${RAG3WEAVER_REGIME:-confort}"',
        'confined() { [ "${1##*/}" != cargo ] || { echo "CARGO_INTERDIT_RUNTIME" >&2; return 101; }; "$@"; }',
    ])+'\n'
    tail='''
mkdir -p "$E2E_TMP_PARENT" "$E2E_DISQUE_PARENT"
preparer_journaux_des_demons
jouer_les_suites
rc=$?
# Stop only processes carrying this exact pass marker, via tracked guards.
if ! arreter_les_demons_de_la_passe || ! archiver_journaux_des_demons; then rc=101; fi
# Tracked suite code already checks state/TMP residues. Keep owned parent
# directories for review; no target or foreign-parent cleanup is performed.
printf 'RUNTIME_PRIMITIVES_FIN %s code=%s\n' SUITE "$rc"
exit "$rc"
'''.replace('SUITE',q(suite))
    return prelude+'\n'.join(parts)+'\n'+tail,dict(source=source,run_e2e_sha256=hashlib.sha256(raw).hexdigest(),functions=records,only_compilation_stanza_removed=True,cargo_calls_refused=True,full_run_e2e_script_executed=False,suite=suite,binary=str(binary))


def main():
    binary=Path(sys.argv[1]);expected=sys.argv[2];source=sys.argv[3];proof=Path(sys.argv[4]);suite=sys.argv[5]
    crate=Path(os.environ['CARGO_MANIFEST_DIR'])
    subprocess.run(['python3',str(crate/'scripts/confiner_validation.py'),'verifier'],check=True)
    assert hashlib.file_digest(binary.open('rb'),'sha256').hexdigest()==expected
    assert subprocess.check_output(['git','-C',str(crate),'rev-parse','HEAD'],text=True).strip()==source
    body,metadata=functions(crate,source,binary,proof,suite)
    path=proof/('runtime-primitives-'+suite+'.sh');assert not path.exists()
    path.write_text(body);metadata.update(elf_sha256=expected,generated_sha256=hashlib.sha256(body.encode()).hexdigest())
    (proof/('runtime-primitives-'+suite+'.json')).write_text(json.dumps(metadata,indent=2))
    os.execvpe('bash',['bash',str(path)],os.environ.copy())
if __name__=='__main__':main()
