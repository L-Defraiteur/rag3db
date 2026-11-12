#!/usr/bin/env python3
"""Exerce le demandeur Rust et le fournisseur CPU par barrières ; aucun GPU.
Le processus annulé est maintenu en vie : le crash de l'activateur est hors preuve.
"""
import argparse
import csv
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time


def read_events(path):
    if not path.exists(): return []
    text=path.read_text()
    # Seule la dernière ligne complète peut être jugée pendant l'écriture.
    return [json.loads(x) for x in text.splitlines() if x] if text.endswith('\n') else [json.loads(x) for x in text.rsplit('\n',1)[0].splitlines() if x]


def read_client(path):
    if not path.exists(): return []
    text=path.read_text()
    lines=text.splitlines() if text.endswith('\n') else text.rsplit('\n',1)[0].splitlines()
    return [row for row in csv.reader(lines, delimiter='\t') if len(row)==5]


def wait(predicate,deadline,processes):
    while time.monotonic()<deadline:
        if predicate(): return
        if any(p.poll() not in (None,0) for p in processes): raise RuntimeError('demandeur en échec, lire stderr')
        time.sleep(.005)
    raise TimeoutError('condition non observée avant échéance commune')


def scenario(root,driver,fixture,cancel):
    root.mkdir(exist_ok=False)
    events=root/'fournisseur.jsonl'; charge=root/'chargement.go';warm=root/'chauffe.go';end=root/'fin.go'
    # La primitive actuelle requiert une adresse fixe. La fermeture de cette
    # réservation laisse une course ; une occupation concurrente est un rouge,
    # jamais une permission de tuer l'occupant ou de rejouer sur un autre port.
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1',0));address='127.0.0.1:'+str(reservation.getsockname()[1])
    processes=[];streams=[];begin=time.monotonic();deadline=begin+15
    proof={'cancel_a':cancel,'address':address,'timebase':'durées contrôleur ; traces Rust locales à chaque demandeur','observations':[]}
    def observed(event):proof['observations'].append(dict(event=event,elapsed_s=time.monotonic()-begin))
    try:
        for client in ['A','B']:
            out=(root/(client+'.stdout')).open('x');err=(root/(client+'.stderr')).open('x');streams.extend([out,err])
            argv=[str(driver),'--adresse',address,'--coordination',str(root/'coordination'),'--fixture',str(fixture),
                  '--journal-service',str(events),'--generation',root.name,'--gate-chargement',str(charge),'--gate-chauffe',str(warm),
                  '--annulation',str(root/(client+'.annule')),'--fin',str(end),'--journal-client',str(root/(client+'.tsv')),
                  '--id',client,'--attend-annulation','oui' if cancel and client=='A' else 'non','--budget-ms','12000']
            processes.append(subprocess.Popen(argv,stdout=out,stderr=err,start_new_session=True))
            observed('demandeur_'+client+'_lance')
            if client=='A':wait(lambda:any(e['event']=='chargement_attente' for e in read_events(events)),deadline,processes)
        wait(lambda: any(r[3:] == ['phase','Activation'] for r in read_client(root/'A.tsv')) and any(r[3:] == ['phase','Coordination'] for r in read_client(root/'B.tsv')),deadline,processes)
        observed('A_active_B_coordonne_avant_liberation')
        if cancel:
            (root/'A.annule').touch()
            wait(lambda:any(r[3]=='attente_erreur' and 'annulée' in r[4] for r in read_client(root/'A.tsv')),deadline,processes)
            assert processes[0].poll() is None,'demandeur annulé maintenu en vie pour isoler annulation et crash'
            observed('A_annule_B_toujours_en_attente')
        charge.touch()
        wait(lambda:any(e['event']=='chauffe_attente' for e in read_events(events)),deadline,processes)
        assert not any(e['event']=='pret' for e in read_events(events))
        observed('chauffe_en_attente_sans_pret')
        warm.touch()
        required=['B'] if cancel else ['A','B']
        wait(lambda:all(any(r[3]=='usage_reussi' for r in read_client(root/(c+'.tsv'))) for c in required),deadline,processes)
        observed('usages_reussis')
        assert sum(e['event']=='constructeur_demarre' for e in read_events(events))==1,'un seul constructeur effectif'
        assert len({e['pid'] for e in read_events(events)})==1
        end.touch()
        for process in processes:assert process.wait(timeout=max(.1,deadline-time.monotonic()))==0
        proof.update(verdict='VERT',constructor_count=1,fixture_pid=read_events(events)[0]['pid'])
    except BaseException as exc:
        proof.update(verdict='ROUGE',error=repr(exc));raise
    finally:
        # Groupes privés des seuls demandeurs créés, dont leur fixture héritée.
        for process in processes:
            try:os.killpg(process.pid,signal.SIGTERM)
            except ProcessLookupError:pass
        stop_deadline=time.monotonic()+3
        while any(not any(e['event']=='arrete' for e in read_events(events)) for _ in [0]) and time.monotonic()<stop_deadline:time.sleep(.01)
        for process in processes:
            try:os.killpg(process.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            process.wait(timeout=2)
        for stream in streams:stream.close()
        proof['fixture_stop_seen']=any(e['event']=='arrete' for e in read_events(events))
        proof['elapsed_s']=time.monotonic()-begin
        (root/'preuve.json').write_text(json.dumps(proof,indent=2)+'\n')
    if not proof['fixture_stop_seen']:raise RuntimeError('arrêt fixture non observé')
    print(root.name,proof['verdict'],flush=True)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--demandeur',type=Path,required=True);p.add_argument('--preuves',type=Path,required=True)
    a=p.parse_args();a.preuves.mkdir(exist_ok=False)
    fixture=Path(__file__).with_name('fournisseur.py').resolve()
    scenario(a.preuves/'partage',a.demandeur.resolve(),fixture,False)
    scenario(a.preuves/'annulation',a.demandeur.resolve(),fixture,True)

if __name__=='__main__':main()
