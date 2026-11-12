# Une scope e2e imbriquée peut échapper au plafond du lanceur

- **État** : ouvert le 12 novembre 2026 ; porteur recherche pour la primitive scripts/procédure, relecture ED ; faits ED et mémoire.
- **Pour** : recherche pour la recette commune des lanceurs ; orchestration coordonne les créneaux et ED relit son adoption. Aucun changement du runner produit dans le lot des accords.
- **Famille** : produit
- **Gravité** : risque de dépassement du budget mémoire et d'arrêt incomplet des enfants.
- **Atteignable en service** : non, lanceurs de validation.
- **Touche rag3weaver** : oui, montage de run_e2e et attestations de cgroup.

## Faits et limite de la preuve ancienne

Dans `extension/rag3weaver/run_e2e.sh`, l'absence de `RAG3WEAVER_VERROU_TENU` relance la suite
sous poste lourd (64–102). `confined` (181–190) crée aussi par défaut une scope systemd
`MemoryHigh=16G`. Une scope ainsi créée ne constitue pas une preuve d'appartenance à l'unité
privée de l'appelant : elle peut être sœur, hors de son MemoryMax et de son KillMode.
MemoryHigh est une limite souple, pas MemoryMax.

ED a lancé sa globale b42 sous unité privée Max32G et KillMode=control-group, vérifiés sur
le parent. Les deux suites e2e sont fonctionnellement vertes, mais leur PID/cgroup réel n'avait
pas été attesté. Le ciblé lectures24 avait une unité enveloppante au pic9,4M, alors que le
runner lançait sa propre scope. Ces mesures du parent ne prouvent pas le plafond des enfants ;
on ne réattribue pas rétrospectivement un cgroup non observé. Mémoire a identifié le même piège
et interrompu préventivement une tentative avant e2e ; ses preuves restent dans son ticket M8.

