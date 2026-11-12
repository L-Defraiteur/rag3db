# Les témoins de lecture imposent le nom du dossier des journaux

- **État** : corrigé ; source487c68af3, verdict composé c94515115 relu VERT par orchestration le12novembre2026, tour master explicitement accordé.
- **Pour** : recherche, fixtures e2e_lectures ; aucun correctif de fixture dans le lot des accords.
- **Famille** : produit
- **Gravité** : coût ; une isolation valide des journaux fait échouer la passe avant les lectures.
- **Atteignable en service** : non, dépendance de montage des témoins.
- **Touche rag3weaver** : oui, témoins et configuration RAG3WEAVER_JOURNAUX_DIR.

## Constat et preuve

Sur `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`, `RAG3WEAVER_JOURNAUX_DIR` est posé sur
un dossier privé nommé `.../accord-20261112/journaux-passe`. L'outil index rend bien le
chemin du journal réel sous `journaux-passe/index/`, mais les témoins de lecture cherchent
une sous-chaîne littérale `commandes/index/` dans leur extraction de la réponse.

Sources : `extension/rag3weaver/tests/e2e_lectures.rs:279`, `:322`, `:683`, `:1460`.
Le produit compose le chemin depuis la variable dans `src/dataflow/run_nodes.rs::dossier_journaux`.
Résultat réel : **5 réussis, 19 échecs, 0 ignoré en 6,37 s**. Les 19 paniques portent
exactement « le journal de l'index » ; elles surviennent sur l'extraction du chemin,
pas sur une assertion de résultats de recherche.

Preuve luciepc, hors target :
`~/.cache/rag3weaver-build/I-luciepc/accord-20261112/passe-rouge-journaux-b42/`
(`e2e_lectures.log`, `rapports-e2e/`, source et verdict rouge conservés).
Même source et mêmes assertions, seule racine privée changée en `.../accord-20261112/commandes` :
**24 réussis, 0 échec, 0 ignoré en 9,57 s**, une suite jouée sur une demandée.
Preuve : `lectures-commandes.log` et `lectures-commandes-complet.log` dans le dossier ED voisin.
Ce montage compatible débloque le lot courant ; il ne corrige pas la dette de fixture.

## Suite et critères de fin

Réutiliser une extraction du chemin effectivement rendu, sans imposer le nom de sa racine
ni fabriquer un résultat ou abaisser les assertions de lecture. Chercher d'abord une primitive
commune de reçu/journal pour éviter quatre parseurs différents. Prouver la suite complète avec
une racine arbitraire hors dossier réel, sans le segment commandes, et garder les assertions
sur l'index et les lectures. Le ciblé et la passe restent requis selon le périmètre du correctif.

Avant toute globale mémoire contenant lectures24, un montage privé `.../commandes` évite ce
rouge identifié. ED transmet ce ticket et cette précondition à mémoire et orchestration.

## Reprise recherche et montage préparé

Reprise explicitement autorisée par orchestration, worktree/branche neuf `cx-recherche-lectures-journaux`, base master `af2ef6846`. Ticket repris depuis le blob35c23c2a6 sans embarquer le lot ED. Périmètre limité à `tests/e2e_lectures.rs` : aucun runtime, manifeste, ED/M8 ou service modifié.

La relecture des contrats existants trouve le reçu dans IndexNode : chaîne Markdown avec ligne `journal : …`, ou objet JSON à champ `journal`. Le backend enveloppe dans `result`, l’hôte ajoute une enveloppe `result` ; aucun helper de reçu/journal commun aux témoins consultés ne fait cette extraction. Neuf parseurs locaux découpaient la sérialisation JSON par guillemets, antislashs et espaces puis exigeaient `commandes/index/`.

Un helper local unique lit maintenant la Value réelle, descend les enveloppes `result`, puis extrait le champ/ligne désigné. Il préserve les espaces et échappements ; il exige toujours un journal `.out` sous `index`. Les neuf usages le composent, sans changer leurs boucles bornées de fin/échec ni les assertions index/recherche. Trois tests de helper couvrent les deux formats, zéro/une/deux enveloppes et l’aller-retour JSON avec chemin arbitraire/espaces/accents/guillemets/antislash ; les refus d’une simple mention de chemin et d’un autre journal restent explicites. Ces tests non ignorés se jouent séparément ; la suite ignorée existante conserve ses24 témoins.

