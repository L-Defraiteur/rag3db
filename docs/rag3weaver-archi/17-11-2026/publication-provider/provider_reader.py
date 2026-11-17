"""Reader and business validation only. No lifecycle, clock admission, export or ACK.

A returned record is not physical permission. The common Gate must independently
validate the owner/window and active original authority before any publication.
The native controller 229b and its copied Q are intentionally untouched.
"""
from pathlib import Path
import hashlib,json,math,os,re,stat

LIMIT=16*1024*1024
WINDOW_LIMIT=65536
LABELS=('global','agent-red1','agent-green1','review1')

def require(ok,message):
 if not ok:raise ValueError(message)

def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()

def digest(raw):return hashlib.sha256(raw).hexdigest()

def decode(raw):
 def pairs(items):
  value={}
  for key,item in items:
   require(key not in value,'duplicate JSON key');value[key]=item
  return value
 def invalid(value):raise ValueError('nonfinite JSON constant')
 try:return json.loads(raw,object_pairs_hook=pairs,parse_constant=invalid)
 except (UnicodeError,json.JSONDecodeError) as error:raise ValueError('invalid JSON') from error

def leaf(name):
 require(type(name) is str and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,159}',name) is not None,'unsafe private file ID')
 return name

def finite(value):
 require(type(value) in (int,float) and math.isfinite(value) and value>=0,'invalid monotonic value');return value

def sha(value):require(type(value) is str and re.fullmatch('[0-9a-f]{64}',value) is not None,'invalid SHA256');return value