Preuves ED : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/`
(`passe.sh`, `cgroup-passe`, `memorymax-passe`, journaux et verdict global b42-r2).
Source produit figée : `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`.

## Montage correct à composer

Après acquisition réelle de poste lourd, dans une unité privée effectivement Max32G avec
KillMode=control-group et budget après acquisition :

- poser `RAG3WEAVER_VERROU_TENU` pour ne pas reprendre un second verrou ; jamais avant un vrai verrou ;
- poser `RAG3WEAVER_BUILD_MEMORY_HIGH=0` pour éviter la scope intérieure quand le plafond extérieur
  est déjà garanti ; cela ne retire pas le MemoryMax extérieur ;
- relever le **PID du vrai binaire e2e**, son `/proc/PID/cgroup`, le memory.max de ce cgroup,
  son identité de suite et le KillMode de l'unité ; échouer si aucun vrai enfant n'est observé ;
- préserver les anciens passages, puis refaire seulement les étapes dont le confinement restait
  non prouvé, avec verdict composé explicite et source inchangée.

ED prépare cette vérification bornée pour lectures24 et démon13, autorisée par orchestration,
sans rejouer features/lib/nav déjà validés. Aucune désactivation de bac à sable, hausse du plafond,
nouveau service ou test de charge n'est nécessaire.

## Pour fermer

Consigner les preuves enfants ED/M8, prévenir les autres drivers et intégrer la recette dans
la procédure commune de passe. Vérifier que budget, arrêt et comptage des lourds couvrent les
vrais descendants. Toute évolution du runner lui-même est un lot séparé, relisible et testé ;
ce ticket n'en annonce aucune comme livrée.

## Preuve enfant ED acquise

Sur b42 inchangé, après autorisation et créneau7879 : lectures24/0/0 et démon13/0/0,
deux suites sur deux sans saut, unité600s après acquisition. PID2697195 et2702388
réellement observés dans cx-ed-accord-e2e-confines-20261112.service, avec
memory.max34359738368, RuntimeMaxUSec10min et KillModecontrol-group.
`I-luciepc/accord-20261112/{e2e_lectures,e2e_demon_hote}-enfant.json`,
`verdict-compose.json` et `quiescence-e2e-confines.json` ([]). Le verdict composé garde
features/lib/nav précédents et remplace seulement la preuve de confinement des e2e.
M8, prévention des autres drivers et procédure commune restent à orchestration : ticket ouvert.

## Extension du diagnostic : Chromium et le bus de session

Recherche487 a arrêté NAV40 après déplacement d'un PID Chromium dans une scope
app-org.chromium sœur. Le préflight natif distinct a observé le même PID/exe
d'abord sous Max32 puis dans cette scope avec memory.max=max ; arrêt des seuls
processus privés, aucun reste. Ni NAV40 complète ni lectures27 de cette source
ne sont validées par ces essais. Les anciens contrôles des PID e2e ne prouvent
pas rétroactivement le confinement de tous leurs descendants Chromium.

Source primaire relue à la version du paquet :
[Chromium156 systemd.cc](https://github.com/chromium/chromium/blob/156.0.8078.79/components/dbus/xdg/systemd.cc)
et [son contrat](https://github.com/chromium/chromium/blob/156.0.8078.79/components/dbus/xdg/systemd.h).
L'initialisation du portail peut demander au gestionnaire systemd du bus de
session une scope app avec le PID courant. Le chemin observé est cohérent avec
ce mécanisme ; la lecture ne remplace pas une trace locale de l'appel D-Bus.
Changer seulement le lanceur /usr/bin en binaire /usr/lib n'isole pas ce bus.

Recette candidate à préparer avant un nouveau préflight borné : bus de session
strictement privé au job via dbus-run-session, configuration explicite sans
répertoires d'activation de services, puis Chrome réel avec les mêmes arguments
et un profil neuf. Les outils et leur manuel1.16.2 existent déjà sur luciepc.
Le helper IPC doit rester dans le même Max32 et terminer avec le programme ;
aucune modification du bus utilisateur, de KDE, du système ou du bac à sable.
Ne pas simuler un environnement Flatpak/Snap pour éviter une branche de code.

Critères du seul préflight candidat : configuration/source/version/hash publiés,
30s et budget global initial inchangés, CDP réel disponible, tous les processus
privés observés dans le groupe borné, aucun détenteur restant après arrêt.
Au premier écart, arrêter les seuls processus attribués et soumettre le résultat.
Une conformité prouvée permettrait ensuite les seules phases manquantes, sans
rejouer compilation/lib/features déjà valides ni annoncer un navigateur simulé.
La recette n'est pas déclarée validée à l'écriture de cette note.

## Extension observée : navigateur sortant du scope de validation

Recherche, intégration des fixtures journaux, source figée `487c68af39575582784cc26fc67dddad9bbc4c3a` après rebase surmasterb726d907b. Vrai poste lourd/MemoryMax32G, BUILD_MEMORY_HIGH=0, contrôle des descendants activé. Features5/5 et liblarge2201/0/65 verts ; au démarrage séparé des40 témoins navigateur, le garde observe PID2997235 dans `/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.chromium.Chromium-2997235.scope`, extérieur à la portée du poste. Le pilote arrête immédiatement la session2994167 et ses descendants possédés ; aucune poursuite vers lectures ni retry.

Il ne s’agit pas de la scope créée par `run_e2e.sh` : l’étape en cours est `scripts/temoins_navigateur.sh`. Le code de test lance le navigateur par Command::new, le candidat disponible est /usr/bin/chromium, petit lanceur ELF vers /usr/lib/chromium/chromium. Le composant exact qui demande le déplacement et le plafond effectif de la scope sœur n’ont pas été mesurés avant l’arrêt : aucune attribution rétrospective de limite ni cause interne affirmée. Une recette qui ne vérifie que le cgroup du binaire de tests ne prouve donc pas celui de Chromium.

Preuves deux postes : `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/integration-487c68/` (source.json, argv.json, cgroups-enfants.json, navigateur.log, lancement.log, verdict-final.log, arret-session-2994167.json). Le PID2997235 et le wrapper2947827 sont ensuite absents ; quiescence-apres-arret.json porte active=[] et chromes=[] par exe/cwd/profil privé. Aucun service7879 utilisé, aucun navigateur utilisateur arrêté par nom.

Suite à orchestration/propriétaires des drivers : établir une recette navigateur restant sous le plafond et l’arrêt de la passe, l’attester sur vrais PID enfants avant de reprendre l’étape. Aucun changement produit ni désactivation de limite livré par ce constat ; pas de relance inchangée. Le ticket des fixtures reste distinct et ouvert.

## Résultat de l’unique préflight native, sans reprise des tests

Recette publiée avant exécution dans5e4d91596 puis4ff5d71b9 (ajout du traitement de SIGTERM pour le nettoyage). Une seule préflight sous poste lourd réel/Max32, timeout30s, deadlineglobale17:27:44 inchangée. Binaire direct `/usr/lib/chromium/chromium` identifié statiquement par paquet/version/hash ci-dessus, mêmes arguments que le témoin et profil neuf ; aucun changement système/KDE/DBus, option de sécurité ou service.

**ROUGE en1,246s.** PID3028835, starttime84506851, observé successivement dans `run-p3028833-i82714792.scope` avec memory.max34359738368 puis dans `app-org.chromium.Chromium-3028835.scope` avec memory.max=`max`, exécutable /usr/lib/chromium/chromium inchangé. L’accès direct au natif ne suffit donc pas à préserver le confinement. Le parent garde le même PID/starttime : il ne s’agit pas d’une confusion avec un navigateur déjà ouvert.

Onze PID privés attribués par filiation ; le garde a arrêté l’inventaire à la première sortie, donc le relevé cgroup publié porte ce parent aux deux états, pas une mesure complète rétrospective de tous les enfants. Version CDP non obtenue avant arrêt (null) : seule la version de paquet est revendiquée, aucun succès de protocole inventé. Tous les PID privés attribués sont arrêtés via pidfd/starttime, restes=[], profil supprimé après vérification. Preuves deux postes : integration-487c68/preflight-native-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log} et lancement-preflight-native.log.

Conformément à l’autorisation, aucun autre essai ni reprise de NAV40/lectures27/vrai checklib minimal. Les verts features5 et liblarge2201/0/65 restent distincts ; aucun verdict composé complet n’est affirmé. Blocage soumis à orchestration : il faut une recette de confinement du vrai navigateur avant de compléter la validation ; le lanceur /usr/bin n’est pas une explication suffisante. Ticket commun des scopes et ticket journaux restent ouverts, ressources de build et preuves conservées, aucun master.

## Préflight bus privé verte puis verdict composé d’intégration

Recette explicitement autorisée publiée268460373 avant son unique lancement ; sourceRust487c68af39575582784cc26fc67dddad9bbc4c3a inchangée. Préflight bus privé **VERTE en4,533s** : CDP réel protocol1.3, productChrome/156.0.8078.79, revisiona21ce95c24ce3964416f09ae1862f53f319eb4d0. Treize PID attribués, dont dbus-daemon et le navigateur natif, observés dans la portée Max32 ; restes=[], profil et socket privés supprimés. Preuves preflight-dbus-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log}. Aucun bus utilisateur, KDE ou système modifié, aucun service permanent activé.

La conformité de cette préflight permet la seule reprise explicitement autorisée des phases manquantes, sous le même budget initial2400 et deadline17:27:44 ; aucune relance des features5 ou de la liblarge déjà vertes. Vrai minimal : **cargo check --locked --lib --no-default-features rc0**, sans feature native et sans cargo test. NAV40 séparée, même ELF et mêmes arguments/profils, sélection native via RAG3WEAVER_NAVIGATEUR sous dbus-run-session/config privée sans activation : **40/0/0 en131,52s**. Puis /sante7879 réelle BGE-M3/sparse/facticefalse, lectures sous racine privée avec espace sans segment commandes : **27/0/0 en9,65s et2filets verts**. Santé après OK, aucun autre usage7879 recherche.

Les532 PID observés durant les phases finales, dont bus éphémère et vrais processus Chrome/e2e, restent dans un seul cgroup `memory.max=34359738368` ; HIGH0 et marker sous poste réel conservés. Source/manifestes propres et identiques. Trois ELF attestés source/native29 complète/provenance du bon worktree/**220 gabarits présents chacun** : lib SHA2566782a8e2c5a5be3f2a297530c97be9bfc10eb3437a0c4bb2eefdf1b995625691 ; NAV2567af2a82fc73ae3a6b3b0bd039af2b4eeb46d6697eeabf867474e48021ac2c ; e2e4a4266874ae7ed026e18d55481a4ad83af79c4e42c7a2e92120e524d718e9621. Chrome natif et configuration D-Bus ont toujours les hashes publiés. Scan /proc exeTarget/cwdWT/profilprivé=[] et socketbus absent : quiescence-finale.json.

**Verdict composé VERT**, sans transformer les anciennes tentatives en une passe sans incident : features5/5 et lib2201/0/65/0filtré proviennent du premier passage487 ; vrai minimal, NAV40, lectures27 et attestations des phases finales ci-dessus. Les rouges du garde et de la préflight native restent conservés, diagnostiqués séparément. Le checklib natif redondant est étiqueté comme tel ; il ne remplace pas le minimal. Le global2400 initial est respecté, aucune réinitialisation de budget ni prolongation, fin avant17:27:44. Aucun test/runtime/feature produit modifié pour le confinement ; aucun e2e entier, service/GPU/modèle nouveau ou rerunCI.

Preuves brutes deux postes : integration-487c68/{verdict-compose.json,verdict-compose.log,preflight-dbus-config.xml} et finalisation-dbus/{source.json,argv.json,cgroups-enfants.json,vrai-minchecklib.log,navigateur.log,lectures.log,e2e-complet.log,sante-7879-avant/apres.json,attestations.json,quiescence-finale.json,verdict-final.log,provenances.d}. Ticket commun des scopes reste ouvert pour la procédure transversale. Correction de fixture et preuves soumises à revue ; ticket journaux ouvert et aucun master avant àtoi. Ressources propres conservées, aucun lourd ou utilisateur7879 recherche restant.

## Reprise parallèle autorisée le 12 novembre : rendre la recette commune

Lucie demande de reprendre des chantiers utiles en parallèle des couches.
Recherche prend la suite d'outillage de ce ticket : intégrer la recette prouvée
dans une primitive de lanceur partagée et sa procédure, au lieu de recopier les
scripts privés à chaque passe. Le lot journaux et le lot BGE sont déjà intégrés
(0d7bf10eb et 6dd116e8f) ; aucun rejeu de leurs tests ou ancien plan FTS n'est demandé.

Composer les scripts existants : bus de session privé sans activation pour les
témoins navigateur sous Linux, acquisition réelle du poste avant marqueur de
verrou, absence de scope sœur échappant au plafond extérieur, arrêt et nettoyage
des seuls processus et chemins attribués au lot. Pas de modification du bus de
Lucie, de KDE, des services permanents, de Chromium ni de ses options de sandbox.
Conserver les limites Max32, les délais et les refus explicites ; une dépendance
manquante ne devient pas un succès ou un saut silencieux.

Proposer un diff borné des lanceurs et témoins d'outillage, publié avant essais,
avec adoption explicite par les chemins existants concernés. Première validation
CPU des contrats de montage et de nettoyage ; une préflight réelle bornée ne
sera lancée qu'après recette et coordination, sans reconstruire ou rejouer les
lots verts. ED relit la réutilisation ; Recherche garde l'édition des scripts.
Le ticket restera ouvert tant que la garantie transversale n'est pas réellement
adoptée et éprouvée. Les sources des passes Arbre figées restent inchangées ;
aucun push de ces scripts pendant sa passe sans coordination de l'intégration.

## Diff de primitive commune préparé sur43db9634a

Worktree/branche neuf cx-recherche-confinement-lanceurs, aucun ancien lot rouvert ni source Arbre modifiée. Propriété scripts recherche, ED relecture. Proposition : `scripts/confiner_validation.py`, primitive unique de portée de validation Linux, adoptée réellement aux entrées run_e2e.sh et temoins_navigateur.sh. La procédure donne aussi l’entrée composée pour posséder un budget/bus/plafond uniques pour toute une passe.

Contrats : vraie ascendance poste/fiche active avant marqueur ; plafond effectif hérité <=32Gio ; reçu privé valide seulement dans l’ascendance du superviseur vivant (PID/starttime/cgroup/token), sans bypass par variable ; HIGH0 et suppression de la création de scopes sœurs dans confined ; bus éphémère privé EXTERNAL/UID courant sans aucun répertoire d’activation ; suivi des descendants/subreaper et cgroups, arrêt pidfd des seuls attribués, code/rouge conservé sans retry, résidu rouge et nettoyage des seuls chemins privés après quiescence. Aucune option de test permettant de désactiver le confinement en production ; dépendances/limites manquantes refusées. Aucun Rust produit, réglage système/KDE, modèle/service permanent ou option de sandbox modifié.

Témoins CPU préparés : XML sans activation/échappement, plafond hérité et refus sans limite, marqueur seul/reçu périmé refusés, scope sœur rouge avec preuve, PID réutilisé jamais signalé, dépendance absente avant lancement, adoption des DEUX vraies entrées avec frontière factice et sans Cargo, commande normale/nettoyage, code rouge sans retry, deadline avec enfant et enfant détaché résiduel rouge/nettoyé. Les simulations n’écrivent ni dans les verrous réels ni dans les cgroups système, ne lancent aucun bus/service/navigateur ; seuls petits processus CPU privés.

Contrôles statiques préparatoires : parsing Python, bash -n des deux entrées et diff --check verts. Source et recette à publier avant exécution réelle ; CPU à jouer en premier, premier rouge inattendu stop/diagnostic. Unique essai réel <=30s à coordonner après NAV Arbre, preuve CDP vraie + helper/PID/cgroups/Max32/lifecycle via la primitive publiée, sans Cargo/lib/FTS/journaux à rejouer. Aucun push code master pendant A sans coordination ; ticket reste ouvert jusqu’à adoption prouvée et revue/intégration.

## Relecture ED/C7 avant préflight : P1/P2 corrigés et borne de nettoyage

Premier passage CPU e2bcdd6cd :11/11 verts en1,650s ; ajout897291e78 :2/2 verts (reçu vivant/expiré et conservation d’un plafond explicitement plus bas,8G au lieu d’imposer32G). Aucun bus/navigateur réel lancé, preuves `~/.cache/rag3weaver-build/journaux-cx-recherche-confinement-12nov/temoins-cpu-{e2bcdd,ajouts}.log`.

P1 ED confirmé par inspection : possedes[777]=ancien ne permet pas d’adopter888 si table777 est désormais un tiers. La filiation exige maintenant que le parent actuel ait encore le même starttime ; test adversarial stable vérifie888 jamais attribué ni signalé. P2 : un reçu cohérent/token choisi nommant un ancêtre poste ne prouve pas une supervision. La vérification exige maintenant aussi l’exécutable Python réel, l’entrée `lancer` et l’identité/hash du script superviseur en cours dans cette ascendance. Test avec ancêtre réellement vivant et poste/plafond simulés valides, fichier0600/token/starttime/cgroup/délai cohérents : refus s’il n’exécute pas le superviseur. Cette garantie porte sur l’absence de bypass par environnement/reçu seuls ; aucune isolation absolue entre programmes hostiles de même UID n’est revendiquée.

Retour Arbre/mémoire intégré : pidfd est ouvert AVANT la dernière relecture du starttime et le signal passe par ce FD ; nouveau test de réutilisation entre open et recheck sans signal. Un échec de nettoyage après commande rc0 capture maintenant l’erreur dans le verdict ROUGE et conserve le temporaire, au lieu de perdre le verdict dans une exception ; témoin négatif préparé. Sources à republier avec ces corrections et CPU à rejouer car les frontières de suivi/autorité/nettoyage changent. Premier rouge inattendu stop/diagnostic ; aucun essai réel avant finNAV Arbre et revue de cette source.

## Recette de l’unique préflight réelle de la primitive (non lancée)

Source scripts publiée62c64b152a8d2e9c0cae6a312f9ffb510ddf4854 après P1/P2. CPU complet17/17 verts en1,791s ; les ROUGE dans le journal sont ceux des cas négatifs attendus (code7, délai, résidu, nettoyage refusé), pas une passe réelle rouge. Le diff reste scripts/procédure, sans Cargo/Rust produit. ED relit ; finNAV Arbre et accord de créneau requis avant l’unique runtime, aucune source A modifiée.

Préflight proposée : utiliser LA primitive publiée, pas recopier sa logique de bus/supervision. Poste réellement acquis par elle, --delai26 (réserve l’arrêt dans le maximum30s autorisé), preuves dans `journaux-cx-recherche-confinement-12nov/reel-unique/`. Commande :

```bash
python3 "$W/extension/rag3weaver/scripts/confiner_validation.py" lancer --delai 26 --preuves "$J/reel-unique" -- python3 "$B/cx-confinement-sonde-cdp.py" "$W/extension/rag3weaver/scripts/confiner_validation.py" "$J/sonde-unique"
```

La sonde vérifie d’abord le reçu réel par l’entrée `verifier`, puis demande Browser.getVersion au Chromium natif156.0.8078.79 et Browser.close ; mêmes arguments et flags de sandbox que le témoin existant, profil neuf. Bus/config/limites/descendants/lifecycle sont entièrement possédés par confiner_validation.py. Après retour, vérifier verdict, vrais enfants (Python/DBus/Chrome), cgroups/effectiveMax32, rc et quiescence exe/cwd/profil, puis retirer le seul profil enregistré si aucun détenteur. Aucun service permanent, GPU/modèle ou Cargo/lib/anciens lots ; premier écart stop et soumission sans autre essai.

Chrome natif SHA25639533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a ; helpers D-Bus déjà présents/version1.16.2, hashes à recontrôler dans la recette générée. Sonde externe publiée ci-dessous SHA256 `56d282a45a5deb6e623e896cc1de83bd84a37a66829b297244d1021af9c697ec` ; aucun runtime joué à cette écriture.

```python
import fcntl,hashlib,json,os,shutil,signal,subprocess,sys,tempfile,time
from pathlib import Path
helper=Path(sys.argv[1]);journal=Path(sys.argv[2]);journal.mkdir(parents=True,exist_ok=True)
subprocess.run([sys.executable,str(helper),'verifier'],check=True,timeout=2)
natif=Path('/usr/lib/chromium/chromium')
assert hashlib.sha256(natif.read_bytes()).hexdigest()=='39533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a'
profil=Path(tempfile.mkdtemp(prefix='r3-cdp-',dir='/tmp'))
argv=[str(natif),'--headless','--remote-debugging-pipe','--no-first-run','--no-default-browser-check','--disable-gpu','--disable-extensions','--disable-background-networking','--disable-sync','--no-sandbox','--user-data-dir='+str(profil),'about:blank']
a,b=os.pipe();c,d=os.pipe();aa=fcntl.fcntl(a,fcntl.F_DUPFD_CLOEXEC,10);dd=fcntl.fcntl(d,fcntl.F_DUPFD_CLOEXEC,10)
def fds():os.dup2(aa,3);os.dup2(dd,4)
p=None;fd=None;version=None;erreur=None
try:
 with (journal/'chrome-stderr.log').open('w') as out:p=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=out,stderr=out,preexec_fn=fds,close_fds=False)
 fd=os.pidfd_open(p.pid)
 os.set_blocking(c,False);os.write(b,json.dumps(dict(id=1,method='Browser.getVersion')).encode()+b'\0');buf=b'';fin=time.monotonic()+10
 while time.monotonic()<fin and version is None:
  try:buf+=os.read(c,65536)
  except BlockingIOError:pass
  while b'\0' in buf:
   raw,buf=buf.split(b'\0',1)
   if raw:
    msg=json.loads(raw)
    if msg.get('id')==1:version=msg.get('result')
  assert p.poll() is None,'Chrome terminé avant CDP'
  time.sleep(.05)
 assert version and version.get('product')=='Chrome/156.0.8078.79','réponse CDP réelle attendue'
 time.sleep(.5)
 os.write(b,json.dumps(dict(id=2,method='Browser.close')).encode()+b'\0');p.wait(timeout=3)
 assert p.returncode==0,'sortie Chrome non verte'