Joué à cette étape : diff --check et parsing rustfmt vers copie externe, pas de compilation ni exécution native. Source à publier/figer avant test. Plan ciblé : après coordination ED/M8 et service7879, poste lourd lib29 complète/-j8/Mem32/target97k privé plafonné32Go ; compilation bornée1500s puis tests du helper60s et lectures27 (24 existants + 3 helpers, aucun filtre ajouté) borné120s sous racine privée arbitraire ne contenant pas le segment `commandes`, global1800s. Logs hors target, source/ELF/native/gabarits attestés. Aucun service/GPU/modèle nouveau ni e2e entier ; service existant uniquement après accord de créneau, premier autre rouge stop sans retry. Diff et preuves à revoir avant globale/master, aucun Rust poussé pendant les globales ED/M8.

## Préfixe exécuté, avant ciblé utilisant le service

Source figée propre et publiée `aa58d16979f20c00bdcff060f49524697c72a8df`, base `af2ef6846`. Préfixe explicitement autorisé après revue du diff : compilation1500s + helpers60s, global1800s incluant l’attente du service et le futur ciblé. Départ12novembre15:48:55 CET, échéance16:18:55. Wrapper2563127, timeout2563159, Python2563160. Tous SERVICE_* et EMBED_SERVICE retirés ; aucun appel au service durant ce préfixe. Scope réel `run-p2563159-i82252833.scope`, `memory.max=34359738368` vérifié pour le lanceur et ses enfants, cible privée target97k neuve et plafonnée32Go.

Compilation rc0 en4min52s ; helpers **3 réussis,0 échec,0 ignoré,857 filtrés**, sans rejouer. Le préfixe s’arrête ensuite ROUGE sur une erreur de son instrumentation, pas du produit : section readelf `.rag3db_native_build` erronée. Le nom réel `.rag3db_compiled_against` a été confirmé et l’attestation autonome seule autorisée par orchestration, sans rejeu compilation/tests et sans prolonger l’échéance. Sa première version cherchait par appartenance mmap au lieu de find : parcours mal écrit et lent, arrêté par pidfd sur le seul PID2655739 ; erreur et arrêt conservés. Version V2 corrigée et bornée120s exécutée en moins d’une seconde : **VERTE**, lib29 complète exacte et **220 gabarits présents**. ELF `target-97k/debug/deps/e2e-f9dffd07ba40d3a6`, SHA256 `729f0d114c65c867e44089219ab4b48e7e44776b327279d41ed99312ad94da1a`. Quiescence du target vérifiée, lourd rendu. Aucun test ni compilation rejoué pour corriger ces outils.

Preuves brutes deux postes : `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/` : source.json, argv.json, compilation.log, helpers.log, resume.log, verdict-prefixe.log (rouge conservé), cgroup-prefixe.json, arret-attestation-autonome.json, attestation-autonome-v2.log, attestation-binaire-corrigee.json, identite-native-corrigee.txt, verdict-attestation-corrigee.log, quiescence-prefixe.json. Les compteurs et verdicts d’outillage restent séparés.

M8 a confirmé ses8suites100/0/0 terminées et la quiescence de son usage7879. Le ciblé **27** (24 existants + 3 helpers) attend la coordination ED/C7, avec contrôle /sante réel, poste lourd effectif, BUILD_MEMORY_HIGH=0 pour éviter un scope frère hors MemoryMax extérieur, et vérification des cgroups enfants. Racine privée avec espace `racine arbitraire`, sans segment commandes. Aucun master avant revue, ED/M8 prioritaires pour l’intégration ; ticket toujours ouvert.

## Ciblé de lecture terminé — verdict soumis à revue

Après confirmation explicite C7 de la fenêtre ED/M8 (M8 terminé, ED libre/non lancé ; ordre recherche27 puis ED2), /sante7879 réellement relue : service rag3weaver-embeddings, modèle bge-m3, dim1024, sparse/dual vrais, factice faux. Le service existant n’a été ni démarré, ni arrêté, ni modifié par recherche.

Source distante toujours figéeaa58d16979f20c00bdcff060f49524697c72a8df et ELF729f0d114c65c867e44089219ab4b48e7e44776b327279d41ed99312ad94da1a inchangé avant/après. Lancement wrapper2687989 sous vrai poste lourd, marker seulement dans cette portée, MemoryMax32G et BUILD_MEMORY_HIGH=0. Les cgroups enfants observés restent dans la portée bornée ; argv/env/ascendance conservés. Compilation/tests du préfixe n’ont pas été rejoués pour corriger l’outillage.

