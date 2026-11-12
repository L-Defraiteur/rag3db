from pathlib import Path
import json,mmap,os,subprocess,sys
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-publication-composite';C=W/'extension/rag3weaver';P=R/'cx-publication-composite-probe-maps-v8'
SOURCE=sys.argv[1]
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip()==SOURCE
assert not subprocess.check_output(['git','status','--porcelain'],cwd=W,text=True).strip()
P.mkdir(exist_ok=False)
a=json.loads((R/'cx-passe-publication-composite-v3/artefacts.json').read_text())['backend'];binary=Path(a['actual_path'])
with binary.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as raw:assert raw.find(b'626d050d3')>=0
E=dict(os.environ,RAG3DB_LIB='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF',RAG3WEAVER_MOTEUR_ANCIEN='1',CARGO_MANIFEST_DIR=str(C),RAG3WEAVER_JOURNAUX_DIR=str(P/'commandes'),RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(P/'builtin-cache'))
subprocess.run(['bash','-c','. scripts/lib_du_magasin.sh; exporter_la_lib || exit 2; exec "$@"','native-probe','python3',str(Path(__file__).with_name('backend_maps.py')),str(binary),str(P)],env=E,cwd=C,check=True)
print('PROBE_MAPS_BACKEND VERT (ELF626 existant, inspection metadata absente uniquement)',flush=True)
