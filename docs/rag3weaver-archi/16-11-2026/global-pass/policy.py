"""Declared contracts; importing this module performs no filesystem/process I/O."""
from pathlib import Path
import hashlib,json,math,os,re
APP='f1e8a8d502614d10f7cdb40eaea330f08b5d3f37'
BASE='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
OLD='7425375d66a64b0782672225c59516f048682b00'
OLD_PILOT='01c3ae28e8218262ea545513d872f8f3370def0f'
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PASS_SHA='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
COMMON_SHA='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
OLD_MANIFEST='fdafa95b335b57fd2066738995d3edbb3de7a5b65ca3ef632832f1e827e06ef3'
SDK_SHA={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561','include/common/build_id.h':'1cf920608514b5f3c5e90952ecc580ca6170398f288dd554d00bb0bb32b03110'}
FEATURES=['rag3db-native','burn-embedder','burn-ocr','code','daemon','postgres','openai-llm','anthropic-llm']
D6_FEATURES=['rag3db-native','code','daemon']
SUITES=['e2e_lectures','e2e_code','e2e_demon_hote','e2e_hooks_claude_code','e2e_equipier']
KEYS=['RAG3DB_LIB','RAG3DB_SHARED','RAG3DB_LIBRARY_DIR','RAG3DB_INCLUDE_DIR','LD_LIBRARY_PATH','RAG3DB_BUFFER_POOL_SIZE','CARGO_MANIFEST_DIR']
D6_NAME='backend::tests::agent_name_rejects_an_inner_space'
REVIEW_NAME='backend::cx_d6_review::interior_whitespace_contract'
BUDGET=7200;TERM=7440;HARD=7455;FINAL=120
CAPS={'preflight':120,'builds':900,'tty':240,'publication':90,'d6':1800,'canonical':3900,'final':120}

def require(ok,reason):
 if not ok:raise RuntimeError(reason)

def jobs(fils):
 require(int(fils)>=8,'exclusive poste allocation must provide at least eight cores')
 return min(8,int(fils))

def canonical(path):
 p=Path(path);require(p.is_absolute() and p.resolve()==p,'noncanonical path');return p

def source_target(root,name):
 safe=''.join(c if c.isascii() and (c.isalnum() or c in '-_.') else '_' for c in name)
 require(safe not in ['','.','..'],'invalid repository name')
 return Path(root)/('taches-'+safe)

def deadline(now,common_end,cap):
 require(all(math.isfinite(x) for x in [now,common_end,cap]) and cap>0,'invalid clock/cap')
 end=min(common_end-15,now+cap);require(now<end,'phase deadline expired');return end

def profile(store,manifest,ld):
 root=Path(store)/LIB
 return dict(RAG3DB_LIB=LIB,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(root),RAG3DB_INCLUDE_DIR=str(root),LD_LIBRARY_PATH=ld,RAG3DB_BUFFER_POOL_SIZE='1073741824',CARGO_MANIFEST_DIR=str(manifest))

def selected(raw,expected):
 require(raw and raw.endswith(b'\0'),'empty/truncated environment')
 fields={}
 for item in raw.split(b'\0'):
  if b'=' not in item:continue
  k,v=item.split(b'=',1)
  if k in [os.fsencode(x) for x in KEYS]+[b'RAG3WEAVER_MOTEUR_ANCIEN']:
   require(k not in fields,'duplicate selected field');fields[k]=v
 actual={k:os.fsdecode(fields[os.fsencode(k)]) if os.fsencode(k) in fields else None for k in KEYS}
 return dict(rawlen=len(raw),actual=actual,present={k:os.fsencode(k) in fields for k in KEYS},expected=expected,
  expected_presence={k:v is not None for k,v in expected.items()},diff={k:dict(expected=v,actual=actual[k]) for k,v in expected.items() if actual[k]!=v},old_engine_present=b'RAG3WEAVER_MOTEUR_ANCIEN' in fields)

def validate_profile(snapshot):
 require(not snapshot['diff'] and snapshot['actual']==snapshot['expected'],'SDK profile mismatch')
 require(snapshot['rawlen']>0 and snapshot['present']==snapshot['expected_presence'] and not snapshot['old_engine_present'],'SDK field presence mismatch')

def sandbox(manifest,q,socket_root,sdk,fils):
 result=json.loads(json.dumps(manifest));w=result['workspace'];s=w['sandbox']
 require(w['commands']=='approval' and s.get('mode','landlock')=='landlock' and not s.get('network',False) and not s.get('extra_write',[]),'unsafe sandbox declaration')
 for path in [q,socket_root,sdk]:
  text=str(canonical(path))
  if text not in s['extra_read']:s['extra_read'].append(text)
 s['env'].update(CARGO_TARGET_DIR='{repo_target}',TMPDIR='{task_dir}/tmp',CARGO_NET_OFFLINE='true',CARGO_BUILD_JOBS=str(jobs(fils)))
 s['env']['PATH']=str(Path(q)/'bin')+':'+str(Path.home()/'.cargo/bin')+':/usr/bin:/bin'
 s['env']['RAG3WEAVER_D6_SOCKET']=str(Path(socket_root)/'cargo.sock')
 s['env']['RAG3WEAVER_D6_OUTPUT']='{task_dir}/tmp/pilot'
 return result

def feature_closure(manifest,requested,default):
 import tomllib
 table=tomllib.loads(Path(manifest).read_text()).get('features',{})
 result=set();queue=list(requested)+(['default'] if default else [])
 while queue:
  name=queue.pop()
  if name in result:continue
  require(name in table,'undeclared root feature '+name);result.add(name)
  queue.extend(v for v in table[name] if v in table)
 return sorted(result)

def d6_command(name):
 require(name in [D6_NAME,REVIEW_NAME],'unknown D6 witness')
 return ['test','--lib','--no-default-features','--features',','.join(D6_FEATURES),'--locked','--offline',name,'--','--exact','--nocapture','--test-threads=1']

def split_test(argv):
 require(argv in [d6_command(D6_NAME),d6_command(REVIEW_NAME)],'undeclared agent Cargo argv')
 cut=argv.index('--');before=argv[:cut];name=before[-1]
 return before[:-1]+['--no-run','--message-format=json-render-diagnostics'],[name,*argv[cut+1:]]

def judged(text,name,failed=False):
 require('error: could not compile' not in text and '"level":"error"' not in text,'compile failure is not a test result')
 rows=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
 want=('FAILED','0','1','0') if failed else ('ok','1','0','0')
 require(rows==[want],'one exact judged test required')
 require(re.findall(r'^test (.+?)(?: - should panic)? \.\.\. ',text,re.M)==[name],'another test ran')
 if failed:require('panicked at' in text,'TDD failure must be a real Rust assertion')
 return dict(name=name,passed=0 if failed else 1,failed=1 if failed else 0,ignored=0)

def private_environment(inherited,p,tmp,targets,fils):
 keys=['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON']
 env={k:inherited[k] for k in keys if k in inherited}
 env.update(POSTE_FILS=str(fils),CARGO_BUILD_JOBS=str(jobs(fils)),RAYON_NUM_THREADS=str(jobs(fils)),RUST_TEST_THREADS='1',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_BACKTRACE='1',TMPDIR=str(tmp),TMP=str(tmp),TEMP=str(tmp),RAG3WEAVER_TARGETS_DIR=str(targets),RAG3WEAVER_TARGET_MAX_GO='32',RAG3DB_LIB=LIB,RAG3DB_LIBS_DIR=str(Path.home()/'.cache/rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824')
 for key,leaf in {'XDG_CONFIG_HOME':'config','XDG_DATA_HOME':'data','XDG_CACHE_HOME':'cache','XDG_STATE_HOME':'xdg-state','RAG3WEAVER_STATE_DIR':'state-fixture','RAG3WEAVER_CONFIG_DIR':'config','RAG3WEAVER_DATA_DIR':'data','RAG3WEAVER_DAEMON_DIR':'daemon','RAG3WEAVER_BUILTIN_TEMPLATES_DIR':'templates','RAG3WEAVER_RUNS_DIR':'runs','RAG3WEAVER_TACHES_DIR':'tasks','RAG3WEAVER_CHATS_DIR':'chats','RAG3WEAVER_JOURNAUX_DIR':'journals','RAG3WEAVER_CACHE_DIR':'cache'}.items():env[key]=str(p/leaf)
 for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
 return env
