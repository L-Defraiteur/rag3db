"""Private real host/TTY task, its two agent Cargo calls and independent review."""
from pathlib import Path
import hashlib,http.client,json,os,shlex,shutil,subprocess,time,urllib.parse,threading
import broker,evidence,policy
from settings import *
REVIEW='''\n#[cfg(test)]\nmod cx_d6_review {\n #[test]\n fn interior_whitespace_contract() {\n  use super::valider_le_nom_de_l_agent as check;\n  assert!(check("Om bre").unwrap_err().contains("agent.name"));\n  assert_eq!(check("Ombre").unwrap(), "Ombre");\n  assert_eq!(check("  Ombre  ").unwrap(), "Ombre");\n  assert!(check("").is_err());\n  assert!(check("Om\\nbre").is_err());\n  assert!(check("Om\\rbre").is_err());\n }\n}\n'''

def request(endpoint,path,body=None,session=None,end=None,cap=60):
 address=endpoint['address'];host,port=address.rsplit(':',1)
 policy.require(host=='127.0.0.1' and int(port)>0,'not the private host')
 timeout=min(cap,max(.01,(end or time.monotonic()+60)-time.monotonic()))
 headers={'Authorization':'Bearer '+endpoint['token'],'Content-Type':'application/json'}
 if session:headers['X-Rag3weaver-Session']=session
 connection=http.client.HTTPConnection(host,int(port),timeout=timeout)
 try:
  connection.request('GET' if body is None else 'POST',path,body=None if body is None else json.dumps(body),headers=headers)
  response=connection.getresponse();data=response.read();policy.require(response.status==200,'private HTTP refusal '+str(response.status));value=json.loads(data)
  if isinstance(value,dict) and value.get('ok') is False:raise RuntimeError('private API refused: '+str(value.get('error')))
  return value
 finally:connection.close()

def await_request(ctx,call):
 box=[]
 def worker():
  try:box.append((True,call()))
  except BaseException as error:box.append((False,error))
 thread=threading.Thread(target=worker,daemon=True);thread.start()
 while thread.is_alive():ctx.tick();time.sleep(.05)
 thread.join();ok,value=box[0]
 if not ok:raise value
 return value

def result(value):return value.get('result',value)

def setup(ctx):
 root=F/'d6-source';ctx.run('d6-clone',['git','clone','--no-hardlinks','--no-local',str(W),str(root)],90,cwd=F)
 ctx.run('d6-checkout',['git','-C',str(root),'checkout','--detach',policy.APP],30,cwd=F)
 ctx.run('d6-gitlink',['git','-C',str(root),'submodule','update','--init','extension/rag3weaver/codeparsers'],90,cwd=F)
 policy.require(ctx.git(root,'rev-parse','HEAD')==policy.APP and not ctx.git(root,'status','--porcelain'),'D6 copy not exact')
 runtime=F/'runtime';source_templates=runtime/'config'/'code-source'/'templates';source_templates.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(C/'templates',source_templates)
 source=source_templates/'backends/code/backend.json';value=policy.sandbox(json.loads(source.read_text()),Q,F/'private-controller',Path(ctx.config['sdk_root']),os.environ['POSTE_FILS'])
 value['database']=str(runtime/'data'/'source-code.rag3db');value['models']['llm']=dict(provider='vertex',model='gemini-3.8-flash',params=dict(project='lr-hub-472010',location='global'))
 value['agent']=dict(name='Algareth')
 memory=source_templates/'backends/memory/backend.json';m=json.loads(memory.read_text());m['database']=str(runtime/'data'/'d6-judges.rag3db');memory.write_text(json.dumps(m,indent=2))
 value['bilans']=dict(memoire=str(memory));source.write_text(json.dumps(value,indent=2))
 declaration=value['workspace']['sandbox'];ctx.save('d6-sandbox-before-add.json',dict(manifest=str(source),sandbox=declaration,sha256=evidence.sha(source)))
 host=ctx.artifacts['host']['copy'];env=dict(ctx.env)
 text=ctx.run('d6-code-tree-add',[host,'code-tree','add',str(root/'extension/rag3weaver'),'--from',str(source),'--as','d6-private','--sans-hote'],90,env=env)
 manifest=runtime/'config/d6-private/templates/backends/code/backend.json';installed=json.loads(manifest.read_text())
 policy.require(installed['workspace']['sandbox']==declaration,'Sandbox not installed before task')
 config_root=runtime/'config/d6-private';relative=str(manifest.relative_to(config_root));committed=json.loads(ctx.git(config_root,'show','HEAD:'+relative))
 policy.require(committed['workspace']['sandbox']==declaration,'Sandbox not in parent HEAD before task')
 ctx.save('d6-sandbox-committed.json',dict(manifest=str(manifest),head=ctx.git(config_root,'rev-parse','HEAD'),relative=relative,sandbox=declaration,installed_equals_declaration=True,head_equals_declaration=True))
 return root,manifest,config_root,declaration

