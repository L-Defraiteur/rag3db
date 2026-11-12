from pathlib import Path
import json,os,subprocess,time,hashlib,sys
r=Path('/home/lucied/.cache/rag3weaver-build');p=r/'cx-passe-faits-lies-v6';w=r/'cx-wt-faits-lies';t=r/'cx-target-contenu-derive'
source='d2cb7f0636d099823f63de2cef674b630a46bc6f'
checks={};verdict='ROUGE';error=''
try:
    client_rc=int(sys.argv[1]);assert client_rc==0,('client systemd réel',client_rc)
    checks['client_systemd_rc']=client_rc
    assert (p/'VERDICT').read_text().strip()=='VERT'
    d=json.loads((p/'verdict-detaille.json').read_text());assert d['source']==source and d['verdict']=='VERT'
    assert d['elapsed_s']<3600
    # Le résultat réel du service est conservé par le client systemd --wait dans lancement.log.
    for unit in ['cx-passe-faits-lies-v6.service','cx-passe-faits-lies-v6.scope','cx-passe-faits-lies-v5.scope','cx-passe-faits-lies-v4.scope','cx-passe-faits-lies-v3.scope','cx-passe-faits-lies-v2.scope','cx-passe-faits-lies-v1.scope']:
        state=subprocess.run(['systemctl','--user','show',unit,'-p','ActiveState','-p','Result','-p','ExecMainStatus'],capture_output=True,text=True).stdout
        assert 'ActiveState=inactive' in state,(unit,state)
        checks[unit]=state
    tmp=json.loads((p/'tmp-retire.json').read_text());assert tmp['source']==source and tmp['absent'] and not Path(tmp['path']).exists()
    checks['tmp']=tmp
    assert not json.loads((p/'quiescence.json').read_text())['live']
    assert not json.loads((p/'quiescence.json').read_text())['context']
    assert not json.loads((p/'nav-quiescence.json').read_text())['live']
    assert not json.loads((p/'nav-quiescence.json').read_text())['context']
    leftovers=[]
    for q in Path('/proc').iterdir():
        if not q.name.isdigit() or int(q.name)==os.getpid():continue
        try:
            if q.stat().st_uid!=os.getuid():continue
            st=(q/'stat').read_text().rsplit(') ',1)[1].split()
            if st[0]=='Z':continue
            exe=os.readlink(q/'exe');cwd=os.readlink(q/'cwd');args=(q/'cmdline').read_bytes()
            if exe.startswith(str(t)+'/') or cwd==str(w) or cwd.startswith(str(w)+'/') or tmp['path'].encode() in args:leftovers.append(dict(pid=q.name,exe=exe,cwd=cwd))
        except OSError:continue
    assert not leftovers,leftovers
    checks['independent_proc_scan']=leftovers
    artifacts=json.loads((p/'artefacts.json').read_text());assert set(artifacts)=={'lib','navigateur','cli','backend'}
    for name,item in artifacts.items():
        with (p/('binaire-'+name)).open('rb') as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
        assert digest==item['sha256'],name
    checks['rehash_4_elf']='VERT'
    reused=json.loads((p/'verts-reutilises.json').read_text());assert reused['source']==source
    prior=json.loads((r/'cx-passe-faits-lies-v2/verdict-detaille.json').read_text());assert prior['verdict']=='ROUGE'
    checks['phases_reutilisees']=reused['phases']
    checks['phases_finales']=d['phases']
    lib4=json.loads((p/'lib-verte-reutilisee.json').read_text());assert lib4['counts']==[2222,0,65]
    checks['lib_reelle_v4']=lib4
    nav5=json.loads((p/'nav-verte-reutilisee.json').read_text());assert nav5['counts']==[40,0,0]
    checks['nav_reelle_v5']=nav5
    verdict='VERT'
except BaseException as e:error=repr(e)
(p/'VERDICT_COMPOSE').write_text(verdict+'\n')
(p/'verdict-compose.json').write_text(json.dumps(dict(source=source,verdict=verdict,error=error,checks=checks,driver_file_alone_insufficient=True,limitation='v1/v2/v3 ROUGE conservés; F5/min/compilation d2 repris v2, lib v4 env corrigé, NAV v5, CLI/attest v6; bus privés arrêtés après NAV et scan indépendant vide, aucune prétention de passe sans incident',monotonic=time.monotonic()),indent=2)+'\n')
print(verdict,error)
raise SystemExit(0 if verdict=='VERT' else 1)