Un appel `run_e2e.sh --test e2e_lectures`, borne120s : **27 réussis,0 échec,0 ignoré en9,46s**, soit24 existants +3 helpers, sans filtre/skip ajouté. Les2filets préalables sont verts. Racine réelle privée `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/racine arbitraire`, avec espace et sans segment commandes : les journaux `.out` sous index y sont recensés dans preuves-racine.json. Toutes les attentes de fin/échec d’indexation et assertions de lecture/recherche existantes sont conservées et jouées.

Fin vers15:59:20 CET, avant échéance globale16:18:55, sans extension de budget. Vérification /proc exe target97k ou cwd worktree : **liste vide** dans quiescence-lectures.json ; santé7879après relue OK. Fin et disponibilité du service annoncées directement ED/M8/C7 ; aucun autre usage recherche prévu. Aucun nouveau modèle, GPU ou service déployé, aucun e2e entier, aucune CI relancée.

Preuves ajoutées aux journaux sur les deux postes : sante-7879-avant/apres.json, lancement-lectures.log, argv-env-lectures.json, cgroup-enfants-lectures.json, lectures.log, e2e-complet.log, preuves-racine.json, verdict-lectures.log et quiescence-lectures.json. Le rouge d’attestation initial reste conservé et attribué à l’outil ; compilation, helpers, attestation corrigée et ciblé ont chacun leur résultat distinct.

Diff et preuves transmis à orchestration pour revue avant toute globale/master. ED/M8 gardent priorité à l’intégration ; aucun Rust poussé sur master pendant leurs globales. Ticket ouvert, worktrees et target97k privés conservés avec remotes et journaux, sans nettoyage d’un autre lot.

## Passe d’intégration après ED/M8 : arrêt de confinement navigateur

Dépendance levée explicitement par C7 : masterb726d907b4c808c501263b61da3424ed3d893bee. Rebase sous nouveau nom `cx-recherche-lectures-journaux-integration`, source propre publiée `487c68af39575582784cc26fc67dddad9bbc4c3a`. Conflit add/add limité au ticket déjà apporté par ED : texte commun identique et ajout recherche conservés. Range-diff archivé : Rust identique au correctif validéaa58, deux commits documentaires inchangés.

Nouvelle passe autorisée sous nouveau global2400s, départ après vrai lourd16:47:44 CET, échéance17:27:44. Même target97k privé unique/plafond32Go, lib29 complète, -j8/Max32, TMP court privé ~/.cache/cx-rj et React existant. Source/argv/env/cgroups enfants enregistrés ; tous services retirés avant les lectures. **Features5/5 vertes** ; contrôle `cargo check --lib --no-default-features --features rag3db-native` rc0 en11s, correctement identifié comme contrôle natif redondant. La précision C7 demandait le vrai minimal SANS --features ; le pilote étant déjà chargé, celui-ci restait à jouer séparément sous même échéance, sans interrompre les compilations. **Liblarge2201 réussis,0 échec,65 ignorés,0 filtré**,36,61s de tests.

Démarrage de l’étape navigateur séparée :40 tests détectés, puis **premier rouge du garde de confinement**, PID2997235 déplacé dans une scope Chromium extérieure au Max32 du poste. Arrêt de la session2994167 et des descendants possédés ; aucun résultat fonctionnel NAV40 complet revendiqué. Détail et suite dans [ticket commun des scopes](../../orchestration/tickets/2026-11-12-une-scope-e2e-imbriquee-peut-echapper-au-plafond-du-lanceur.md). Quiescence après arrêt active=[]/chromes=[] ; wrapper et PID fautif absents. **Lectures27 et vrai contrôle minimal non joués sur cette source**, aucun appel7879 ni relance après le rouge. Le vert fonctionnel antérieuraa58 reste historique, sans être présenté comme une passe intégration complète.

Preuves brutes aux deux postes dans `journaux-cx-recherche-lectures-12nov/integration-487c68/` : source.json, argv.json, features.log et features-detail/, minchecklib.log (natif redondant), lib.log, navigateur.log, resume.log, verdict-final.log ROUGE, cgroups-enfants.json, arret-session-2994167.json, quiescence-apres-arret.json et lancement.log. Aucun code runtime modifié, aucun master avant nouvelle revue/àtoi. Worktrees/target privés conservés, aucun retry aveugle ni budget prolongé.

## Recette préflight native unique publiée avant exécution

Autorisation C7 : une seule préflight30s/Max32 dans le global2400 initial (échéance17:27:44), aucun autre essai si échappement. Option existante `RAG3WEAVER_NAVIGATEUR` lue par trouver_navigateur ; candidat direct `/usr/lib/chromium/chromium`, pas une cause affirmée. Paquet installé chromium156.0.8078.79-1.1 ; ELF natif SHA25639533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a, lanceur /usr/bin SHA256df68ac926f09a1b892b68cd718aea4698a7a4963c172ede0b851e47f1da9871e.

