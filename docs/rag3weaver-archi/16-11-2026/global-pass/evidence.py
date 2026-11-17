"""Bounded writes, independent test inventories and strict phase classification."""
from pathlib import Path
import hashlib,json,os,re,time,contextlib,stat
import policy
SUMMARY=re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$',re.M)
EVENT=re.compile(r'^test (\S+?)( - should panic)? \.\.\. (ok|FAILED|ignored(?:, .*)?)$')
SLOW=re.compile(r'^test (\S+) has been running for over [1-9][0-9]* seconds$')

def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
 with Path(path).open('x') as f:json.dump(value,f,indent=2,ensure_ascii=False);f.write('\n');f.flush();os.fsync(f.fileno())

def finish(path,value,clock=time.monotonic):
 with Path(path).open('x') as f:
  if value['status']=='GREEN_GLOBAL_D6_AND_CANONICAL' and clock()>=value['deadline']:value=dict(value,status='RED',error='deadline after open')
  json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
  if value['status']=='GREEN_GLOBAL_D6_AND_CANONICAL' and clock()>=value['deadline']:
   value=dict(value,status='RED',error='deadline after fsync');f.seek(0);f.truncate();json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
 return value

def names(text):
 values=re.findall(r'^(.+): test$',text,re.M)
 policy.require(len(values)==len(set(values)) and values,'empty/duplicate test inventory')
 return sorted(values)

def inventory(listed,ignored):
 all_names=names(listed);skip=re.findall(r'^(.+): test$',ignored,re.M)
 policy.require(len(skip)==len(set(skip)) and set(skip)<=set(all_names),'invalid ignored inventory')
 return dict(names=all_names,ignored=sorted(skip))

NESTED_NAMES=['daemon_host::teammate_tests::task_fixture_uses_the_declared_private_sandbox_before_add',
              'e2e_equipier::documentation_fixture_uses_the_declared_private_sandbox_before_add']

def nested_setups(text):
    records=[]
    for name in NESTED_NAMES:
        pattern=re.compile(r'^test '+re.escape(name)+r' \.\.\. DOCUMENTATION_SETUP_CHILD_OK '+re.escape(name)+r'\n(?P<child>.*?^test result: ok\. 1 passed; 0 failed; 0 ignored;[^\n]*\n)\s*ok\n',re.M|re.S)
        def collapse(match):
            child=match['child'];policy.judged(child,name)
            rows=re.findall(r'DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED ([^\n]+)',child)
            policy.require(len(rows)==1,'one child committed-Sandbox marker')
            row=json.loads(rows[0]);policy.require(row['name']==name and row['commands']=='approval' and row['manifest_equals_declaration'] is True and row['head_equals_declaration'] is True,'child HEAD/declaration mismatch')
            policy.require(re.fullmatch('[a-f0-9]{40}',row['head']) and row['cargo_jobs']=='1' and row['offline']=='true' and row['target_token']=='{repo_target}' and row['tmp_token']=='{task_dir}/tmp','child Sandbox fields differ')
            policy.require(Path(row['targets_root']).is_absolute() and Path(row['manifest']).is_absolute(),'child private paths absent')
            summary=next(SUMMARY.fullmatch(line) for line in child.splitlines() if line.startswith('test result:'))
            policy.require(summary is not None,'invalid child summary')
            records.append(dict(attestation=row,summary=list(map(int,summary.groups()[1:]))));return 'test '+name+' ... ok\n'
        text=pattern.sub(collapse,text)
    policy.require('DOCUMENTATION_SETUP_CHILD_OK' not in text and 'DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED' not in text,'unconsumed nested setup marker')
    return text,records

