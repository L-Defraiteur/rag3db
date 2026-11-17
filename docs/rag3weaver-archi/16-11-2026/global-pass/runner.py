"""One bounded subprocess tree under the shared confiner; explicit active logs."""
from pathlib import Path
import json,os,re,shutil,subprocess,time,threading
import evidence,policy
from settings import *
FAILED=re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.|^(?:CHECK_FEATURES )?ROUGE\b',re.M)

class Runner:
 def __init__(self,config,end,cgroup):
  self.config=config;self.end=end;self.cgroup=cgroup;self.observer=None;self.env={};self.artifacts={};self.inventories={};self.phase='preflight';self.last_storage=0.;self.ledgers=[];self.probes=[];self._tick_lock=threading.RLock();self.phase_deadline=end;self.started_phases=[]
 def save(self,name,value):evidence.save(P/name,value)
 def fail(self,reason,payload=None):
  try:self.save('first-red.json',dict(phase=self.phase,reason=reason,payload=payload,observed=time.monotonic()))
  except FileExistsError:pass
 def start_phase(self,name,cap):
  policy.require(name not in self.started_phases,'phase cap cannot be reset')
  self.started_phases.append(name)
  self.phase=name;self.phase_deadline=policy.deadline(time.monotonic(),self.end,cap)
  if self.observer:self.observer.phase=name
 def git(self,root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
 def tick(self):
  with self._tick_lock:self._tick()
 def _tick(self):
  for probe in self.probes:probe()
  policy.require(time.monotonic()<self.phase_deadline,'phase deadline')
  policy.require(time.monotonic()<self.end,'common/effective deadline')
  if self.observer:self.observer.tick()
  if time.monotonic()-self.last_storage>1:
   owned_disks=[]
   if self.observer:
    parent_ids={row['pid'] for row in self.observer.frames.values() if any(a.endswith('/run_e2e.sh') for a in row.get('argv',[]))}
    for path in R.glob('e2e-[0-9]*'):
     if not path.is_dir():continue
     policy.require(path.name.removeprefix('e2e-').isdigit() and int(path.name.removeprefix('e2e-')) in parent_ids,'foreign e2e parent: canonical sweep would touch another scope')
     policy.require(path.resolve()==path and path.stat().st_uid==os.getuid(),'owned e2e root identity differs');owned_disks.append(path)
    for row in self.observer.frames.values():
     value=row.get('e2e_pass','')
     if re.fullmatch('[0-9]+-[0-9]+',value) and int(value.split('-')[0]) in parent_ids:
      path=R/'e2e-gardes'/('demons-'+value)
      if path.exists():
       policy.require(path.resolve()==path and path.stat().st_uid==os.getuid(),'owned daemon archive identity differs');owned_disks.append(path)
   sizes={str(path):int(subprocess.check_output(['du','-sx','--block-size=1',str(path)],text=True).split()[0]) for path in [SESSION,F,*set(owned_disks)] if path.exists()}
   policy.require(sum(sizes.values())<=32*1024**3,'cumulative session+private storage exceeds32GiB')
   g=Path('/sys/fs/cgroup'+self.cgroup);sample=dict(observed=time.monotonic(),storage=sizes,peak=int((g/'memory.peak').read_text()),events=dict((k,int(v)) for k,v in [l.split() for l in (g/'memory.events').read_text().splitlines()]))
   with (P/'resource-samples.jsonl').open('a') as f:f.write(json.dumps(sample)+'\n')
   self.last_storage=time.monotonic()
 def active_failure(self,paths):
  for path in paths:
   if not path.exists():continue
   with path.open('rb') as f:before=os.fstat(f.fileno());raw=f.read();after=os.fstat(f.fileno())
   text=raw.decode(errors='replace');match=FAILED.search(text)
   compiler=None
   for line in text.splitlines():
    try:row=json.loads(line)
    except ValueError:continue
    if isinstance(row,dict) and row.get('reason')=='compiler-message' and row.get('message',{}).get('level') in ['error','failure-note']:compiler=row;break
   if match or compiler:
    value=dict(path=str(path),device=before.st_dev,inode=before.st_ino,size_before=before.st_size,size_after=after.st_size,bytes_read=len(raw),sha256=evidence.sha(path),match=match.group() if match else compiler,observed=time.monotonic())
    self.fail('active output failure',value);raise RuntimeError('first red in '+str(path))
 def run(self,label,args,cap,env=None,cwd=None,extra_logs=None,expected_code=0):
  end=policy.deadline(time.monotonic(),self.end,cap);self.save(label+'.command.json',dict(argv=args,cwd=str(cwd or C),deadline=end,observed=time.monotonic()))
  path=P/(label+'.log');paths=[path,*(extra_logs or [])]
  with path.open('x') as output:
   child=subprocess.Popen(args,cwd=cwd or C,env=env or self.env,stdout=output,stderr=subprocess.STDOUT)
   while True:
    self.tick();policy.require(time.monotonic()<end,label+' deadline')
    self.active_failure(paths)
    if child.poll() is not None:break
    time.sleep(.05)
   rc=child.wait()
  self.active_failure(paths);policy.require(time.monotonic()<end,label+' deadline after wait')
  text=path.read_text(errors='replace');self.save(label+'.result.json',dict(code=rc,observed=time.monotonic()))
  policy.require(rc==expected_code,label+' returned '+str(rc));return text
 def acknowledge(self,phase,cap=90):
  request=dict(phase=phase,source=policy.APP,pilot=self.config['pilot'],cgroup=self.cgroup,deadline=policy.deadline(time.monotonic(),self.end,cap),services=['7878','7879','fixturePG'],provider='Vertex',account='lr-hub-472010',observed=time.monotonic())
  self.save('resource-request-'+phase+'.json',request);path=P/('resource-ack-'+phase+'.json')
  while not path.exists():self.tick();policy.require(time.monotonic()<request['deadline'],'fresh service/account ACK missing');time.sleep(.1)
  ack=json.loads(path.read_text());policy.require(all(ack.get(k)==request[k] for k in ['phase','source','pilot','cgroup']),'ACK not bound to this phase/source/authority')
  policy.require(ack.get('approved') is True and ack.get('services')==request['services'] and ack.get('provider')=='Vertex' and ack.get('account')=='lr-hub-472010','resource contract differs')
  policy.require(0<=time.monotonic()-ack['observed']<=120 and ack['deadline']>=request['deadline'],'ACK stale/too short')
  return ack
 def published(self,label,ledger):
  path=P/(label+'-ledger.json');self.save(path.name,ledger);end=policy.deadline(time.monotonic(),self.end,90);publication=P/(label+'-published.json')
  while not publication.exists():self.tick();policy.require(time.monotonic()<end,'ledger publication pause expired');time.sleep(.1)
  value=json.loads(publication.read_text());hashes={k:v['sha256'] for k,v in ledger['artifacts'].items()}
  policy.require(value['source']==ledger['source'] and value['pilot']==self.config['pilot'] and value['ledger_sha256']==evidence.sha(path),'ledger publication provenance differs')
  policy.require(value['elf_sha256']==hashes and value['copies']==dict(pc=hashes,local=hashes),'both-host ELF copies missing')
  policy.require(ledger['created']<=value['observed']<=time.monotonic()<end,'publication time invalid')
  for item in ledger['artifacts'].values():policy.require(evidence.sha(item['copy'])==item['sha256'],'ELF copy changed')
  self.ledgers.append(dict(label=label,sha256=evidence.sha(path),publication=value));return value
 def compile(self,label,selection,features,cap,manifest=C,default=True,kind='test'):
  env=dict(self.env,CARGO_MANIFEST_DIR=str(manifest));env.pop('LD_LIBRARY_PATH',None)
  args=['bash',str(C/'scripts/cargo_magasin.sh'),kind,*selection]
  if manifest!=C:args+=['--manifest-path',str(manifest/'Cargo.toml')]
  if not default:args+=['--no-default-features']
  args+=['--features',','.join(features),'--locked','--offline']
  if kind=='test':args+=['--no-run']
  args+=['--message-format=json-render-diagnostics']
  text=self.run('build-'+label,args,cap,env=env)
  if self.observer:policy.require(any(row['argv'][1:]==args[2:] for row in self.observer.cargo),'actual compilation Cargo SDK frame missing')
  builds={};found=[]
  for line in text.splitlines():
   try:row=json.loads(line)
   except ValueError:continue
   if row.get('reason')=='build-script-executed':builds[row['package_id']]=row['out_dir']
   wanted={'large':'rag3weaver','nav':'rag3weaver','e2e':'e2e','host':'rag3weaver-backend','tty':'rag3weaver-tty'}[label]
   if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')==wanted and row.get('executable') and (kind!='test' or row.get('profile',{}).get('test')):found.append(row)
  policy.require(len(found)==1,'unique CargoJSON artifact required');row=found[0];binary=Path(row['executable'])
  policy.require(binary.resolve()==binary and binary.is_relative_to(T) and binary.is_file(),'artifact outside single target')
  policy.require(sorted(row['features'])==policy.feature_closure(manifest/'Cargo.toml',features,default),'feature closure/default differs')
  policy.require(row['manifest_path']==str(manifest/'Cargo.toml'),'CargoJSON from another manifest')
  copy=P/(label+'-elf');shutil.copy2(binary,copy);policy.require(evidence.sha(copy)==evidence.sha(binary),'ELF copy changed')
  self.artifacts[label]=dict(binary=str(binary),copy=str(copy),sha256=evidence.sha(copy),size=copy.stat().st_size,features=sorted(row['features']),requested=features,default=default,artifact=row,builds=builds)
  if self.observer:
   self.observer.executable(binary,manifest,label!='tty',label);self.observer.executable(copy,manifest,label!='tty',label)
  return self.artifacts[label]
 def inventory(self,label):
  item=self.artifacts[label];binary=item['copy']
  listed=self.run(label+'-inventory',[binary,'--list','--format','terse'],30)
  ignored=self.run(label+'-ignored',[binary,'--list','--ignored','--format','terse'],30)
  value=evidence.inventory(listed,ignored);self.inventories[label]=value;self.save(label+'-inventory.json',value);return value