Script externe ci-dessous, publié avant toute exécution, avec profil neuf sous integration-487c68/preflight-native-unique ; arguments identiques au témoin (y compris --no-sandbox déjà existant, aucune option de sécurité ajoutée/retirée). Version réelle par Browser.getVersion sur le même protocole CDP/fd3-fd4 que le témoin ; inventaire PID/starttime/cgroups/max et contrôle d’ascendance au fil de la vie. Browser.close puis TERM/KILL uniquement PID privés attribués via starttime/pidfd si nécessaire. Profil supprimé seulement après quiescence ; aucune mutation système/KDE/DBus/application utilisateur. Accès direct candidat seulement : si ce natif échappe aussi, verdict rouge conservé sans nouvel essai ni reprise NAV. Aucune nouvelle source Rust ni compilation.

SHA256 du script : `bd1f270e6b819a280e9b5360b3bcfbbbdaa3885be19d0eba4172278d086e2eb8`.

```python
import os,signal,subprocess,time,json,hashlib,fcntl,shutil,datetime
from pathlib import Path
B=Path.home()/'.cache/rag3weaver-build';J=B/'journaux-cx-recherche-lectures-12nov/integration-487c68';P=J/'preflight-native-unique';N=Path('/usr/lib/chromium/chromium')
S=json.loads((J/'source.json').read_text());deadline=datetime.datetime.fromisoformat(S['start']).timestamp()+2400
assert deadline-time.time()>=30,'deadline globale : aucun nouveau runtime'
assert not P.exists(),'préflight unique déjà consommée'
P.mkdir();profil=P/'profil';profil.mkdir();debut=time.monotonic()
cg=Path('/proc/self/cgroup').read_text().split('0::',1)[1].strip();assert Path('/sys/fs/cgroup'+cg+'/memory.max').read_text().strip()=='34359738368'
assert hashlib.sha256(N.read_bytes()).hexdigest()=='39533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a'
argv=[str(N),'--headless','--remote-debugging-pipe','--no-first-run','--no-default-browser-check','--disable-gpu','--disable-extensions','--disable-background-networking','--disable-sync','--no-sandbox','--user-data-dir='+str(profil),'about:blank']
(P/'recette.json').write_text(json.dumps(dict(argv=argv,cgroup=cg,memory_max='34359738368',source=S['source'],deadline_globale=deadline,limite_s=30),indent=2))
owned={};observes=[];p=None;version=None;erreur=None
uid=os.getuid()
def vivant(pid,start):
 try:
  q=Path('/proc')/str(pid);st=(q/'stat').read_text().rsplit(') ',1)[1].split()
  return q.stat().st_uid==uid and st[19]==start and st[0]!='Z'
 except OSError:return False
def inventaire():
 table={}
 for q in Path('/proc').iterdir():
  if not q.name.isdigit():continue
  try:
   if q.stat().st_uid!=uid:continue
   st=(q/'stat').read_text().rsplit(') ',1)[1].split();table[int(q.name)]=(int(st[1]),st[19],st[0])
  except OSError:continue
 changed=True
 while changed:
  changed=False
  for pid,(ppid,start,state) in table.items():
   if pid not in owned and (pid==p.pid or ppid in owned):owned[pid]=start;changed=True
 for pid,start in list(owned.items()):
  if not vivant(pid,start):continue
  q=Path('/proc')/str(pid)
  try:
   childcg=(q/'cgroup').read_text().split('0::',1)[1].strip();m=Path('/sys/fs/cgroup'+childcg+'/memory.max').read_text().strip()
   row=dict(pid=pid,starttime=start,cgroup=childcg,memory_max=m,exe=os.readlink(q/'exe'))
   if row not in observes:observes.append(row)
   if childcg!=cg and not childcg.startswith(cg+'/'):raise RuntimeError('enfant hors cgroup : '+json.dumps(row))
  except FileNotFoundError:pass
 (P/'pid-cgroups.json').write_text(json.dumps(observes,indent=2))
def stop():
 for sig in (signal.SIGTERM,signal.SIGKILL):
  for pid,start in list(owned.items()):
   if not vivant(pid,start):continue
   try:
    fd=os.pidfd_open(pid)
    if vivant(pid,start):signal.pidfd_send_signal(fd,sig)
    os.close(fd)
   except ProcessLookupError:pass
  time.sleep(.5)
def echeance(sig,frame):raise TimeoutError('préflight : échéance/signal')
signal.signal(signal.SIGTERM,echeance)
# Comme le témoin : pipe de commande vers fd3, réponses sur fd4 ; aucune option ajoutée.
a,b=os.pipe();c,d=os.pipe();aa=fcntl.fcntl(a,fcntl.F_DUPFD_CLOEXEC,10);dd=fcntl.fcntl(d,fcntl.F_DUPFD_CLOEXEC,10)
def fds():os.dup2(aa,3);os.dup2(dd,4)
try:
 with (P/'stderr.log').open('w') as out:
  p=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=out,stderr=out,preexec_fn=fds,close_fds=False,start_new_session=True)
 owned[p.pid]=Path(f'/proc/{p.pid}/stat').read_text().rsplit(') ',1)[1].split()[19]
 os.set_blocking(c,False);os.write(b,json.dumps(dict(id=1,method='Browser.getVersion')).encode()+b'\0');buf=b'';version_at=None
 while time.monotonic()-debut<20:
  inventaire()
  try:buf+=os.read(c,65536)
  except BlockingIOError:pass
  while b'\0' in buf:
   raw,buf=buf.split(b'\0',1)
   if raw:
    msg=json.loads(raw)
    if msg.get('id')==1:version=msg.get('result');version_at=time.monotonic()
  if version_at is not None and time.monotonic()-version_at>=3:break
  assert p.poll() is None,'navigateur terminé avant mesure'
  time.sleep(.1)
 assert version and version.get('product'),'version CDP absente'
 os.write(b,json.dumps(dict(id=2,method='Browser.close')).encode()+b'\0')
 fin=time.monotonic()+3
 while p.poll() is None and time.monotonic()<fin:inventaire();time.sleep(.1)
except BaseException as e:erreur=repr(e)
finally:
 stop()
 if p:
  try:p.wait(timeout=2)
  except subprocess.TimeoutExpired:erreur=erreur or 'parent non terminé'
 restes=[pid for pid,start in owned.items() if vivant(pid,start)]
 if not restes:shutil.rmtree(profil)
 (P/'pid-cgroups.json').write_text(json.dumps(observes,indent=2))
 (P/'verdict.json').write_text(json.dumps(dict(verdict='ROUGE' if erreur or restes else 'VERT',erreur=erreur,version=version,observed_pids=len(owned),restes=restes,profil_supprime=not profil.exists(),elapsed_s=time.monotonic()-debut),indent=2))
 print((P/'verdict.json').read_text())
 if erreur or restes:raise SystemExit(1)
```