def phase(text,selected,ignored,total):
 text,nested=nested_setups(text)
 policy.require(all(row['summary']==[1,0,0,0,total-1] for row in nested),'child summary not from the same ELF inventory')
 lines=text.splitlines();headers=[(i,l) for i,l in enumerate(lines) if re.match(r'\s*running(?:\s|$)',l)]
 policy.require(len(headers)==1 and re.fullmatch(r'running [1-9][0-9]* tests?',headers[0][1]),'one valid nonzero running header')
 ends=[(i,SUMMARY.fullmatch(l)) for i,l in enumerate(lines) if l.lstrip().startswith('test result:')]
 policy.require(len(ends)==1 and ends[0][1] is not None,'one exact summary')
 start,count=headers[0][0],int(headers[0][1].split()[1]);end,summary=ends[0]
 expected=[len(selected)-len(ignored),0,len(ignored),0,total-len(selected)]
 policy.require(summary[1]=='ok' and list(map(int,summary.groups()[1:]))==expected and count==len(selected) and start<end,'phase counts/filters differ from pretest inventory')
 events=[];slow=[];pending=None
 for i,line in enumerate(lines):
  if line.startswith('test result:'):continue
  if pending is not None and line in ['ok','FAILED']:
   policy.require(start<i<end and line=='ok','pending Rust test failed/outside phase')
   events.append((pending,'ok'));pending=None;continue
  if not line.lstrip().startswith('test '):continue
  event=EVENT.fullmatch(line);diagnostic=SLOW.fullmatch(line)
  policy.require(start<i<end,'test event outside phase')
  if diagnostic:slow.append(diagnostic[1]);continue
  if event:
   policy.require(pending is None and event[3]!='FAILED','overlapping or failed test')
   events.append((event[1],'ignored' if event[3].startswith('ignored') else 'ok'));continue
  # --nocapture output can share the harness prefix; one thread is an explicit runtime invariant.
  prefix=re.match(r'^test (\S+?)(?: - should panic)? \.\.\. ',line)
  policy.require(prefix is not None and prefix[1] in selected and pending is None,'unconsumed/interleaved test event')
  pending=prefix[1]
 policy.require(pending is None,'unterminated Rust test prefix')
 policy.require(len(events)==len(set(n for n,s in events))==len(selected) and set(n for n,s in events)==set(selected),'missing/duplicate/foreign judge')
 policy.require({n for n,s in events if s=='ignored'}==set(ignored) and set(slow)<=set(selected),'ignored/long-running mismatch')
 return dict(summary=expected,names=sorted(selected),nested_setups=nested,nested_raw_invocations=len(nested))