class Reader:
 def __init__(self,config):
  self._directory_fds=[];self._root_fd=None
  self._config=canonical(config) # Own immutable bytes: caller mappings cannot mutate expectations.
  value=decode(self._config)
  require(value.get('schema_version')==1 and value.get('kind')=='archi-publication-reader','reader schema')
  for key in ['source','pilot']:require(re.fullmatch('[0-9a-f]{40}',value.get(key,'')) is not None,'full source/pilot pin required')
  for key in ['execution_id','phase_id']:require(type(value.get(key)) is str and value[key],'execution/phase missing')
  for key in ['prepared_sha256','plan_sha256']:sha(value[key])
  root=Path(value['root']);require(root.is_absolute() and root.resolve()==root,'private publication root must be canonical')
  self._directory_chain=[]
  for path in reversed((root,*root.parents)):
   info=path.lstat();require(stat.S_ISDIR(info.st_mode),'noncanonical directory ancestor')
   fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC);self._directory_fds.append(fd)
   identity=self._directory_identity(info);require(self._directory_identity(os.fstat(fd))==identity,'directory changed during anchoring')
   self._directory_chain.append((path,fd,identity))
  self._root_fd=self._directory_fds[-1]
  require(type(value['owner_uid']) is int and value['owner_uid']>=0,'invalid owner')
  labels=[s['label'] for s in value['stages']];require(labels and len(set(labels))==len(labels) and all(n in LABELS for n in labels),'unknown or duplicate stage')
  for index,stage in enumerate(value['stages']):
   require(type(stage['sequence']) is int and stage['sequence']==index,'noncontiguous stage order');require(stage['binding_sha256']==digest(canonical(value['bindings'][stage['label']])),'source contract hash differs')
   for key in ['window_file','ledger_file']:leaf(stage[key])
   for name in stage['artifact_files'].values():leaf(name)
   require(set(stage['artifacts'])==set(stage['artifact_files']) and set(stage['files'])=={'ledger',*stage['artifacts']},'declared file/role set differs')
   require(value['bindings'][stage['label']]['source']==value['source'],'source binding differs from APP')
   require(set(value['bindings'][stage['label']]['roles'])==set(stage['artifacts']),'source contract roles differ')
   if stage['label']!='global':
    expected=value.get('broker_sources',{}).get(stage['label']);require(type(expected) is dict and set(expected)=={'cwd','manifest'},'independent Broker source rule missing')
    cwd=Path(expected['cwd']);require(cwd.is_absolute() and '..' not in cwd.parts and str(cwd)==expected['cwd'] and expected['manifest']==str(cwd/'Cargo.toml'),'independent Broker manifest rule invalid')
  self._verify_root()
 @staticmethod
 def _directory_identity(info):return (info.st_dev,info.st_ino,info.st_uid,info.st_mode)
 def _verify_root(self):
  require(self._root_fd is not None,'reader directory handles closed')
  for path,fd,identity in self._directory_chain:
   require(self._directory_identity(path.lstat())==identity and self._directory_identity(os.fstat(fd))==identity,'publication root or canonical parent changed')
 def close(self):
  descriptors=self._directory_fds;self._directory_fds=[];self._root_fd=None
  for fd in reversed(descriptors):os.close(fd)
 def __del__(self):
  for fd in reversed(getattr(self,'_directory_fds',[])):
   try:os.close(fd)
   except OSError:pass
 def _open(self,name):
  config=decode(self._config);leaf(name);self._verify_root()
  require(os.fstat(self._root_fd).st_uid==config['owner_uid'],'publication directory owner differs')
  fd=os.open(name,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=self._root_fd)
  try:
   info=os.fstat(fd);require(stat.S_ISREG(info.st_mode) and info.st_uid==config['owner_uid'],'publication file is not regular/owned');self._verify_root()
   return fd,info
  except BaseException:
   os.close(fd);raise
 def _bytes(self,name,limit):
  fd,before=self._open(name)
  with os.fdopen(fd,'rb') as stream:
   require(before.st_size<=limit,'publication JSON too large');raw=stream.read(limit+1);after=os.fstat(stream.fileno())
   require(len(raw)<=limit and (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns,before.st_ctime_ns)==(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns,after.st_ctime_ns),'publication file changed during read')
  self._verify_root();return raw
 def _artifact(self,name):
  fd,info=self._open(name)
  try:
   self._verify_root();return dict(size=info.st_size,mode=stat.S_IMODE(info.st_mode))
  finally:os.close(fd)
 def read(self,label,phase):
  config=decode(self._config)
  require(label in [s['label'] for s in config['stages']],'unknown stage')
  for key in ['execution_id','phase_id','prepared_sha256']:require(getattr(phase,key,None)==config[key],'common phase binding differs')
  stage=next(s for s in config['stages'] if s['label']==label);binding=config['bindings'][label];root=Path(config['root'])
  window_raw=self._bytes(stage['window_file'],WINDOW_LIMIT);window=decode(window_raw)
  require(window.get('schema_version')==1,'window schema')
  for key in ['source','pilot','execution_id','phase_id','prepared_sha256','plan_sha256']:require(window.get(key)==config[key],'owner event binding differs: '+key)
  require(window.get('label')==label and window.get('sequence')==stage['sequence'] and window.get('binding_sha256')==stage['binding_sha256'],'owner event stage/contract differs')
  opened=finite(window['opened_monotonic']);end=finite(window['deadline_monotonic']);operation=finite(window['operation_end_monotonic'])
  require(opened<end<=min(opened+90,operation),'owner event extends declared window')
  owner=window.get('owner',{});require(type(owner.get('pid')) is int and owner['pid']>0 and type(owner.get('start')) is str and owner['start'].isdigit(),'owner event identity missing');sha(owner['script_sha256'])
  ledger_name=stage['ledger_file'];artifact_names=dict(stage['artifact_files']);admission=None;admission_raw=None;admission_name=None
  if label!='global':
   require(stage.get('broker_bound') is True and ledger_name=='broker-ledger' and set(artifact_names.values())=={'broker-elf'},'Broker path rule not explicitly declared')
   identity=window.get('admission_identity');require(type(identity) is str and re.fullmatch(r'task-[1-9][0-9]*-[0-9]+',identity) is not None,'unbound Broker admission')
   admission_name=identity+'-admission.json';admission_raw=self._bytes(admission_name,LIMIT);admission=decode(admission_raw);ledger_name=identity+'-ledger.json';artifact_names={aid:identity+'-elf' for aid in stage['artifacts']}
   require(admission['identity']==identity,'another Broker identity')
  ledger_raw=self._bytes(ledger_name,LIMIT);ledger=decode(ledger_raw);require(digest(ledger_raw)==sha(window['ledger_sha256']),'original ledger byte hash differs')
  require(ledger.get('source')==config['source'] and ledger.get('pilot')==config['pilot'],'ledger source/pilot differs')
  require(finite(ledger['created'])<=opened,'ledger appeared after owner event')
  roles=binding['roles'];expected_keys={r.get('ledger_key',aid) for aid,r in roles.items()};require(set(ledger['artifacts'])==expected_keys,'missing or extra executable occurrence')
  files={'ledger':dict(path=str(root/ledger_name),sha256=digest(ledger_raw),size=len(ledger_raw))};elf_map={}
  if label!='global':
   expected_stage={'agent-red1':'agent-red','agent-green1':'agent-green','review1':'review'}[label]
   require(admission['identity']==identity and admission['stage']==expected_stage and ledger['stage']==expected_stage,'another Broker stage')
   require(ledger['source_head']==admission['source_head']==ledger['source_state']['head']==admission['source_state']['head']==binding['source']==config['source'],'APP/admission/ledger/state HEAD chain differs')
   require(ledger['source_state']==admission['source_state'],'ledger source state differs from admission')
   require(admission['cwd']==config['broker_sources'][label]['cwd'],'Broker cwd differs from independent expected source')
   require(ledger_name==identity+'-ledger.json','ledger is not the unique admitted identity')
   require(ledger['source_patch_sha256']==admission['patch_sha256'],'source patch differs')
  for aid,role in roles.items():
   item=ledger['artifacts'][role.get('ledger_key',aid)];name=artifact_names[aid]
   require(item['copy']==str(root/name),'artifact copy path differs from fixed contract')
   artifact=item.get('artifact',item.get('cargo_artifact'));require(type(artifact) is dict,'real CargoJSON artifact missing')
   require(artifact['target']['name']==role['target_name'] and artifact['target']['kind']==[role['target_kind']],'Cargo target kind/name differs')
   require(artifact['profile']['test'] is role['profile_test'],'Cargo profile differs')
   require(sorted(artifact['features'])==sorted(item['features'])==sorted(role['features']),'Cargo feature closure differs')
   manifest=config['manifest_paths'][role['manifest']] if admission is None else config['broker_sources'][label]['manifest']
   require(artifact['manifest_path']==manifest,'Cargo artifact from another manifest')
   size=self._artifact(name)['size'];require(type(item['size']) is int and item['size']==size,'ELF size differs')
   files[aid]=dict(path=str(root/name),sha256=sha(item['sha256']),size=size);elf_map[aid]=item['sha256']
  # Re-read immutable inputs; this layer neither hashes ELF content nor authorizes it.
  require(self._bytes(stage['window_file'],WINDOW_LIMIT)==window_raw and self._bytes(ledger_name,LIMIT)==ledger_raw,'publication metadata changed before returning')
  if admission_name is not None:require(self._bytes(admission_name,LIMIT)==admission_raw,'Broker admission bytes changed during read')
  self._verify_root()
  return dict(label=label,sequence=stage['sequence'],binding_sha256=stage['binding_sha256'],ledger=ledger,ledger_bytes_sha256=digest(ledger_raw),window=window,window_sha256=digest(window_raw),files=files,elf_map=elf_map,admission=admission,admission_bytes_sha256=None if admission_raw is None else digest(admission_raw),physical_admission=False)