## Résultat de l’unique préflight native, sans reprise des tests

Recette publiée avant exécution dans5e4d91596 puis4ff5d71b9 (ajout du traitement de SIGTERM pour le nettoyage). Une seule préflight sous poste lourd réel/Max32, timeout30s, deadlineglobale17:27:44 inchangée. Binaire direct `/usr/lib/chromium/chromium` identifié statiquement par paquet/version/hash ci-dessus, mêmes arguments que le témoin et profil neuf ; aucun changement système/KDE/DBus, option de sécurité ou service.

**ROUGE en1,246s.** PID3028835, starttime84506851, observé successivement dans `run-p3028833-i82714792.scope` avec memory.max34359738368 puis dans `app-org.chromium.Chromium-3028835.scope` avec memory.max=`max`, exécutable /usr/lib/chromium/chromium inchangé. L’accès direct au natif ne suffit donc pas à préserver le confinement. Le parent garde le même PID/starttime : il ne s’agit pas d’une confusion avec un navigateur déjà ouvert.

Onze PID privés attribués par filiation ; le garde a arrêté l’inventaire à la première sortie, donc le relevé cgroup publié porte ce parent aux deux états, pas une mesure complète rétrospective de tous les enfants. Version CDP non obtenue avant arrêt (null) : seule la version de paquet est revendiquée, aucun succès de protocole inventé. Tous les PID privés attribués sont arrêtés via pidfd/starttime, restes=[], profil supprimé après vérification. Preuves deux postes : integration-487c68/preflight-native-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log} et lancement-preflight-native.log.

Conformément à l’autorisation, aucun autre essai ni reprise de NAV40/lectures27/vrai checklib minimal. Les verts features5 et liblarge2201/0/65 restent distincts ; aucun verdict composé complet n’est affirmé. Blocage soumis à orchestration : il faut une recette de confinement du vrai navigateur avant de compléter la validation ; le lanceur /usr/bin n’est pas une explication suffisante. Ticket commun des scopes et ticket journaux restent ouverts, ressources de build et preuves conservées, aucun master.