def canonical(root,inventories,nav_names,suites):
 root=Path(root);results={};lib=inventories['large'];e2e=inventories['e2e'];nav=inventories['nav']
 chosen=[n for n in lib['names'] if 'daemon_host::teammate_tests' in n]
 results['temoins']=phase((root/'temoins.log').read_text(),chosen,[],len(lib['names']))
 for suite in suites:
  text=(root/(suite+'.log')).read_text();a='     Running tests/e2e_filets.rs (binaire e2e unique, ses propres filets)';b=f'     Running tests/{suite}.rs (binaire e2e unique, filtre {suite}::)'
  policy.require(re.findall(r'^     Running tests/[^\n]+$',text,re.M)==[a,b],'e2e headers/order differ')
  before,tail=text.split(a,1);left,right=tail.split(b,1)
  policy.require(not re.search(r'^\s*(?:running|test)\s',before,re.M),'events before e2e phase')
  filets=['chaque_suite_est_montee','aucune_suite_ne_se_relance_par_son_nom_court'];selected=[n for n in e2e['names'] if n.startswith(suite+'::')]
  policy.require(set(filets)<=set(e2e['names']) and selected,'missing filets/suite in independent inventory')
  results[suite+'/filets']=phase(left,filets,[],len(e2e['names']))
  results[suite+'/suite']=phase(right,selected,[],len(e2e['names']))
 results['lib']=phase((root/'lib.log').read_text(),lib['names'],lib['ignored'],len(lib['names']))
 results['navigateur']=phase((root/'navigateur.log').read_text(),nav_names,[],len(nav['names']))
 feature=(root/'check_features.log').read_text().splitlines();policy.require(feature.count('ETAT_SUITE_E2E VERT')==1,'F8 preflight boundary')
 games=['sans-feature','daemon','rag3db-native,daemon','rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres','rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm','rag3db-native,code,daemon,openai-llm','rag3db-native,code','rag3db-native']
 policy.require(feature[feature.index('ETAT_SUITE_E2E VERT')+1:]==['VERT  '+g for g in games]+['CHECK_FEATURES VERT 8/8'],'F8 games differ')
 resume=(root/'resume.log').read_text().splitlines();sections=[(i,l) for i,l in enumerate(resume) if l.startswith('== ')]
 expected=[('== témoin',['temoins'])]+[('== '+s,[s+'/filets',s+'/suite']) for s in suites]+[('== check_features',[]),('== lib',['lib']),('== témoins navigateur',['navigateur'])]
 policy.require([l for i,l in sections]==[l for l,k in expected],'canonical phase sequence differs')
 policy.require(not any(l.startswith(('test result:','preuve :','CHECK_FEATURES ')) for l in resume[:sections[0][0]]),'orphan summary')
 for index,(start,header) in enumerate(sections):
  stop=sections[index+1][0] if index+1<len(sections) else len(resume);body=resume[start+1:stop];keys=expected[index][1]
  rows=[SUMMARY.fullmatch(l) for l in body if l.startswith('test result:')]
  wanted=[]
  for key in keys:wanted.extend([row['summary'] for row in results[key]['nested_setups']]+[results[key]['summary']])
  policy.require(len(rows)==len(wanted) and all(m and m[1]=='ok' and list(map(int,m.groups()[1:]))==fields for m,fields in zip(rows,wanted)),'summary in wrong canonical block')
  markers=[l for l in body if l.startswith(('preuve :','CHECK_FEATURES '))]
  policy.require(markers==(['CHECK_FEATURES VERT 8/8','preuve : rag3weaver revérifié'] if header=='== check_features' else []),'proof/F8 markers in wrong block')
 policy.require(resume[-2:]==['suites jouées : '+' '.join(suites),'VERT'],'canonical final verdict differs')
 return results


@contextlib.contextmanager
def preserve_failure(latch,cleanup):
 primary=None
 try:
  yield
 except BaseException as error:
  primary=error
  try:latch('primary refusal before cleanup',repr(error))
  except BaseException as failure:error.add_note('latch refused: '+repr(failure))
  raise
 finally:
  try:cleanup()
  except BaseException as error:
   try:latch('cleanup refusal',repr(error))
   except BaseException as failure:error.add_note('cleanup latch refused: '+repr(failure))
   if primary is None:raise
   primary.add_note('cleanup refusal preserved separately: '+repr(error))


def source_state(root,git):
 root=Path(root);head=git('rev-parse','HEAD');index=git('ls-files','--stage','-z');others=git('ls-files','--others','-z')
 entries={}
 for record in index.split('\0'):
  if not record:continue
  meta,name=record.split('\t',1);mode,oid,stage=meta.split(' ')
  policy.require(stage=='0','unmerged source index')
  entries[name]=dict(index_mode=mode,index_oid=oid)
 for name in others.split('\0'):
  if name:entries.setdefault(name,dict(untracked=True))
 for name,row in entries.items():
  relative=Path(name);policy.require(not relative.is_absolute() and '..' not in relative.parts,'unsafe source inventory path')
  path=root/relative
  if row.get('index_mode')=='160000':
   # Gitlink source content must be the exact initialized, clean linked checkout.
   import subprocess
   sub=lambda *args:subprocess.check_output(['git','-C',str(path),*args],text=True).strip()
   row['submodule_head']=sub('rev-parse','HEAD');row['submodule_status']=sub('status','--porcelain','--untracked-files=all')
   policy.require(row['submodule_head']==row['index_oid'] and not row['submodule_status'],'dirty or mismatched source gitlink')
  elif path.is_symlink():row.update(kind='symlink',target=os.readlink(path))
  elif path.exists():
   st=path.lstat();policy.require(stat.S_ISREG(st.st_mode),'unsupported source file type')
   row.update(kind='file',mode=stat.S_IMODE(st.st_mode),sha256=sha(path))
  else:row['kind']='missing'
 return dict(head=head,index=index,files=entries)