def verify_mailbox(mailbox):
 requests=[r for r in mailbox if r.get('kind')=='request'];policy.require(len(requests)==1,'D6 must retain exactly one request')
 policy.require(not any(r.get('approved') is True or r.get('granted') for r in mailbox),'generic approval/grant in D6')
 for row in mailbox:
  if row.get('kind') in ['question','approval']:
   if row.get('kind')=='approval' and row.get('command')=='rm Cargo.lock':continue
   raise RuntimeError('unknown D6 question/approval')
 return requests[0]

def run(ctx,service):
 end=ctx.phase_deadline
 ack=ctx.acknowledge('d6');ctx.save('d6-used-resource-ack.json',ack)
 root,manifest,config_root,declaration=setup(ctx);env=dict(ctx.env,RAG3WEAVER_DAEMON_IDLE_MIN='0')
 host_log=P/'d6-host.log';endpoint_path=Path(env['RAG3WEAVER_DAEMON_DIR'])/'daemon.json'
 policy.require(not endpoint_path.exists(),'private host endpoint already exists')
 host_output=host_log.open('x');host=subprocess.Popen([ctx.artifacts['host']['copy'],'daemon','--address','127.0.0.1:0','--threads','8'],cwd=C,env=env,stdout=host_output,stderr=subprocess.STDOUT)
 endpoint=None;tty=None
 try:
  while not endpoint_path.exists():
   ctx.tick();policy.require(host.poll() is None and time.monotonic()<end,'host did not start');time.sleep(.1)
  endpoint=json.loads(endpoint_path.read_text());policy.require(endpoint['pid']==host.pid,'private daemon PID mismatch')
  health=request(endpoint,'/sante',end=end);policy.require(health['pid']==host.pid,'health is another host')
  ctx.save('d6-host-public.json',{k:endpoint[k] for k in ['address','pid','build','started_at']})
  session=request(endpoint,'/session',dict(client='c7-validation',keys=['read_files','run_commands'],pid=os.getpid()),end=end)['session']
  opened=result(request(endpoint,'/backend/open',dict(manifest=str(manifest)),session,end));parent=opened['id']
  task=result(request(endpoint,'/ide/taches',dict(backend=parent,nom='d6',agent_name='Algareth'),end=end));ctx.save('d6-task.json',task)
  task_manifest=Path(task['manifest']);task_cwd=Path(task['workspace']['racine']);policy.require(task_cwd.resolve()==task_cwd and task_cwd.is_relative_to(F),'task outside private fixture')
  policy.require(json.loads(task_manifest.read_text())['workspace']['sandbox']==declaration,'task lost precommitted Sandbox')
  ctx.run('d6-task-gitlink',['git','-C',str(task_cwd),'submodule','update','--init'],90,cwd=task_cwd)
  policy.require(ctx.git(task_cwd,'rev-parse','HEAD')==policy.APP and not ctx.git(task_cwd,'status','--porcelain'),'task copy not exact before model')
  service.cwd=task_cwd;service.mode='d6';target=policy.source_target(TASKS,Path(task['workspace']['racine_principale']).name);service.target=target
  ctx.save('d6-task-admitted.json',dict(cwd=str(task_cwd),target=str(target),source=policy.APP,sandbox=declaration,parent_head=ctx.git(config_root,'rev-parse','HEAD'),task_head=ctx.git(task_cwd,'rev-parse','HEAD')))
  command=shlex.join(['cargo',*policy.d6_command(policy.D6_NAME)])
  prompt=("Dans src/backend.rs, ajoute dans le module tests le test nommé agent_name_rejects_an_inner_space : valider_le_nom_de_l_agent doit refuser le nom Om bre (blanc intérieur) avec une erreur nommant agent.name. "
          "Joue d'abord EXACTEMENT "+command+" via run_command, timeout_s=900, et vois ce test ROUGE pour cette assertion. "
          "Corrige ensuite le validateur pour ce contrat en préservant le trim des bords, le refus vide et des retours ligne ; rejoue EXACTEMENT la même commande VERT. "
          "Rends le diff et Ready. Une seule demande : travaille jusqu'au bout. Pas de commit/push, pas de suppression du Cargo.lock, pas d'autres commandes Cargo, pas de lecture de secrets ou de configuration personnelle.")
  ctx.save('d6-request-before-model.json',dict(text=prompt,expected_test=policy.D6_NAME,expected_argv=policy.d6_command(policy.D6_NAME),expected_red='one real failing Rust assertion before the validator fix'))
  tty_env=dict(env,CARGO_MANIFEST_DIR=str(C/'tty'));tty_log=(P/'d6-tty.log').open('x')
  tty=subprocess.Popen([ctx.artifacts['tty']['copy'],'--plain','--depot',parent,'--tache','d6','--demande',prompt,'--journal',str(P/'d6-terminal.txt'),'--daemon-json',str(endpoint_path)],cwd=C,env=tty_env,stdin=subprocess.DEVNULL,stdout=tty_log,stderr=subprocess.STDOUT)
  observed_running=False;last_mailbox=None
  while True:
   ctx.tick();policy.require(time.monotonic()<end,'D6 deadline')
   state=result(request(endpoint,'/ide/teammate?'+urllib.parse.urlencode(dict(backend=parent,task='d6')),end=end));mailbox=state.get('mailbox',[])
   if mailbox:
    verify_mailbox(mailbox)
    for row in mailbox:
     if row.get('kind')=='approval' and row.get('command')=='rm Cargo.lock' and not any(r.get('reply_to')==row.get('id') and r.get('approved') is False for r in mailbox):
      request(endpoint,'/ide/teammate/mailbox',dict(backend=parent,task='d6',kind='answer',reply_to=row['id'],approved=False,**{'from':'c7-validation'},text='Garde le Cargo.lock et continue la demande initiale.'),end=end)
   observed_running=observed_running or state.get('running',False)
   last_mailbox=mailbox
   if mailbox and any(r.get('kind')=='ended' for r in mailbox) and not state.get('running',False):break
   policy.require(tty.poll() in [None,0],'TTY failed');time.sleep(.25)
  ctx.save('d6-final-mailbox.json',last_mailbox);policy.require(verify_mailbox(last_mailbox).get('text')==prompt,'D6 original request changed')
  policy.require(len(service.completed)==2 and [r['stage'] for r in service.completed]==['agent-red','agent-green'],'agent did not complete red then green')
  policy.require(any(r.get('kind')=='report' and 'ready' in r.get('text','').lower() for r in last_mailbox),'no Ready report')
  if tty.poll() is None:
   while tty.poll() is None:ctx.tick();policy.require(time.monotonic()<end,'TTY did not finish');time.sleep(.1)
  policy.require(tty.wait()==0,'TTY return code');tty_log.close()
  diff=ctx.git(task_cwd,'diff','--binary');names=ctx.git(task_cwd,'diff','--name-only').splitlines()
  policy.require(names==['extension/rag3weaver/src/backend.rs'],'D6 changed another source path')
  policy.require(ctx.git(root,'rev-parse','HEAD')==policy.APP and not ctx.git(root,'status','--porcelain'),'parent D6 source mutated')
  (P/'d6-agent.diff').write_text(diff);ctx.save('d6-agent-diff.json',dict(sha256=hashlib.sha256(diff.encode()).hexdigest(),source=policy.APP,role='agent',before_controller_review=True))
  backend=task_cwd/'src/backend.rs';before=backend.read_bytes();backend.write_bytes(before+REVIEW.encode());service.review=True
  review_command=shlex.join(['cargo',*policy.d6_command(policy.REVIEW_NAME)])
  reply=await_request(ctx,lambda:request(endpoint,'/call',dict(manifest=str(task_manifest),name='run_command',arguments=dict(command=review_command,timeout_s=900,max_output=20000)),session,end,cap=900))
  ctx.save('d6-controller-review-api.json',reply);policy.require(len(service.completed)==3 and service.completed[-1]['stage']=='review','independent real Rust review missing')
  # Do not attribute the reviewer's injected source/test to the agent.
  ctx.save('d6-review-source.json',dict(role='controller-review',before_sha256=hashlib.sha256(before).hexdigest(),after_sha256=evidence.sha(backend),injected_test=policy.REVIEW_NAME))
  steps=[r for r in last_mailbox if r.get('kind')=='step_done'];policy.require(steps,'no actual judged step')
  judge_end=min(end,time.monotonic()+600)
  while True:
   ctx.tick();policy.require(time.monotonic()<judge_end,'real Vertex judges did not finish')
   judgment=result(request(endpoint,'/ide/bilans?'+urllib.parse.urlencode(dict(backend=task['id'])),end=judge_end))
   cards=[v for v in judgment.get('fiches',[]) if v.get('outil') is not True]
   if any(len(v.get('apports',[]))>=len(steps) for v in cards):break
   time.sleep(1)
  ctx.save('d6-real-judgments.json',dict(steps=len(steps),view=judgment,controller_test_is_not_agent_work=True))
  journals=[]
  for file in sorted((F/'runtime/runs').rglob('*.json.zst')):
   raw=subprocess.check_output(['zstd','-dc',str(file)]);value=json.loads(raw);journals.append(value)
   secret_values=[endpoint['token'],os.environ.get('RAG3WEAVER_VALIDATION_JETON','')]
   text=raw.decode()
   for secret in secret_values:
    if secret:text=text.replace(secret,'[private-capability-redacted]')
   (P/('d6-run-'+str(len(journals))+'.json')).write_text(text)
  policy.require(journals and any(v.get('tours') for v in journals),'actual run/tool journal absent')
  events=[e for value in journals for e in value.get('evenements',[])];policy.require(events,'actual model/tool events absent')
  policy.require(any(r['label']=='host' and r['pid']==host.pid for r in ctx.observer.runtimes),'private daemon SDK identity was not observed')
  policy.require(any(r['label']=='tty' and r['pid']==tty.pid for r in ctx.observer.runtimes),'real TTY runtime profile was not observed')
  ctx.save('d6-result.json',dict(status='GREEN_D6_ONLY',agent_cargo_stages=['agent-red','agent-green'],controller_review='review',source=policy.APP,request_count=1,journal_count=len(journals),events=len(events),observed_running=observed_running,finished=time.monotonic(),not_integrated=True))
 finally:
  if endpoint:
   try:request(endpoint,'/arret',{},end=min(end,time.monotonic()+15))
   except Exception as error:ctx.save('d6-host-stop-error.json',dict(error=repr(error)));raise
  if host.poll() is None:
   while host.poll() is None:policy.require(time.monotonic()<end,'private host did not stop');time.sleep(.1)
  policy.require(host.wait()==0,'private host failed');host_output.close()