## Seconde préflight explicitement autorisée : bus éphémère privé sans activation

C7 fournit le diagnostic sourcé e1b892d9c, ticket commun relu : Chromium156 components/dbus/xdg/systemd.cc et son contrat peuvent créer une scope app via le bus de session lors de l’initialisation du portail. Ce mécanisme est cohérent avec l’observation, sans trace locale de l’appel D-Bus. Nouvelle autorisation distincte : une seule préflight30s dans le budget2400 initial inchangé (17:27:44), dbus-run-session1.16.2 existant, aucun service permanent ni changement du bus utilisateur/KDE/système. SourceRust487 inchangée et tous rouges précédents gardés.

Configuration privée XML ci-dessous : aucun include, standard_session_servicedirs, servicedir ou répertoire d’activation. Socket dans un dossier privé neuf mode0700 ; authentification EXTERNAL et seul UID1000. Le bus ne peut activer aucun service déclaré, aucune fausse identité Flatpak/Snap. L’argv ajoute seulement le helper `dbus-run-session --config-file=<config privée> --` devant le même Chrome natif et les mêmes arguments/profil neuf ; rien ne change aux options de sandbox. La version effective doit être obtenue par CDP Browser.getVersion. Cgroups/PID/starttime/exe/max observés pour le helper éphémère et Chrome, vérification de tous les PID attribués, arrêt et nettoyage des seuls privés, pas de reprise si écart.

SHA256 dbus-run-session d310854dcd72b103aa95a7f6a54eae2cb1579d6c71af40243f840e37fa64d4c5 ; dbus-daemon b5d6c0469c7dc4050733f2a90fe64e9f55a3377521dbb295f03ee2cd9807acc4. SHA256 configuration `46950973c22dc27a8bf9be82bd489a10b760e89db110025c8a0f2b72d35f10a6` ; script `9536855300b18f41a90c92f85a03554e0153f9575d2cfb8e1028c9c3b7ae29a3`. Contrôle statique du script : compile Python sans exécution ; contrôles XML et hashes avant spawn.

```xml
<busconfig>
  <type>session</type>
  <listen>unix:dir=/home/lucied/.cache/cx-rj-bus</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow user="1000"/>
    <allow own="*"/>
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
  </policy>
</busconfig>
```

