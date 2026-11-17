#!/usr/bin/env python3
"""B source proposal: bind the unchanged Qv9 protocol to CLI99/SDK59.

No launch is authorized by publication or metadata preparation. Old Qv9 is
imported under an alias, never edited. A/P/Q/S/T and all old proofs survive.
"""
from pathlib import Path
import argparse,hashlib,json,os,shutil,stat,subprocess,sys,time

APP='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
ROOT=Path('/home/lucied/.cache/rag3weaver-build')
R=ROOT/'bascule-live-20261116'
P=R/'cli99-runtime-b-copy-r1';Q=R/'cli99-runtime-b-control-r1';S=R/'cli99-runtime-b-scopes-r1'
SOURCE_ROOT=Path('/home/lucied/git_workspaces/rag3db-cx-arbre-qualif-cli99-20261117-r1')
SOURCE_SUBMODULE='bd133850f89d6b12a3ca2b2f6ce238785ed37e3a'
A=ROOT/'qualification-cli99-pc-r1';A_Q=ROOT/'qualification-cli99-control-r1'
A_PROOF=ROOT/'qualification-cli99-static-a-proofs-01'
A_MANIFEST='fab645b8020848f4a3119fb45ebddfca59189a1a74f83f8c060901c20515d0ee'
A_PREPARED='f88671ab5b27d3d6723dfbdc7e1bbee08734077def4d4911a07954a1b4ff6eb5'
A_LEDGER='02b6117e36374de7d1b0bf8eb98192b8ad637a991c7df6d638aa2b18a455da31'
Q9_PREPARED='96ebc22e7fae474f812273e0bf2f8d41b3f24b5205c6e4f21d4594fbbd0f3099'
PREFIX='docs/arbre-principal/17-11-2026/runtime-99/'
OWN=['lifecycle.py','reader_runtime.py','reader_capture.py','reader_exec.py','lifecycle.json','helper-origins.json','q9-prepared-reference.json']

def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def read(path):
    path=Path(path)
    if path.resolve()!=path or any('.env' in part or '.vault' in part for part in path.parts) or not stat.S_ISREG(path.stat().st_mode) or path.stat().st_uid!=os.getuid():raise ValueError('ambiguous/foreign/protected metadata')
    return json.loads(path.read_text())

def git(root,*args):return subprocess.check_output(['/usr/bin/git','-C',str(root),*args],text=True).strip()

def source_guard():
    if git(SOURCE_ROOT,'rev-parse','HEAD')!=APP or git(SOURCE_ROOT,'status','--porcelain'):raise ValueError('APP99 not exact and clean')
    parser=SOURCE_ROOT/'extension/rag3weaver/codeparsers'
    if git(parser,'rev-parse','HEAD')!=SOURCE_SUBMODULE or git(parser,'status','--porcelain'):raise ValueError('private parser not exact/clean')

def a_guard():
    if sha(A_PROOF/'proof-manifest.json')!=A_MANIFEST or sha(A_Q/'prepared.json')!=A_PREPARED or sha(A/'ledger-before-first-invocation.json')!=A_LEDGER:raise ValueError('A provenance changed')
    ledger=read(A/'ledger-before-first-invocation.json')
    if ledger.get('app')!=APP or ledger.get('config_sha256')!=A_PREPARED or set(ledger['artifacts'])!={'rag3weaver-backend','rag3db-check-lib','typed-reader','ids','query'}:raise ValueError('foreign/incomplete A ledger')
    for item in ledger['artifacts'].values():
        for key in ['path','origin']:
            path=Path(item[key])
            if path.resolve()!=path or path.is_symlink() or sha(path)!=item['sha256']:raise ValueError('A artifact/origin changed')
    composed=read(A_Q/'composed.json')
    if composed.get('status')!='BUILD_STATIC_PAIR_OBSERVED' or composed.get('source')!=APP or composed.get('prepared_sha256')!=A_PREPARED:raise ValueError('A not observed static pair')
    return ledger

