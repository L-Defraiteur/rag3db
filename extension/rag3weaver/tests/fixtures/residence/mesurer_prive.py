#!/usr/bin/env python3
"""Banc privé : ModelSource réelle, annulation, attache chaude et reprise.
Lancer sous poste mesure après publication du plan figé. Aucun modèle à l'import.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.request
from observation_gpu import capture, start

GIB=1024**3; MIB=1024**2; PCI='0000:04:00.0'; SEAT='0000:07:00.0'


def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for b in iter(lambda:f.read(4*MIB),b''):h.update(b)
    return h.hexdigest()


def rows(path):
    if not path.is_file():return []
    raw=path.read_bytes();end=raw.rfind(b'\n')
    return [json.loads(line) for line in raw[:end+1].splitlines()] if end>=0 else []


def memory_guard(c):
    for client in c['clients']:
        if client['pci'] not in (PCI,SEAT):raise RuntimeError('PCI amdgpu inattendu')
    seat=[c for c in c['clients'] if c['pci']==SEAT]
    for key in ['vram_bytes','gtt_bytes']:
        if sum(c[key] for c in seat if c[key] is not None)>16*MIB:raise RuntimeError('mémoire seat1 dépasse16Mio')
    return 'INCONNUS' if not seat or any(not c['engine_ns'] for c in seat) else 'OBSERVABLES'


def health_guard(h,source,recipe):
    a=h.get('activation')
    if h.get('modele')!=source['model'] or h.get('dim')!=source['dimensions'] or h.get('factice') is not False:
        raise RuntimeError('identité modèle/dimension/factice non conforme')
    if not isinstance(a,dict) or a.get('profil')!=source['activation']['profil'] or a.get('peripherique')!=source['activation']['peripherique'] or a.get('recette')!=recipe:
        raise RuntimeError('attestation de recette non conforme')
    reports=h.get('chauffe')
    chauffe=reports.get('embed') if isinstance(reports,dict) else None
    if not isinstance(chauffe,dict) or chauffe.get('etat') not in ('complete','partielle') or not isinstance(chauffe.get('formes'),list):
        raise RuntimeError('chauffe non observée')
    if not chauffe['formes'] or any(not isinstance(f,dict) for f in chauffe['formes']):raise RuntimeError('formes de chauffe absentes')
    errors=sum(f.get('erreur') is not None for f in chauffe['formes'])
    if chauffe['etat']=='complete' and errors:raise RuntimeError('chauffe déclarée complète avec erreurs')
    if chauffe['etat']=='partielle' and not errors:raise RuntimeError('chauffe partielle sans erreur rapportée')
    return chauffe


def members(group,minimum):
    out=[]
    for p in Path('/proc').iterdir():
        if not p.name.isdigit():continue
        try:
            if p.stat().st_uid!=os.getuid():continue
            x=(p/'stat').read_text().rsplit(')',1)[1].split()
            if int(x[2])==group:
                if int(x[3])!=group or int(x[19])<minimum:raise RuntimeError('groupe non attribuable')
                if x[0]!='Z':out.append(int(p.name))
        except (FileNotFoundError,ProcessLookupError):pass
    return out


class Actor:
    def __init__(self,exe,config,folder):
        self.files=[(folder/'stdout').open('x'),(folder/'stderr').open('x')]
        env=dict(os.environ,ROCR_VISIBLE_DEVICES='0')
        env.pop('HIP_VISIBLE_DEVICES',None);env.pop('CUDA_VISIBLE_DEVICES',None)
        self.proc=subprocess.Popen([exe,str(config)],stdout=self.files[0],stderr=self.files[1],env=env,start_new_session=True)
        self.pid=self.proc.pid;self.birth=start(Path('/proc')/str(self.pid))
    def stop(self,deadline):
        if members(self.pid,self.birth):
            try:os.killpg(self.pid,signal.SIGTERM)
            except ProcessLookupError:pass
        until=min(time.monotonic()+20,deadline)
        while members(self.pid,self.birth) and time.monotonic()<until:
            self.proc.poll();time.sleep(.1)
        if members(self.pid,self.birth):os.killpg(self.pid,signal.SIGKILL)
        self.proc.wait(timeout=5)
        if members(self.pid,self.birth):raise RuntimeError('groupe privé restant')
        for f in self.files:f.close()


class Run:
    def __init__(self,plan_path,folder):
        self.plan=json.loads(plan_path.read_text());self.root=folder;folder.mkdir(exist_ok=False)
        self.begin=time.monotonic();self.end=self.begin+1200;self.actors={};self.active=set();self.index=0;self.last_capture=0
        self.baseline=None;self.peak=0;self.source=None;self.recipe=None;self.provider_ids=set();self.engine_baselines={};self.stopped=set();self.expected_activations=1
        self.gpu=Path('/sys/class/drm/card2/device');self.exec_log=folder/'executions.jsonl';self.gate=folder/'exec.go'
    def event(self,name,**data):
        data.update(elapsed_s=round(time.monotonic()-self.begin,6),utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()))
        (self.root/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n');print(name,json.dumps(data,ensure_ascii=False),flush=True)
    def memory(self):
        used=int((self.gpu/'mem_info_vram_used').read_text());total=int((self.gpu/'mem_info_vram_total').read_text());self.peak=max(self.peak,used)
        if self.baseline is not None and (used-self.baseline>8*GIB or total-used<2*GIB):raise RuntimeError('budget global card2 dépassé')
        return used,total
    def client(self,id,cancel=False,reprise=False):
        d=self.root/id;d.mkdir()
        config=dict(id=id,source=self.source,journal=str(d/'events.jsonl'),annulation=str(d/'annule.go'),
                    fin=str(self.root/('fin-reprise.go' if reprise else 'fin-premier.go')),budget_ms=1100000,
                    attend_annulation=cancel,attend_factice=False,textes=['un document source court','une mémoire utile'],
                    gate_usage=str(self.root/('usage-reprise.go' if reprise else 'usage-premier.go')))
        p=d/'config.json';p.write_text(json.dumps(config,indent=2)+'\n')
        self.actors[id]=Actor(self.plan['consumer']['path'],p,d)
        self.event('acteur-'+id,pid=self.actors[id].pid,starttime=self.actors[id].birth)
    def events(self,id):return rows(self.root/id/'events.jsonl')
    def has(self,id,event):return any(r['event']==event for r in self.events(id))
    def phase(self,id,name):return any(r['event']=='phase' and r['details'].get('phase')==name for r in self.events(id))
    def tick(self):
        self.memory()
        for id,a in self.actors.items():
            if a.proc.poll() not in (None,0):raise RuntimeError('acteur en échec '+id)
        executions=rows(self.exec_log)
        if any(e['event']=='erreur_avant_exec' for e in executions):raise RuntimeError('barrière privée refusée')
        declared=[e for e in executions if e['event']=='exec_fournisseur']
        if len(declared)>self.expected_activations or sum(e['event']=='barriere_avant_exec' for e in executions)>self.expected_activations:raise RuntimeError('activation réelle supplémentaire')
        if time.monotonic()-self.last_capture<1:return
        self.last_capture=time.monotonic()
        for entry in declared:
            pid=entry['pid'];identity=(pid,entry['starttime']);proc=Path('/proc')/str(pid)
            if identity in self.stopped:continue
            if not proc.exists():raise RuntimeError('fournisseur disparu avant arrêt prévu')
            group=os.getpgid(pid)
            if group not in {a.pid for a in self.actors.values()}:raise RuntimeError('fournisseur hors groupes possédés')
            # L'événement précède exec de quelques instructions, sans GPU avant exec.
            try:
                if not os.path.samefile(proc/'exe',self.plan['artifacts']['provider']['path']):continue
            except FileNotFoundError:continue
            birth=start(proc)
            if birth!=entry['starttime']:raise RuntimeError('PID fournisseur réutilisé')
            self.provider_ids.add((pid,birth));self.active.add((pid,birth))
            c=capture(pid,birth,allow_partial=True);c['argv']=[a.decode(errors='replace') for a in c['argv']]
            (self.root/f'gpu-{self.index:05d}.json').write_text(json.dumps(c,indent=2)+'\n');self.index+=1
            if c['exe_sha256']!=self.plan['artifacts']['provider']['sha256']:raise RuntimeError('exécutable chargé étranger')
            memory_guard(c)
            for client in c['clients']:
                if client['pci']==SEAT:
                    key=(pid,birth,client['client_id']);before=self.engine_baselines.setdefault(key,dict(client['engine_ns']))
                    if before and client['engine_ns']!=before:raise RuntimeError('compteurs seat1 modifiés')
    def wait(self,predicate,deadline,label):
        while time.monotonic()<min(deadline,self.end):
            self.tick()
            if predicate():return
            time.sleep(.05)
        raise TimeoutError(label)
    def qualify(self,reprise):
        self.last_capture=0;self.tick()
        current=[]
        for pid,birth in self.active:
            try:
                if start(Path('/proc')/str(pid))==birth:current.append((pid,birth))
            except FileNotFoundError:pass
        if len(current)!=1:raise RuntimeError('instance privée active non unique')
        pid,birth=current[0]
        c=capture(pid,birth,allow_partial=False);c['argv']=[x.decode(errors='replace') for x in c['argv']]
        memory_guard(c)
        if any(x[f] is None for x in c['clients'] if x['pci']==SEAT for f in ['vram_bytes','gtt_bytes']):raise RuntimeError('mémoire seat1 inconnue avant usage')
        on=[x for x in c['clients'] if x['pci']==PCI]
        if not on or any(x['vram_bytes'] is None or x['gtt_bytes'] is None for x in on) or sum(x['vram_bytes']+x['gtt_bytes'] for x in on)<128*MIB:
            raise RuntimeError('allocation du modèle sur card2 non prouvée')
        with urllib.request.urlopen('http://'+self.source['address']+'/sante',timeout=3) as r:h=json.load(r)
        declarations=[e for e in rows(self.exec_log) if e['event']=='barriere_avant_exec' and e['pid']==pid]
        self.recipe=declarations[-1]['declaration']['RAG3WEAVER_ACTIVATION_IDENTITE']
        chauffe=health_guard(h,self.source,self.recipe)
        self.event('qualification-'+('reprise' if reprise else 'premier'),pid=pid,starttime=birth,sante=h,chauffe=chauffe,capture=c,
                   placement='SOURCE_SELECTION_FD_ALLOCATION_PCI04',compteurs_seat1='inconnus si absents, jamais zéro')
        (self.root/('usage-reprise.go' if reprise else 'usage-premier.go')).write_text('preuve de placement et santé conservée\n')
        return pid,birth
    def stop(self,ids,label,graceful=True):
        (self.root/'fin-premier.go').touch(exist_ok=True)
        (self.root/'fin-reprise.go').touch(exist_ok=True)
        until=min(time.monotonic()+60,self.end)
        errors=[]
        for id in ids:
            if id not in self.actors:continue
            actor=self.actors[id]
            try:
                code=actor.proc.wait(timeout=max(.01,min(5,until-time.monotonic())))
                if graceful and code!=0:errors.append('demandeur '+id+' code '+str(code))
            except subprocess.TimeoutExpired:
                if graceful:errors.append('demandeur '+id+' ne rend pas son usage')
        if graceful and not errors and self.source is not None:
            try:
                with urllib.request.urlopen('http://'+self.source['address']+'/sante',timeout=3) as r:h=json.load(r)
                health_guard(h,self.source,self.recipe)
                self.event('maintenu-sans-clients-'+label,sante=h)
            except BaseException as error:errors.append('maintien après fin des clients : '+repr(error))
        for id in ids:
            if id in self.actors:
                try:self.actors[id].stop(until)
                except BaseException as error:errors.append('arrêt groupe '+id+' : '+repr(error))
        for pid,birth in list(self.active):
            p=Path('/proc')/str(pid)
            if not p.exists():self.active.discard((pid,birth));self.stopped.add((pid,birth));continue
            try:
                x=(p/'stat').read_text().rsplit(')',1)[1].split()
                if x[0]=='Z':self.active.discard((pid,birth));self.stopped.add((pid,birth));continue
            except FileNotFoundError:self.active.discard((pid,birth));self.stopped.add((pid,birth));continue
            errors.append('fournisseur reste actif '+str(pid))
        if self.source is not None:
            # Le port et les objets du pilote peuvent être rendus après exit.
            while time.monotonic()<until:
                with socket.socket() as sock:
                    sock.settimeout(.2);occupied=sock.connect_ex(('127.0.0.1',int(self.source['address'].split(':')[1])))==0
                used,total=self.memory()
                if not occupied and (self.baseline is None or used<=self.baseline+256*MIB):break
                time.sleep(.1)
            else:errors.append('port ou mémoire privée non rendus dans la borne')
        used,total=self.memory()
        self.event(label,used=used,baseline=self.baseline,peak=self.peak,erreurs=errors)
        if errors:raise RuntimeError('; '.join(errors))
    def run(self):
        failed=None
        def interrupt(sig,frame):raise RuntimeError('signal '+str(sig))
        for sig in (signal.SIGALRM,signal.SIGTERM,signal.SIGINT):signal.signal(sig,interrupt)
        signal.setitimer(signal.ITIMER_REAL,1200)
        self.event('mesure-acquise',pid=os.getpid(),budget_s=1200,deadline_unix=time.time()+1200)
        try:
            for name,a in self.plan['artifacts'].items():
                if sha(a['path'])!=a['sha256']:raise RuntimeError('artefact modifié '+name)
            for file,digest in self.plan['scripts'].items():
                if sha(Path(__file__).parent/file)!=digest:raise RuntimeError('script modifié '+file)
            if self.gpu.resolve().name!=PCI:raise RuntimeError('card2 PCI inattendu')
            self.baseline,total=self.memory()
            if total-self.baseline<10*GIB:raise RuntimeError('moins10Gio libres pour le modèle privé')
            with socket.socket() as reserve:
                reserve.bind(('127.0.0.1',0));address='127.0.0.1:'+str(reserve.getsockname()[1])
            (self.root/'journaux').mkdir()
            self.source=dict(provider='service',model=self.plan['model'],address=address,dimensions=384,timeout_secs=30,
                activation=dict(programme=str(Path(__file__).with_name('activation_privee.py').resolve()),
                  args=['--plan',str(self.plan_path),'--autoriser',str(self.gate),'--journal',str(self.exec_log),'--budget-ms','300000'],
                  env=self.plan['env'],profil='banc-residence-107m-384-'+self.root.name,peripherique='pci:0000:04:00.0',
                  coordination=str(self.root/'coordination'),journal=str(self.root/'journaux'),budget_activation_secs=300,budget_attente_secs=360,residence='maintenir'))
            self.event('plan-effectif',source=self.source,plan=self.plan,baseline=self.baseline)
            self.client('A',cancel=True)
            self.wait(lambda:any(e['event']=='barriere_avant_exec' for e in rows(self.exec_log)),self.begin+60,'barrière A')
            self.client('B')
            self.wait(lambda:self.phase('A','Activation') and self.phase('B','Coordination'),self.begin+90,'A activation/B coordination')
            (self.root/'A'/'annule.go').touch()
            self.wait(lambda:self.has('A','attente_erreur'),self.begin+100,'annulation A')
            errors=[e for e in self.events('A') if e['event']=='attente_erreur']
            if not any('annulée' in e['details'].get('cause','') for e in errors):raise RuntimeError('A pas annulé pour la cause attendue')
            self.gate.write_text(json.dumps(dict(ok=True,source='A annulé et B en Coordination observés'))+'\n')
            self.wait(lambda:self.has('B','usage_en_attente'),self.begin+360,'acquisition et chauffe B')
            first=self.qualify(False)
            self.wait(lambda:self.has('B','usage_reussi'),min(time.monotonic()+60,self.begin+420),'usage B')
            self.client('C')
            self.wait(lambda:self.has('C','usage_reussi'),min(time.monotonic()+60,self.begin+480),'attache chaude C')
            if len([e for e in rows(self.exec_log) if e['event']=='exec_fournisseur'])!=1:raise RuntimeError('second chargement avant reprise')
            self.event('partage-annulation-chaud-vert',provider=first)
            self.stop(['A','B','C'],'premier-prive-arrete')
            self.expected_activations=2
            self.client('D',reprise=True)
            self.wait(lambda:self.has('D','usage_en_attente'),min(time.monotonic()+360,self.end-180),'reprise D')
            second=self.qualify(True)
            if second==first:raise RuntimeError('reprise sans nouvelle instance')
            self.wait(lambda:self.has('D','usage_reussi'),min(time.monotonic()+60,self.end-120),'usage D')
            if len([e for e in rows(self.exec_log) if e['event']=='exec_fournisseur'])!=2:raise RuntimeError('nombre activations différent de2')
            self.event('reprise-verte',premier=first,second=second)
        except BaseException as error:
            failed=repr(error);self.event('defaut',cause=failed)
        finally:
            signal.setitimer(signal.ITIMER_REAL,0)
            try:self.stop(list(self.actors),'fin-prives-arretes',graceful=failed is None)
            except BaseException as error:
                self.event('BLOCAGE-arret',cause=repr(error));failed=failed or repr(error)
            self.event('fin',failed=failed,provider_instances=sorted(self.provider_ids),peak=self.peak)
        return 0 if failed is None else 1


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--plan',type=Path,required=True);p.add_argument('--preuves',type=Path,required=True)
    a=p.parse_args();r=Run(a.plan.resolve(),a.preuves.resolve());r.plan_path=a.plan.resolve();return r.run()
if __name__=='__main__':raise SystemExit(main())