```python
import os,signal,subprocess,time,json,hashlib,fcntl,shutil,datetime
from pathlib import Path
B=Path.home()/'.cache/rag3weaver-build';J=B/'journaux-cx-recherche-lectures-12nov/integration-487c68';P=J/'preflight-dbus-unique';N=Path('/usr/lib/chromium/chromium')
S=json.loads((J/'source.json').read_text());deadline=datetime.datetime.fromisoformat(S['start']).timestamp()+2400
assert deadline-time.time()>=30,'deadline globale : aucun nouveau runtime'
assert not P.exists(),'préflight unique déjà consommée'
P.mkdir();profil=P/'profil';profil.mkdir();debut=time.monotonic()
config=J/'preflight-dbus-config.xml';socketdir=Path.home()/'.cache/cx-rj-bus'
assert not socketdir.exists();socketdir.mkdir(mode=0o700)
text=config.read_text();assert '<servicedir' not in text and '<include' not in text and '<standard_' not in text
for path,h in [('/usr/bin/dbus-run-session','d310854dcd72b103aa95a7f6a54eae2cb1579d6c71af40243f840e37fa64d4c5'),('/usr/bin/dbus-daemon','b5d6c0469c7dc4050733f2a90fe64e9f55a3377521dbb295f03ee2cd9807acc4')]:assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==h

cg=Path('/proc/self/cgroup').read_text().split('0::',1)[1].strip();assert Path('/sys/fs/cgroup'+cg+'/memory.max').read_text().strip()=='34359738368'
assert hashlib.sha256(N.read_bytes()).hexdigest()=='39533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a'
argv=['/usr/bin/dbus-run-session','--config-file='+str(config),'--',str(N),'--headless','--remote-debugging-pipe','--no-first-run','--no-default-browser-check','--disable-gpu','--disable-extensions','--disable-background-networking','--disable-sync','--no-sandbox','--user-data-dir='+str(profil),'about:blank']
(P/'recette.json').write_text(json.dumps(dict(argv=argv,cgroup=cg,memory_max='34359738368',source=S['source'],deadline_globale=deadline,limite_s=30,config_sha256=hashlib.sha256(config.read_bytes()).hexdigest()),indent=2))
owned={};observes=[];p=None;version=None;erreur=None
uid=os.getuid()
def vivant(pid,start):
 try:
  q=Path('/proc')/str(pid);st=(q/'stat').read_text().rsplit(') ',1)[1].split()
  return q.stat().st_uid==uid and st[19]==start and st[0]!='Z'
 except OSError:return False
def inventaire():
 table={}
 for q in Path('/proc').iterdir():
  if not q.name.isdigit():continue
  try:
   if q.stat().st_uid!=uid:continue
   st=(q/'stat').read_text().rsplit(') ',1)[1].split();table[int(q.name)]=(int(st[1]),st[19],st[0])
  except OSError:continue
 changed=True
 while changed:
  changed=False
  for pid,(ppid,start,state) in table.items():
   if pid not in owned and (pid==p.pid or ppid in owned):owned[pid]=start;changed=True
 ecarts=[]
 for pid,start in list(owned.items()):
  if not vivant(pid,start):continue
  q=Path('/proc')/str(pid)
  try:
   childcg=(q/'cgroup').read_text().split('0::',1)[1].strip();m=Path('/sys/fs/cgroup'+childcg+'/memory.max').read_text().strip()
   row=dict(pid=pid,starttime=start,cgroup=childcg,memory_max=m,exe=os.readlink(q/'exe'))
   if row not in observes:observes.append(row)
   if childcg!=cg and not childcg.startswith(cg+'/'):ecarts.append(row)
  except FileNotFoundError:pass
 (P/'pid-cgroups.json').write_text(json.dumps(observes,indent=2))
 if ecarts:raise RuntimeError('enfants hors cgroup : '+json.dumps(ecarts))
def stop():
 for sig in (signal.SIGTERM,signal.SIGKILL):
  for pid,start in list(owned.items()):
   if not vivant(pid,start):continue
   try:
    fd=os.pidfd_open(pid)
    if vivant(pid,start):signal.pidfd_send_signal(fd,sig)
    os.close(fd)
   except ProcessLookupError:pass
  time.sleep(.5)
def echeance(sig,frame):raise TimeoutError('préflight : échéance/signal')
signal.signal(signal.SIGTERM,echeance)
# Comme le témoin : pipe de commande vers fd3, réponses sur fd4 ; aucune option ajoutée.
a,b=os.pipe();c,d=os.pipe();aa=fcntl.fcntl(a,fcntl.F_DUPFD_CLOEXEC,10);dd=fcntl.fcntl(d,fcntl.F_DUPFD_CLOEXEC,10)
def fds():os.dup2(aa,3);os.dup2(dd,4)
try:
 with (P/'stderr.log').open('w') as out:
  p=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=out,stderr=out,preexec_fn=fds,close_fds=False,start_new_session=True)
 owned[p.pid]=Path(f'/proc/{p.pid}/stat').read_text().rsplit(') ',1)[1].split()[19]
 os.set_blocking(c,False);os.write(b,json.dumps(dict(id=1,method='Browser.getVersion')).encode()+b'\0');buf=b'';version_at=None
 while time.monotonic()-debut<20:
  inventaire()
  try:buf+=os.read(c,65536)
  except BlockingIOError:pass
  while b'\0' in buf:
   raw,buf=buf.split(b'\0',1)
   if raw:
    msg=json.loads(raw)
    if msg.get('id')==1:version=msg.get('result');version_at=time.monotonic()
  if version_at is not None and time.monotonic()-version_at>=3:break
  assert p.poll() is None,'navigateur terminé avant mesure'
  time.sleep(.1)
 assert version and '156.0.8078.79' in version.get('product',''),'version CDP réelle absente'
 assert any('/dbus-daemon' in r['exe'] for r in observes),'helper bus non observé'
 os.write(b,json.dumps(dict(id=2,method='Browser.close')).encode()+b'\0')
 fin=time.monotonic()+3
 while p.poll() is None and time.monotonic()<fin:inventaire();time.sleep(.1)
except BaseException as e:erreur=repr(e)
finally:
 stop()
 if p:
  try:p.wait(timeout=2)
  except subprocess.TimeoutExpired:erreur=erreur or 'parent non terminé'
 restes=[pid for pid,start in owned.items() if vivant(pid,start)]
 if not restes:
  shutil.rmtree(profil)
  shutil.rmtree(socketdir)
 (P/'pid-cgroups.json').write_text(json.dumps(observes,indent=2))
 (P/'verdict.json').write_text(json.dumps(dict(verdict='ROUGE' if erreur or restes else 'VERT',erreur=erreur,version=version,observed_pids=len(owned),restes=restes,profil_supprime=not profil.exists(),bus_prive_supprime=not socketdir.exists(),elapsed_s=time.monotonic()-debut),indent=2))
 print((P/'verdict.json').read_text())
 if erreur or restes:raise SystemExit(1)
```