def install():
    import lifecycle_v9 as old
    import reader_runtime
    a=old.a;q=old.q
    old.P=P;old.Q=Q;old.S=S;old.BASE=P/'copy/code-principal.rag3db'
    old.f.Q=Q;old.f.carried.Q=Q
    a.P=P;a.Q=Q;a.S=S;a.BASE=old.BASE;q.P=P;q.Q=Q;q.BASE=old.BASE
    a.BINARY=A/'artifacts/typed-reader'
    old_cli=q.CLI;q.CLI=A/'artifacts/rag3weaver-backend';q.IDS=A/'artifacts/ids'
    q.PINS={path:digest for path,digest in q.PINS.items() if path not in [old_cli,Path('/home/lucied/.cache/rag3weaver-build/bascule-live-20261116/id-sets-pc-01/snapshot-files')]}
    q.PINS.update({q.CLI:'687de930ed499b2ced472af6e8ccba17b2b4837b6c282d60bdf5f3d01f958f78',q.IDS:'d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7',Path('/usr/bin/gdb'):'e2db863afa4d39c743d5057509aa18a9a6330d5b7ccd71a63ffe70c8b862810a'})
    q.GDB='/usr/bin/gdb';q.PYTHON='/usr/bin/python3.14'
    # All original logical/physical judges, controller, transport and Common
    # remain byte-identical. Only source/role bindings change here.
    origins=read(Q/'helper-origins.json')
    a.FILES={name:item['path'] for name,item in origins.items()}
    a.FILES.update({name:PREFIX+name for name in OWN})
    original_guard=old.guard
    def guard(cfg):
        source_guard();ledger=a_guard();original_guard(cfg)
        if sha(R/'lifecycle-control-v9/prepared.json')!=Q9_PREPARED or sha(Q/'q9-prepared-reference.json')!=Q9_PREPARED:raise ValueError('Q9 reference changed')
        if cfg.get('app')!=APP or cfg.get('prepared_kind')!='B_METADATA_ONLY_NOT_GO' or cfg.get('a_manifest')!=A_MANIFEST:raise ValueError('foreign B configuration')
        if ledger['artifacts']['rag3weaver-backend']['path']!=str(q.CLI) or ledger['artifacts']['typed-reader']['path']!=str(a.BINARY) or ledger['artifacts']['ids']['path']!=str(q.IDS):raise ValueError('B roles not bound to A ledger')
        for name,item in origins.items():
            if cfg['files'][name]!=item['sha256']:raise ValueError('qualified helper not byte-identical')
        if sha(Q/'lifecycle.py')!=cfg['files']['lifecycle.py']:raise ValueError('B adapter not prepared')
    old.guard=guard
    NativeReader=q.Reader
    class AttestedReader(NativeReader):
        def query(self,sql,collect,callback=None):
            if self.counter==0:
                profile={key:value for key,value in self.env.items() if key!='RAG3WEAVER_VALIDATION_JETON'}
                return reader_runtime.first_query(self,q,profile,sql,collect,callback)
            return super().query(sql,collect,callback)
    q.Reader=AttestedReader
    original_driver=old.driver
    def driver():
        guard(read(Q/'prepared.json'));ledger=a_guard()
        # Before clone, DB, FTS or any ELF invocation; artifacts themselves stay A.
        old.save('ledger-before-first-B-invocation.json',dict(app=APP,prepared_sha256=sha(Q/'prepared.json'),a_manifest=A_MANIFEST,a_ledger_sha256=A_LEDGER,artifacts=ledger['artifacts'],runtime_backend_qualified=False,live_authorized=False))
        return original_driver()
    old.driver=driver
    original_final=old.finalize_composed
    def final(record):
        if record.get('status')=='LIFECYCLE_FIXTURE_OBSERVED':
            try:
                cfg=read(P/'reader-sdk/config.json')
                reader_runtime.validate_capture(cfg,*[read(P/'reader-sdk'/name) for name in ['capture-exec.json','capture-main.json','native-exit.json']])
                guard(read(Q/'prepared.json'));old.postwriter_scope(record['deadline'])
                record.update(app=APP,a_manifest=A_MANIFEST,backend_sha256=q.PINS[q.CLI],scope_of_result='CLI99_PRIVATE_FIXTURE_RUNTIME_ONLY',reader_sdk_scope='FIRST_TYPED_QUERY_MAIN_LIB59_ONLY',reader_query_qualified=False,portable_qualified=False)
            except BaseException as error:
                a.red(error);record=dict(status='RED',error=str(error),live_authorized=False)
        return original_final(record)
    old.finalize_composed=final
    return old

