"""Capture /proc d'un serveur connu ; aucun lancement ni appel GPU.
Usage: python3 garde_charge.py PID STARTTIME DOSSIER_NEUF
Produit une capture unique pour une boucle opérateur ; aucune valeur absente
n'est remplacée par zéro. Le processus appelant doit décider arrêt/restauration.
"""
import hashlib,json,os,re,sys,time
from pathlib import Path

def parse(raw):
    return dict(line.split(':',1) for line in raw.splitlines() if ':' in line)

def memory(fields,preferred,fallback):
    value=fields.get(preferred,fields.get(fallback))
    if value is None: return None
    m=re.fullmatch(r'\s*(\d+)\s+(KiB|kB|MiB|B)\s*',value)
    if not m: raise ValueError('unité mémoire inconnue')
    return int(m[1])*{'KiB':1024,'kB':1024,'MiB':1024**2,'B':1}[m[2]]

def start(proc): return int((proc/'stat').read_text().rsplit(')',1)[1].split()[19])

def gpu_fds(proc):
    found={}
    for fd in (proc/'fd').iterdir():
        try: target=os.readlink(fd)
        except FileNotFoundError: continue
        if target.startswith('/dev/dri/') or target=='/dev/kfd': found[fd.name]=target
    return found

def capture(pid,expected,allow_partial=False):
    proc=Path('/proc')/str(pid)
    if start(proc)!=expected: raise RuntimeError('PID réutilisé')
    exe=os.readlink(proc/'exe')
    h=hashlib.sha256()
    with (proc/'exe').open('rb') as f:
        for block in iter(lambda:f.read(1024**2),b''): h.update(block)
    env={}
    for item in (proc/'environ').read_bytes().split(b'\0'):
        k,_,v=item.partition(b'=')
        if k in (b'ROCR_VISIBLE_DEVICES',b'HIP_VISIBLE_DEVICES',b'CUDA_VISIBLE_DEVICES'):
            env[k.decode()]=v.decode(errors='replace')
    partial=[]
    before_gpu=gpu_fds(proc)
    fds=[]; clients={}
    for path in sorted((proc/'fdinfo').iterdir(),key=lambda p:int(p.name)):
        try:
            raw=path.read_text(); target=os.readlink(proc/'fd'/path.name)
        except FileNotFoundError:
            if path.name in before_gpu:
                if not allow_partial: raise RuntimeError('FD GPU disparu pendant capture')
                partial.append('FD GPU disparu '+path.name)
            fds.append(dict(fd=int(path.name),vanished_non_gpu=path.name not in before_gpu,vanished_gpu=path.name in before_gpu))
            continue
        fields={k:v.strip() for k,v in parse(raw).items()}
        fds.append(dict(fd=int(path.name),target=target,raw=raw))
        if fields.get('drm-driver')!='amdgpu': continue
        pci=fields.get('drm-pdev'); cid=fields.get('drm-client-id')
        if pci is None or cid is None:
            if not allow_partial: raise RuntimeError('identité client DRM absente')
            partial.append('identité client DRM absente '+path.name); continue
        engines={}
        for k,v in fields.items():
            if k.startswith('drm-engine-') and not k.startswith('drm-engine-capacity-'):
                m=re.fullmatch(r'(\d+)\s+ns',v)
                if not m: raise ValueError('unité moteur inconnue')
                engines[k]=int(m[1])
        client=dict(pci=pci,client_id=cid,vram_bytes=memory(fields,'drm-resident-vram','drm-memory-vram'),gtt_bytes=memory(fields,'drm-resident-gtt','drm-memory-gtt'),engine_ns=engines)
        key=(pci,cid)
        if key in clients and clients[key]!=client:
            if not allow_partial: raise RuntimeError('client change pendant capture')
            partial.append('client change pendant capture '+str(key))
            for field in ('vram_bytes','gtt_bytes'):
                values=[v for v in [clients[key][field],client[field]] if v is not None]
                client[field]=max(values) if values else None
        clients[key]=client
    after_gpu=gpu_fds(proc)
    if after_gpu!=before_gpu:
        if not allow_partial: raise RuntimeError('ensemble FD GPU modifié pendant capture')
        partial.append('ensemble FD GPU modifié')
    if start(proc)!=expected: raise RuntimeError('identité a changé pendant capture')
    return dict(complete=not partial,partial_reasons=partial,gpu_fd_before=before_gpu,gpu_fd_after=after_gpu,monotonic_ns=time.monotonic_ns(),pid=pid,starttime=expected,exe=exe,exe_sha256=h.hexdigest(),argv=(proc/'cmdline').read_bytes().split(b'\0')[:-1],env=env,fds=fds,clients=list(clients.values()))

if __name__=='__main__':
    pid,expected=int(sys.argv[1]),int(sys.argv[2]); out=Path(sys.argv[3]); out.mkdir(exist_ok=False)
    try:
        result=capture(pid,expected)
        result['argv']=[x.decode(errors='replace') for x in result['argv']]
        seat=[c for c in result['clients'] if c['pci']=='0000:07:00.0']
        known_mem=all(c['vram_bytes'] is not None and c['gtt_bytes'] is not None for c in seat)
        mem_ok=known_mem and sum(c['vram_bytes'] for c in seat)<=16*1024**2 and sum(c['gtt_bytes'] for c in seat)<=16*1024**2
        # L'absence de moteurs est INDETERMINE, même avec un contexte minuscule.
        observable=bool(seat) and all(c['engine_ns'] for c in seat)
        result['qualification']=dict(memory_within_proposed_budget=mem_ok,execution_counters_present=observable,verdict='A_COMPARER' if mem_ok and observable else 'INDETERMINE' if mem_ok else 'ROUGE')
        (out/'capture.json').write_text(json.dumps(result,indent=2)+'\n')
        print(result['qualification']['verdict'])
    except Exception as e:
        (out/'erreur.json').write_text(json.dumps(dict(error=repr(e)))+'\n'); raise
