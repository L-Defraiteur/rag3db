"""Opérations ciblées 7882 : preflight en lecture seule ; stop/restore sur GO séparé."""
import datetime, hashlib, json, os, pathlib, signal, socket, subprocess, sys, time, urllib.request
from attendre_arret import attendre_arret

H=pathlib.Path.home()
ROOT=H/'.cache/optimiseur-notes/banc-k10-12nov-7882-2'
ROOT.mkdir(parents=True,exist_ok=True)
EXE=H/'.local/bin/rag3weaver-embeddings'
CWD=H/'.cache/rag3db-mesure-g5/embed-7882'
SHA='65ec1d1db17d48ee53785cd55e72b9b77ca451d5bc4bb8197a9c1b7d040c595d'
ENV={'RAG3WEAVER_BURN_DEVICE_EMBEDDER':'gpu:0','RAG3WEAVER_EMBED_MODEL':'granite-278m','RAG3WEAVER_EMBEDDINGS_ADDR':'127.0.0.1:7882','RAG3WEAVER_REGIME':'plein'}
def now(): return datetime.datetime.now().astimezone().isoformat()
def log(msg): print(now(),msg,flush=True)
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def command(*args): return subprocess.check_output(args,text=True).strip()
def listening(port):
    return command('ss','-H','-ltnp',f'( sport = :{port} )')
def health():
    with urllib.request.urlopen('http://127.0.0.1:7882/sante',timeout=5) as r: return json.loads(r.read())
def identity(v):
    for k,x in {'modele':'granite-278m','dim':768,'dual':False,'sparse':False,'factice':False,'precision':'Flex32'}.items():
        assert v.get(k)==x, f'identite {k}: {v.get(k)!r}'
def gpu(pid):
    entries=[]
    for f in (pathlib.Path('/proc')/str(pid)/'fdinfo').iterdir():
        try: lines=f.read_text().splitlines()
        except OSError: continue
        d={k.strip():v.strip() for k,v in (x.split(':',1) for x in lines if x.startswith('drm-'))}
        if 'drm-pdev' in d: entries.append(d)
    selected=[x for x in entries if x.get('drm-driver')=='amdgpu' and x.get('drm-pdev')=='0000:04:00.0']
    assert selected, 'fd amdgpu card2 absent'
    assert not any(x.get('drm-pdev')=='0000:07:00.0' and int(x.get('drm-total-vram','0').split()[0])>0 for x in entries), 'VRAM card0 interdite'
    return entries
def proc_identity(pid):
    p=pathlib.Path('/proc')/str(pid)
    assert sha(p/'exe')==SHA, 'executable PID different'
    assert (p/'cwd').resolve()==CWD
    assert (p/'cmdline').read_bytes().rstrip(b'\0').split(b'\0')==[bytes(EXE),b'--adresse',b'127.0.0.1:7882']
    environment=dict(x.split(b'=',1) for x in (p/'environ').read_bytes().split(b'\0') if b'=' in x)
    for k,v in ENV.items(): assert environment.get(k.encode())==v.encode(), 'env selectionne different: '+k
    return (p/'stat').read_text().rsplit(')',1)[1].split()[19]
def other_services():
    return {str(p):listening(p) for p in [7878,7879,7880]}
mode=sys.argv[1]
assert sha(EXE)==SHA
assert pathlib.Path('/sys/class/drm/card2/device').resolve().name=='0000:04:00.0'
if mode in ['preflight','stop']:
    pid=3273544
    start=proc_identity(pid)
    listener=listening(7882)
    assert 'pid='+str(pid)+',' in listener
    clients=command('ss','-H','-tnp','state','established','( sport = :7882 or dport = :7882 )')
    assert not clients,'clients TCP presents'
    v=health(); identity(v)
    proof={'time':now(),'pid':pid,'starttime':start,'sha256':SHA,'argv':[str(EXE),'--adresse','127.0.0.1:7882'],'cwd':str(CWD),'env':ENV,'health':v,'gpu_fdinfo':gpu(pid),'clients':clients,'listener':listener,'other_services':other_services()}
    name='preflight.json' if mode=='preflight' else 'juste-avant-arret.json'
    (ROOT/name).write_text(json.dumps(proof,indent=2)+'\n')
    log(json.dumps(proof,ensure_ascii=False))
    if mode=='preflight': sys.exit(0)
    assert len(sys.argv)==3 and sys.argv[2]=='GO-orchestration', 'GO distinct requis'
    assert not (ROOT/'arret.json').exists(), 'arret deja consigne'
    assert proc_identity(pid)==start,'PID reutilise'
    deadline=time.monotonic()+60
    os.kill(pid,signal.SIGTERM)
    attendre_arret(pid,start,deadline=deadline)
    assert other_services()==proof['other_services'],'autre service modifie'
    after={'time':now(),'pid_stopped':pid,'other_services':other_services(),'verdict':'VERT'}
    (ROOT/'arret.json').write_text(json.dumps(after,indent=2)+'\n')
    log(json.dumps(after))
elif mode=='restore':
    assert (ROOT/'arret.json').is_file(), 'arret non atteste'
    assert not (ROOT/'retour.json').exists(),'retour deja consigne'
    assert not listening(7882), '7882 occupe : aucun remplacement'
    assert not listening(8099), 'llama8099 encore present'
    before=other_services()
    for port in before: assert before[port], 'service principal absent'
    logfile=ROOT/'restauration-service.log'
    with logfile.open('xb') as out:
        proc=subprocess.Popen([str(EXE),'--adresse','127.0.0.1:7882'],cwd=CWD,env=dict(os.environ,**ENV),stdin=subprocess.DEVNULL,stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
    (ROOT/'pid-restauration').write_text(str(proc.pid)+'\n')
    log('restauration lancee PID '+str(proc.pid))
    try:
        for _ in range(120):
            assert proc.poll() is None, 'processus de retour termine prematurement'
            try: v=health(); break
            except OSError: time.sleep(.5)
        else: raise RuntimeError('pas de sante en60s')
        identity(v)
        start=proc_identity(proc.pid)
        fd=gpu(proc.pid)
        assert 'pid='+str(proc.pid)+',' in listening(7882)
        assert other_services()==before, 'autre service modifie'
        result={'time':now(),'pid':proc.pid,'starttime':start,'sha256':SHA,'health':v,'gpu_fdinfo':fd,'other_services':before,'verdict':'VERT'}
        (ROOT/'retour.json').write_text(json.dumps(result,indent=2)+'\n')
        log(json.dumps(result,ensure_ascii=False))
    except BaseException:
        log('ECHEC restauration : arret du seul processus nouvellement lance')
        proc.terminate()
        raise
else: raise ValueError(mode)
