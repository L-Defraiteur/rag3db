from pathlib import Path
import json,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-sonde-cargo-union-v1';record=dict(verdict='ROUGE',client_rc=int(sys.argv[1]))
try:
 assert record['client_rc']==0 and (P/'VERDICT_SONDE').read_text().strip()=='OBSERVATION_CARGO_SANS_COMPILATION'
 source=json.loads((P/'source.json').read_text());receipt=Path(source['receipt']);scope=json.loads((receipt.parent/'verdict.json').read_text())
 assert scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 assert time.monotonic()<source['scope_deadline']<=source['original_deadline']
 probe=json.loads((P/'verdict-sonde.json').read_text());assert probe['target_unchanged'] and probe['not_f8_qualified']
 assert (P/'cargo-env-observe.json').exists()
 record.update(verdict='OBSERVATION_VALIDEE_SEULEMENT',scope=scope,original_deadline=source['original_deadline'],scope_deadline=source['scope_deadline'],f8_qualified=False)
except BaseException as error:record['error']=repr(error)
finally:
 if record['verdict']=='OBSERVATION_VALIDEE_SEULEMENT' and time.monotonic()>=record['scope_deadline']:record.update(verdict='ROUGE',error='deadline before publication')
 (P/'verdict-final-sonde.json').write_text(json.dumps(record,indent=2));print(json.dumps(record))
sys.exit(0 if record['verdict']=='OBSERVATION_VALIDEE_SEULEMENT' else 1)