def prepare():
    here=Path(__file__).resolve().parent;root=here.parents[3]
    if os.uname().nodename.lower()!='luciepc' or git(root,'status','--porcelain') or any(path.exists() or path.is_symlink() for path in [P,Q,S]):raise ValueError('PC clean source/new B metadata paths required')
    source_guard();a_guard()
    if sha(R/'lifecycle-control-v9/prepared.json')!=Q9_PREPARED or sha(here/'q9-prepared-reference.json')!=Q9_PREPARED:raise ValueError('Q9 metadata reference changed')
    cfg=read(here/'q9-prepared-reference.json');origins=read(here/'helper-origins.json');packet={}
    for name,item in origins.items():
        raw=subprocess.check_output(['/usr/bin/git','-C',str(root),'show',item['ref']+':'+item['path']])
        if hashlib.sha256(raw).hexdigest()!=item['sha256']:raise ValueError('qualified source hash differs')
        packet[name]=raw
    packet.update({name:(here/name).read_bytes() for name in OWN})
    # f.guard checks every packet against the new pilot tree as well as E9 pins.
    for name,item in origins.items():
        if subprocess.check_output(['/usr/bin/git','-C',str(root),'show','HEAD:'+item['path']])!=packet[name]:raise ValueError('new pilot changed qualified helper source')
    Q.mkdir(mode=0o700)
    for name,raw in packet.items():
        with (Q/name).open('xb') as out:out.write(raw)
        os.chmod(Q/name,0o600)
    cfg.update(source_root=str(root),pilot=git(root,'rev-parse','HEAD'),files={name:sha(Q/name) for name in packet},app=APP,prepared_kind='B_METADATA_ONLY_NOT_GO',a_manifest=A_MANIFEST)
    with (Q/'prepared.json').open('x') as out:json.dump(cfg,out,indent=2);out.write('\n');out.flush();os.fsync(out.fileno())
    os.chmod(Q/'prepared.json',0o600)
    print(json.dumps(dict(prepared_sha256=sha(Q/'prepared.json'),copied_controllers=len(packet),DB_opened=False,ELF_invoked=False,provider_contact=False)))

def main():
    ap=argparse.ArgumentParser();m=ap.add_mutually_exclusive_group(required=True)
    m.add_argument('--prepare',action='store_true');m.add_argument('--launch');m.add_argument('--compose',action='store_true');m.add_argument('--controller',action='store_true');m.add_argument('--driver',action='store_true')
    ap.add_argument('--allocation');ap.add_argument('--prepared-sha256');args=ap.parse_args()
    if args.prepare:return prepare() # Metadata path imports no native/old driver.
    if args.launch:
        if not args.prepared_sha256 or sha(Q/'prepared.json')!=args.prepared_sha256:raise ValueError('external prepared SHA required')
        if not args.allocation or args.allocation.startswith('PENDING'):raise ValueError('exact distinct B GO allocation required')
    cfg=read(Q/'prepared.json')
    for name,digest in cfg['files'].items():
        if Path(name).name!=name or sha(Q/name)!=digest:raise ValueError('packet changed before import')
    old=install()
    if args.launch:return old.launch(args.launch,args.allocation)
    if args.compose:return old.compose()
    try:
        if args.controller:return old.controller()
        old.driver();return 0
    except BaseException as error:old.a.red(error);raise

if __name__=='__main__':sys.exit(main())