## Préflight bus privé verte puis verdict composé d’intégration

Recette explicitement autorisée publiée268460373 avant son unique lancement ; sourceRust487c68af39575582784cc26fc67dddad9bbc4c3a inchangée. Préflight bus privé **VERTE en4,533s** : CDP réel protocol1.3, productChrome/156.0.8078.79, revisiona21ce95c24ce3964416f09ae1862f53f319eb4d0. Treize PID attribués, dont dbus-daemon et le navigateur natif, observés dans la portée Max32 ; restes=[], profil et socket privés supprimés. Preuves preflight-dbus-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log}. Aucun bus utilisateur, KDE ou système modifié, aucun service permanent activé.

La conformité de cette préflight permet la seule reprise explicitement autorisée des phases manquantes, sous le même budget initial2400 et deadline17:27:44 ; aucune relance des features5 ou de la liblarge déjà vertes. Vrai minimal : **cargo check --locked --lib --no-default-features rc0**, sans feature native et sans cargo test. NAV40 séparée, même ELF et mêmes arguments/profils, sélection native via RAG3WEAVER_NAVIGATEUR sous dbus-run-session/config privée sans activation : **40/0/0 en131,52s**. Puis /sante7879 réelle BGE-M3/sparse/facticefalse, lectures sous racine privée avec espace sans segment commandes : **27/0/0 en9,65s et2filets verts**. Santé après OK, aucun autre usage7879 recherche.

Les532 PID observés durant les phases finales, dont bus éphémère et vrais processus Chrome/e2e, restent dans un seul cgroup `memory.max=34359738368` ; HIGH0 et marker sous poste réel conservés. Source/manifestes propres et identiques. Trois ELF attestés source/native29 complète/provenance du bon worktree/**220 gabarits présents chacun** : lib SHA2566782a8e2c5a5be3f2a297530c97be9bfc10eb3437a0c4bb2eefdf1b995625691 ; NAV2567af2a82fc73ae3a6b3b0bd039af2b4eeb46d6697eeabf867474e48021ac2c ; e2e4a4266874ae7ed026e18d55481a4ad83af79c4e42c7a2e92120e524d718e9621. Chrome natif et configuration D-Bus ont toujours les hashes publiés. Scan /proc exeTarget/cwdWT/profilprivé=[] et socketbus absent : quiescence-finale.json.

**Verdict composé VERT**, sans transformer les anciennes tentatives en une passe sans incident : features5/5 et lib2201/0/65/0filtré proviennent du premier passage487 ; vrai minimal, NAV40, lectures27 et attestations des phases finales ci-dessus. Les rouges du garde et de la préflight native restent conservés, diagnostiqués séparément. Le checklib natif redondant est étiqueté comme tel ; il ne remplace pas le minimal. Le global2400 initial est respecté, aucune réinitialisation de budget ni prolongation, fin avant17:27:44. Aucun test/runtime/feature produit modifié pour le confinement ; aucun e2e entier, service/GPU/modèle nouveau ou rerunCI.

Preuves brutes deux postes : integration-487c68/{verdict-compose.json,verdict-compose.log,preflight-dbus-config.xml} et finalisation-dbus/{source.json,argv.json,cgroups-enfants.json,vrai-minchecklib.log,navigateur.log,lectures.log,e2e-complet.log,sante-7879-avant/apres.json,attestations.json,quiescence-finale.json,verdict-final.log,provenances.d}. Ticket commun des scopes reste ouvert pour la procédure transversale. Correction de fixture et preuves soumises à revue ; ticket journaux ouvert et aucun master avant àtoi. Ressources propres conservées, aucun lourd ou utilisateur7879 recherche restant.

## Clôture après revue du verdict composé

Revue explicite des logs bruts, 532 cgroups Max32, sonde/quiescence, trois ELF rehashés et identité native29 acquise. Diff487→c945 exclusivement Markdown ; clôture et déplacement dans ce même commit. Ticket commun scopes toujours ouvert ; aucun runtime ou rerunCI pour cette intégration. Push sous poste push/dernierfetch/rebase documentaire seulement/gardeVERT et trois contrôles aux deux postes, puis archives fidèles/gardes fraîches avant retrait des seules ressources privées.
