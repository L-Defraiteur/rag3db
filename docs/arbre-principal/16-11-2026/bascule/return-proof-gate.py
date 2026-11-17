"""Read and bind a versioned RET_v2 filesystem qualification; never execute a restore."""
from pathlib import Path
import hashlib,json,re,stat

def digest(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()

def validate_record(record,base,original_path,original_sha,snapshot_root,snapshot_tree):
 if record.get('schema_version')!=1 or record.get('verdict')!='VERT' or record.get('qualification_scope')!='private_filesystem_fixtures_no_real_restore':raise ValueError('return qualification scope invalid')
 if record.get('base')!=base or record.get('original_receipt')!=dict(path=str(original_path),sha256=original_sha):raise ValueError('return proof refers to another base/receipt')
 if record.get('snapshot_root')!=str(snapshot_root) or record.get('snapshot_inventory',{}).get('tree_sha256')!=snapshot_tree:raise ValueError('return proof refers to another snapshot')
 for name in ['original_receipt_unchanged','snapshot_data_unchanged']:
  if record.get(name) is not True:raise ValueError('return original identity not preserved')
 if record.get('live_restore_executed') is not False or record.get('cli_authorized') is not False:raise ValueError('return fixture proof misrepresents live authority')
 retention=record.get('retention_hours_minimum')
 if type(retention) is not int or retention<72:raise ValueError('return retention below 72 hours')
 suite=record.get('fixture_suite',{})
 if type(suite.get('passed')) is not int or suite['passed']<5 or type(suite.get('failed')) is not int or suite['failed']!=0 or type(suite.get('skipped')) is not int or suite['skipped']!=0:raise ValueError('return fixtures not complete green')
 for entry in [record.get('implementation',{}),suite]:
  if not re.fullmatch('[0-9a-f]{40}',entry.get('git_commit','')) or not re.fullmatch('[0-9a-f]{64}',entry.get('sha256','')):raise ValueError('return source provenance invalid')
  repo=Path(entry.get('repository_path',''))
  if repo.is_absolute() or '..' in repo.parts or not repo.parts:raise ValueError('invalid return repository path')
 script=Path(record.get('return_script',{}).get('path',''))
 if script.parent!=Path(snapshot_root) or script.name!='RETOUR_v2.sh':raise ValueError('return launcher path is not the versioned snapshot launcher')
 return record

def validate_fixture_log(text,passed):
 names=re.findall(r'^(test_[^ \n]+) \([^\n]+\) \.\.\. ok$',text,re.M)
 run=re.search(r'^Ran ([0-9]+) tests? in ',text,re.M)
 if len(names)!=passed or len(set(names))!=passed or run is None or int(run[1])!=passed or not text.strip().endswith('\nOK'):raise ValueError('raw return fixture log disagrees with counters')
 if 'test_future_wal_and_all_new_neighbours_leave_the_live_set' not in names:raise ValueError('future WAL refusal/return witness absent')
 return names

def safe_file(path):
 path=Path(path)
 if any(part=='.vault' or part.startswith('.env') for part in path.parts) or not stat.S_ISREG(path.lstat().st_mode):raise ValueError('unsafe return proof file')
 return path

def pinned_file(path,sha):
 path=safe_file(path)
 if digest(path)!=sha:raise ValueError('return proof file changed: '+str(path))

def verify_attestation(path,sha,base,original_path,original_sha,snapshot_root,snapshot_tree):
 pinned_file(path,sha);record=json.loads(Path(path).read_text())
 validate_record(record,base,original_path,original_sha,snapshot_root,snapshot_tree)
 pinned_file(original_path,original_sha)
 for entry in [record['snapshot_inventory'],record['return_script']]:pinned_file(entry['path'],entry['sha256'])
 implementation=record['implementation'];pinned_file(implementation['installed_path'],implementation['sha256'])
 suite=record['fixture_suite'];pinned_file(suite['green_log_path'],suite['green_log_sha256'])
 names=validate_fixture_log(Path(suite['green_log_path']).read_text(),suite['passed'])
 legacy=safe_file(suite['legacy_red_log_path']);legacy_text=legacy.read_text()
 if 'FAILED' not in legacy_text:raise ValueError('legacy red return witness absent')
 return dict(attestation_path=str(path),attestation_sha256=sha,return_script=record['return_script'],implementation=implementation,fixture_suite=suite,fixture_names=names,legacy_red_log_sha256=digest(legacy),qualification_scope=record['qualification_scope'],live_restore_executed=False)