except BaseException as e:
 erreur=repr(e)
 if fd is not None:
  try:signal.pidfd_send_signal(fd,signal.SIGTERM)
  except ProcessLookupError:pass
 raise
finally:
 if fd is not None:os.close(fd)
 (journal/'sonde.json').write_text(json.dumps(dict(argv=argv,pid=p.pid if p else None,version=version,profil=str(profil),erreur=erreur),indent=2))
 # Le profil sera retiré par le contrôle final après la quiescence de tous les descendants.
```

## Bord racine relevé par ED, avant tout essai réel

ED a identifié une seconde occurrence de P1 : une racine déjà Zombie peut être absente du premier inventaire ; après poll qui la récolte, son PID réutilisé ne doit jamais devenir propriété du groupe. Le starttime racine est désormais lu directement dans /proc immédiatement après Popen, AVANT tout poll/wait et même pour Zombie, puis figé dans Descendants ; la règle spéciale « pid == racine » disparaît. Test CPU ajouté : racine absente au premier inventaire puis PID777 tiers réutilisé, aucune adoption/observation/signal. La source de la recette réelle sera cette révision publiée, pas62c64 ; aucun runtime réel ni second essai consommé. Relecture et finNAV Arbre restent préalables.

## Rouge CPU ecd605037 conservé, correction d’assertion préparée sans rejeu

Après18/18 verts1,820s sur4732 puis19/19 verts2,551s sur4b0, ecd605037 ajoute le cas « refus du dernier relevé n’empêche pas l’arrêt ». Passage20témoins : **19 réussis,1 échec**. Le helper a bien rendu ROUGE, restes=[] et temporaire nettoyé ; ces assertions passent. La dernière assertion cherchait « scope simulée » en brut dans une erreur contenant json.dumps/ensure_ascii (« scope simul\u00e9e »). Le test doit juger le champ structuré `enfants[].refus`, pas la représentation JSON incluse dans le texte de l’erreur.

Arrêt/soumission immédiats, pas de rejeu ni essai réel. Log rouge `journaux-cx-recherche-confinement-12nov/temoins-cpu-stop.log` gardé ; PID2175542/start51229804 et2175544/start51229807 attribués absents, preuve quiescence-rouge-cpu.json. Correction préparée : décoder enfants.json puis exiger refus exact « scope simulée hors plafond », en conservant rcROUGE/restes=[]/nettoyage ; aucun contrat de signalement abaissé. unittest failfast activé pour arrêter la suite au premier autre échec inattendu. Rejeu CPU borné à attendre après publication/soumission de ce diagnostic, avant toute préflight réelle.

ED a relu ecd statiquement : refus final enregistré sans court-circuiter groupe.arreter, rouge conservé ; pas de nouvelle objection concrète. Limite correctement maintenue : le refus précoce de ce cas ne prouve pas à lui seul qu’un petit-fils est déjà créé ; les autres cas délai/enfant détaché couvrent ces vies. Garantie ENV seule qualifiée, aucune nonforgeabilité hostile mêmeUID absolue. Sources Arbre et services réels toujours intouchés.

## Reprise CPU autorisée après diagnostic du JSON

GO C7 pour correction TEST seulement et nouvelle CPU<=60s : lire enfants.json/refus exact attribué aux PID/start attendus, garder toutes les assertions d’arrêt/nettoyage/rouge, rouge20(19/1) et quiescence conservés. Le test relève maintenant le PID/start racine du vrai Popen CPU avant toute récolte puis les compare à la ligne de refus structurée. Le helper produit reste strictement ecd605037 ; aucune adaptation de son erreur à une phrase de test. Rejeu sur source publiée modifiée, pas de retry inchangé. Préflight réelle toujours interdite avant finNAV Arbre et revue du plan corrigé.

## Preuve CPU exacte et fenêtre réelle encore non consommée

Après GO C7 et publication du delta TEST seulement5fab0e1f9, passage CPU complet **20/20 verts en2,059s**, borné60s, journal `temoins-cpu-pid-attribue.log`. Le helper produit est strictement celui d’ecd605037 ; le test vérifie maintenant le refus structuré exact sur le PID/start du Popen attendu et garde toutes les assertions de rouge/arrêt/nettoyage. Rouge20(19/1) et quiescence-rouge-cpu.json=[] conservés à côté, aucun retry inchangé. La passe intermédiaire e3fe20/20 en2,135s est également gardée ; elle précède l’assertion renforcée PID/start.

NAV Arbre libérée :40/0/0 en137,88s,528 records Max32, puis garde rouge pour40 dbus-daemon privés résiduels (aucun Chrome). Arrêt attribué pidfd/starttime+scope et preuve indépendante `cx-passe-faits-lies-v5/nav-quiescence-apres-arret.json` : scope inactive, known_live=[], context exeTarget/cwdWT/profil=[], socketnodes=[],20:48:00 Paris. Ce fait reste un vert fonctionnel plus un rouge d’hygiène de la passe A ; aucun rejeu NAV ni modification de ses sources. Une adoption de UN bus pour toute commande par le helper évite de dépendre d’un wrapper bus par Chrome tué lors de Drop ; la nouvelle préflight doit encore éprouver sa propre vie réelle.

Worktree distant propre figé5fab0e1f9, sans Cargo/sous-module initialisé. Baseline privée des processus Chrome/DBus préexistants et recette argv/hashes écrites avant lancement : baseline-avant-proposee.json et recette-proposee.json, copies aux deux postes. Source/helper/probe/Chrome/DBus hashés ; aucun service ni navigateur lancé. Revue du plan par C7 encore attendue avant l’unique préflight26s<=30s via la primitive, profil/bus privés seulement.

SHA256 logCPU20/20 : `dce2e213501f1b4c23077950f0e1ff6867fcf41155c309e3761f5fd107f7d9e1`. Recette exacte :

```json
{
  "source": "5fab0e1f9b59bb1b844b334dda4ac3b3d1ef099e",
  "argv": [
    "python3",
    "/home/lucied/git_workspaces/cx-recherche-confinement-lanceurs/extension/rag3weaver/scripts/confiner_validation.py",
    "lancer",
    "--delai",
    "26",
    "--preuves",
    "/home/lucied/.cache/rag3weaver-build/journaux-cx-recherche-confinement-12nov/reel-unique",
    "--",
    "python3",
    "/home/lucied/.cache/rag3weaver-build/cx-confinement-sonde-cdp.py",
    "/home/lucied/git_workspaces/cx-recherche-confinement-lanceurs/extension/rag3weaver/scripts/confiner_validation.py",
    "/home/lucied/.cache/rag3weaver-build/journaux-cx-recherche-confinement-12nov/sonde-unique"
  ],
  "max_s": 30,
  "hashes": {
    "/home/lucied/git_workspaces/cx-recherche-confinement-lanceurs/extension/rag3weaver/scripts/confiner_validation.py": "4f57d0e589f54d1e5733bcd01d793ce12b10e64ddd05220d22a9719084641c94",
    "/home/lucied/.cache/rag3weaver-build/cx-confinement-sonde-cdp.py": "56d282a45a5deb6e623e896cc1de83bd84a37a66829b297244d1021af9c697ec",
    "/usr/lib/chromium/chromium": "39533024d5bd2582b0915ded18cffb3fae01326723562f4988b08947516d624a",
    "/usr/bin/dbus-run-session": "d310854dcd72b103aa95a7f6a54eae2cb1579d6c71af40243f840e37fa64d4c5",
    "/usr/bin/dbus-daemon": "b5d6c0469c7dc4050733f2a90fe64e9f55a3377521dbb295f03ee2cd9807acc4"
  }
}
```

## GO final de préflight et plafond extérieur, publiés avant entrée

GO C7 reçu après revue CPU20/20, recette c5f et source5fab, NAV Arbre indépendante quiescente. Le helper reste --delai26 ; le plafond extérieur est posé **après vraie acquisition** du poste32G : `poste lourd timeout --signal=TERM --kill-after=1s 29 python3 <contrôleur privé>`, soit plafond total30s arrêt compris, sans restart. Le contrôleur appelle exactement argv de recette-proposee.json (primitive publiée, même sondeCDP/hash et même Chrome/hash) ; il ne réimplémente ni le bus ni les signaux de la primitive. Il observe en plus les exe/PID/start/cgroups du cgroup privé, vérifie rc/verdict/nettoyage et CDP réel+close, puis vérifie quiescence et retire le seul profil enregistré. Les processus préexistants de la baseline ne sont jamais attribués.

SHA256 contrôleur `4a185e239e836245729fd3b3a1ec149a784f6f55eb04f749812f93192f78131c`. Commande extérieure (pas encore lancée à cette écriture) :

```bash
POSTE_MEM_MAX=32G "$W/extension/rag3weaver/scripts/poste" lourd timeout --signal=TERM --kill-after=1s 29 python3 "$B/cx-confinement-preflight-controle.py"
```

Le budget démarre après le poste ; aucune attente ne le prolonge. Premier écart STOP/diagnostic, aucun second essai réel ni ancien NAV/lib/FTS. Contrôleur exact conservé hors dépôt avec les preuves ; source scripts distante5fab propre figée, les commits ultérieurs de recette sont Markdown seuls.

## Primitive commune : unique préflight réelle verte le12novembre20:53:40

Source distante propre figée5fab0e1f9b59bb1b844b334dda4ac3b3d1ef099e, helper production ecd605037 ; commits suivants de recette Markdown seuls. Préflight unique autorisée après CPU20/20 et finNAV/quiescence Arbre, publiée avant entrée (c5f/recette-proposee.json puis4b1c pour plafond extérieur). Commande réelle sous vrai poste32G, timeout extérieur29s+killafter1s (total30 après acquisition), primitive --delai26 ; aucune réinitialisation ni second essai.

**VERTE en1,306s**, rccontrôleur0, rchelper0, verdictVERT, erreurNone, restes=[], bus privé nettoyé. La sonde a validé positivement l’entrée `verifier` du vrai superviseur puis Browser.getVersion réel Chrome/156.0.8078.79, protocol1.3, révisiona21ce95c24ce3964416f09ae1862f53f319eb4d0 et Browser.close/rcChrome0. Seize descendants relevés par la primitive, vingt observations indépendantes dont dbus-daemon PID4186120 et Chrome PID4186123 ; un seul cgroup `run-p4186099-i83843277.scope`, effectif34359738368 pour chaque descendant observé. Pas de wrapper bus par Chrome : un bus unique possédé pour toute la commande.

Quiescence indépendante : known_live=[], profil_holders=[], profil absent, bus/socket privé absent, baseline de9 processus préexistants non attribuée. Nettoyage limité au profil neuf nommé par la sonde et au temporaire du bus créé par la primitive ; aucun Chrome utilisateur, service permanent, modèle/GPU ou bus KDE/utilisateur touché. Aucun Cargo/lib/FTS/ancienNAV/journaux rejoué. Le rouge CPU20(19/1), son diagnostic JSON et quiescence[] sont conservés, ainsi que les preuves d’hygiène Arbre indépendantes.

Preuves deux postes : `~/.cache/rag3weaver-build/journaux-cx-recherche-confinement-12nov/` : recette-proposee.json, baseline-avant-proposee.json, preflight-unique.log, sonde-unique/{sonde.json,chrome-stderr.log}, observations-independantes.json, quiescence-reel.json, verdict-controle.json, reel-unique/validation-zsbgdsf_/{recette.json,bus.xml,autorite.json,enfants.json,verdict.json}, journaux CPU historiques et courants. Reçu final terminé ; aucune autorité active conservée par les fichiers après fin. Les scripts run_e2e et temoins_navigateur adoptent la primitive, leurs entrées sont jugées par les témoins CPU sans Cargo ; cette préflight éprouve le superviseur/bus/navigateur réel sans rejouer les lots clos.

Verdict et diff soumis à C7/ED pour revue avant intégration après A. Ticket toujours ouvert jusqu’à revue/intégration de l’adoption commune ; aucun push code master pendant A sans coordination, aucune nouvelle exécution réelle demandée par ce vert. Worktreescx-recherche-confinement-lanceurs propres aux deux postes et preuves conservés ; aucun target Cargo créé pour ce lot.
