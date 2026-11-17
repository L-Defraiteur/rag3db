# Producteur minimal : protocole concret proposé avant tout backend

Autorisation C7d72aa02288 et suffisance pure892bc/51verts. Cette publication
contient une API .pyi, une recette INACTIVE avec argv, des pins de sources et
spécifications de témoins. **Aucun backend OS, helper de lancement ou appelant
n'est codé/exécuté.** Aucun chemin P/Q/S runtime n'est créé. Le module pur565b
et les archives f74/2a/892bc restent intacts. RO puis arbitrage C7 avant code.

## 1. Fichiers, entrée CLI proposée et mode de la première recette

Futur module réservé par Archi après constat de nonchevauchement Recherche : `scripts/execution_entry_runner.py` ; aucun
fichier de ce nom n'est livré. Futurs contrôleurs privés Q/native_recipe.py et
Q/native_fixture_publisher.py également ABSENTS. Q désigne la copie extérieure
au WT, pas un lien vers des fichiers en cours d'édition. Voir les chemins absolus
et cinq argv dans `native-recipe.inactive.json`. Tous les SHA futurs sont NULL :
**execution_ready=false**, mode **FIXTURE_ONLY**, aucun coupon/slot réservé.

La source de la cible proposée est le fichier ELF `/usr/bin/printf`, dont seule
la lecture/hash de préparation est effectuée. Le futur GO préparerait sa copie
privée P/fixtures/program-A (absente actuellement), mêmeprofil/mount que le test
de write-denial. Sa compatibilité de contexte après copie reste à qualifier. Commande enfant déclarée :
`["P/fixtures/program-A", "ENTRY_FIXTURE_%s\n", "A"]` ; stdout attendu exactement
`ENTRY_FIXTURE_A\n`, stderr vide, wait exit0. Aucun shell pour cette invocation,
aucun SDK/Rust/Cargo/DB/provider. Le programme n'a PAS été exécuté pour préparer
la recette. Son ELF, son interpréteur et le profil de stockage devront être
réadmis dans le petit GO natif ; les hashes présents ne sont pas une qualification.

CLI future, dont le fichier doit encore être écrit/relu/épinglé :

```text
/usr/bin/python3.14 -B Q/execution_entry_runner.py run
  --declaration Q/declared-invocation.json
  --mode FIXTURE_ONLY --proofs P/invocation-printf-A
  --authority-from-common-env
```

Les symboles Q/P de ce bloc sont remplacés par les chemins littéraux de la
recette JSON ; pas d'expansion shell ni de découverte par glob. Refus si mode
non déclaré, fichier/Q/hash/authority absent, appariement LIVE/fixture ou coupon
non admis. Un script absent/option proposée ne constitue pas une commande prête.

## 2. Plan déclaré puis admission du runner : aucune identité future prédite

Avant Common : figer déclaration/source/locks/features, programme SHA, argv,
cwd, environnement public attendu, sinks, labels et fichiers du plan CP.
Aucun PID ni hash d'ACK futur dans cette déclaration. La liste des publications
ne change pas depuis l'objet découvert. A/B ne devient pas attendu depuis actual.

Après vrai démarrage du runner sous Common, AVANT son fork : vérifier le reçu
avec la logique existante `autorite(processus())`/signature_superviseur du
Common épinglé, son ascendance et le poste réel. Relever runner réel, chaîne
jusqu'au Common et sa racine directe ; comparer les commandes/scripts/exe
aux attendus de la déclaration et de recette Common. Figer ensuite l'admission
runtime et son SHA. Elle instancie le plan pur avant l'enfant, pas avant le
runner déjà vivant. Les objets HeldEntry/EntrySeal/ReleasePermit ne pourront
pas être construits depuis les booléens des51fixtures ou un PID libre fourni.

Les variables secrètes de l'autorité ne sont ni sérialisées ni hashées dans un
log. Héritage opaque uniquement depuis Common réellement vérifié ; journaliser
la politique/presence, pas le jeton. L'environnement public est comparé aux
attendus de phase, jamais initialisé depuis les valeurs observées pour réussir.

## 3. Bootstrap précis et confirmation d'entrée

1. Parent runner monothread, enfant bootstrap monothread contrôlé. Ouvrir l'ELF
   déclaré O_RDONLY|O_CLOEXEC, refuser non régulier/script/privilegié/profil
   stockage non qualifié ; vérifier hash initial. Ouvrir sinks exclusifs0600,
   sans symlink, sous preuve approuvée ; stdin=/dev/null. Pipe d'erreur séparé
   close-on-exec, non interprété comme preuve positive d'entrée.
2. Allouer OwnedLaunch/gate/registre AVANT fork, puis `fork()` contrôlé. Dans l'enfant : rediriger0/1/2, fermer les FD étrangers
   (sauf FD ELF/pipe), TRACEME puis SIGSTOP, puis `os.execve(fd, argv, env)`.
   Une erreur syscall conserve errno via pipe si possible puis termine ; aucune
   annonce SUCCESS. Pas de PATH/shell, exec FD indisponible = refus sans fallback.
3. Parent : enregistrer PID/start/UID/CG et pidfd AVANT toute récolte. Un seul
   propriétaire de wait, avec WNOHANG et échéances fraîches. Attendre le SIGSTOP
   de bootstrap du bon enfant, configurer TRACEEXEC+EXITKILL, continuer. Tout
   autre signal/stop, identité changée, enfant mort, EPERM, traceur présent,
   syscall refusé ou deadline ferme la passe. Aucun contournement Yama/Landlock.
4. Seul le wait décodé du bon PID portant **PTRACE_EVENT_EXEC** est confirmé,
   pas SIGTRAP seul, intention, EOF ni returncode. À cet arrêt avant userland :
   revalider enfant/start/CG, ouvrir son exécutable RÉEL depuis /proc/PID/exe,
   vérifier dev/inode contre le FD sélectionné et SHA contre le programme admis.
   Le parent ne rouvre pas T. Génération observée : cette entrée seulement.
5. Toujours arrêté : copier depuis CE FD réel vers entry-object.pending,
   hasher/rehash, fsync puis scellement exclusif. Revalider identité/stabilité/
   END après copie et avant lien final. Sceller les faits canoniques d'entrée
   et leurs SHA ; les transmettre au port CP, jamais publier un simple attendu.

[Python execve FD](https://docs.python.org/3.14/library/os.html#os.execve)
requiert le support de la plateforme ;
[ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html) fournit événement
exec/détachement. Leur disponibilité réelle n'est pas déduite de cette lecture.
Les précédents source `superviser-elf.py`, `capture-stable.py`, génération GDB
`driver.py:393` ont été relus : capture exec/main, libération et wait naturel
sont réutilisables comme frontières. Le candidat minimal retient TRACEME du
parent naturel, PAS GDB ; aucun ancien helper/END/GO n'est rejoué, aucun rcGDB
n'est assimilé au wait du programme. Si GDB devenait nécessaire : nouvelle RO.

## 4. Stabilité : condition physique explicite, pas deux snapshots

Profil proposé : ELF local régulier sur stockage Linux garantissant l'exclusion
d'écriture pendant qu'il est utilisé comme exécutable, sans écrivain hostile ou
mutation hors modèle. Le choix par FD ferme le changement de chemin, pas les
bytes avant exec. À la barrière, la copie de l'exécutable réel et son hash doivent
correspondre au SHA indépendant ; le maintien de son usage exécutable doit
empêcher une mutation pendant cette copie/attente CP. Distant/FUSE/DAX ou support
inconnu ne seront pas admis par simple type stat. Pas de relocation/memfd choisi
silencieusement ; éventuels usages $ORIGIN/cwd doivent conserver leur contexte.

Références : [fexecve](https://man7.org/linux/man-pages/man3/fexecve.3.html),
[execve/ETXTBSY](https://man7.org/linux/man-pages/man2/execve.2.html),
[fs/exec.c](https://github.com/torvalds/linux/blob/v6.18/fs/exec.c) et
[kernel/fork.c](https://github.com/torvalds/linux/blob/v6.18/kernel/fork.c), modèle
sourcev6.18 PAS noyau hôte qualifié. La future recette devra établir ce profil
sur SON espace privé, avec contre-exemple d'écriture refusée pendant la barrière.
Si la stabilité n'est pas démontrable dans le montage : STOP avant release.

## 5. Publication avant release : un seul CP, fixture jamais grant LIVE

Le parent vivant est le propriétaire déclaré de fenêtre, jamais l'enfant arrêté.
Après scellement ledger/faits/copie et ACK antérieurs : ouverture de fenêtre par
le vrai rôle via CP, MIN(cap,ENDop original), puis publication sur LE moteur CP.
Son plan préautorisé contient entry-ledger et entry-object ELF ; le ledger porte
la déclaration, le plan runtime et SHA faits canoniques, distincts du SHA contrat.
ACK lie prepared/plan/phase/séquence/ledger/ELFmap/faits. Références héritées =
ACK accepté + rehash, pas une copie seule. Le port ne réécrit aucune boucle SSH,
copie/ACK/Window ; il attend une attestation CP réelle et relue. Adaptation des
types/canonicalisation et rôle owner encore À RO avec CP, non qualifiés.

**LIVE reste HOLD** tant que gardes/adaptateur CP ne sont pas qualifiés. Un
éventuel `native_fixture_publisher.py` ne fait qu'éditer une réponse privée
**FIXTURE_ONLY**, étiquetée `cp_ack=false`, `global_admission=false`. Son permis
peut libérer uniquement une invocation elle-même FIXTURE_ONLY ; un permis
fixture sur LIVE refuse AVANT detach. Le validateur pur peut vérifier les
liaisons du modèle ; il ne transforme pas cette édition en ACK CP. La première
recette ne prétendra donc qualifier que le mécanisme d'entrée en montage fixture,
PAS le raccord CP ni la passe globale. Aucun faux grant ne sera exporté.

Avant release : comparer permis au mode/déclaration/plan runtime/child/seal/END,
rehash des preuves et horloge fraîche. ACK absent/erroné/tardif : pas de detach
positif, première cause puis arrêt/récolte attribués. Après ACK, detach réussi
conservé ; ni son succès ni un sondage ne promettent des exec futurs observés.
Le programme peut déjà finir avant une lecture ultérieure de TracerPid : pas
exigée comme condition à inventer après coup.

## 6. Wait, Common/dbus et séparation des clôtures

Le runner, parent naturel de l'enfant forké, garde le waitpid brut jusqu'à
WIFEXITED/WIFSIGNALED. Ni stop ni EOF ne deviennent terminaison. Conserver status
brut et rc signé, stdout/stderr exacts/hashés et références ; résultat de
l'INVOCATION, pas d'une image A inaltérée. Signal/erreur producteur restent
première cause sans réécrire un rc enfant déjà authentiquement acquis.

Common6439 lance **dbus-run-session** par Popen(start_new_session=True), capture
son start AVANT poll, suit ses descendants et reape au cleanup. La racine Common
est CE Popen, pas le runner par supposition. Le plan runtime doit refléter cette
chaîne. Le runner ne reape pas son ancêtre dbus ; le contrôleur extérieur ne
prétend pas waitpid(runner) s'il n'est pas son parent.

Raccord proposé sans nouveau parentage : le runner scelle invocation-finalized
puis retourne ; Common garde son vrai poll/wait racine. Le coordinateur extérieur
consomme dans LE scope approuvé recette/autorite/enfants/verdict après retour.
Il lie la racine observée dans l'admission à l'enfant/root du Common épinglé,
source/reçu/identités inchangés ; exige verdictVERT/code0/erreurNone/restes[]/
nettoye/authoritytermine et quiet attribué, avant son propre résultat. Lien root
absent/ambigu ou wait non attribuable = UNKNOWN/refus, jamais tuple inventé.
Si les fichiers actuels ne portent pas ce lien sans ambiguïté, STOP et proposer
un descripteur Common explicite à sa propre RO ; aucune extension Common livrée.
Un verdictROUGE/code par défaut n'est pas converti en wait authentique inventé.

La clôture Common et publication finale sont extérieures au runner/sous-arbre
qu'elles déclarent quiet. Le contrôleur attend son propre enfant de recette
(poste/timeout), pas un descendant libre ; source Common prouve le wait interne.
Terminer l'autorité ne lui donne aucun nouveau temps : fin de copie/manifestes/
prépublication sous ENDCommon ORIGINAL et soft END extérieur, sans réouvrir Gate
actif sur une autorité terminée. Le raccord de finalisation CP doit être relu ;
la première fixture écrit ses artefacts privés, pas un nouveau grant CP terminal.

## 7. Bornes concrètes proposées pour le petit pilote, pas un GO

Origine logique120s émise par deadline-bootstrap AVANT Popen du timeout
physique120s/KILL5 ; queue MAX15s DANS cette borne. Aucun tick proc ne calcule
une échéance CLOCK_MONOTONIC. Le bootstrap reste parent du timeout et conserve
son vrai wait ; identité et source/argv du bootstrap sont vérifiées séparément.
Le paquet initial est immuable, le second lie son SHA au vrai Popen timeout.
Le contrôleur outer lance poste avec le sous-mode `admit-common`. Celui-ci,
APRÈS acquisition réelle, vérifie le timeout ancêtre et le poste, calcule le
budget frais puis exec Common. Le budget n’est PAS calculé avant la file ;
les cinq argv distincts bootstrap/outer/poste/admission-Common/enfant sont dans la recette.
Common demandé floor(MIN(90,softENDextérieur−maintenant−15)), refus<=30.
ENDop=ENDCommon−15, réserve arrêt/finalisation15 DANS Common. Chaque opération
à MIN(cap,reste frais) : bootstrap/exec5, capturecopie10, publicationACK20,
waitprogramme5, finalisation10 ; aucune somme ne crée un budget. Refus de copier
ou publier si la réserve ne tient plus. L'arrêt/reap final, fsync/hashes et
prépublication revérifient les bornes originales ; UNKNOWN ne devient pas vide.

Un seul hôte pour ce premier mécanisme ; durées seules si un port CP futur parle
à un autre hôte. Les copies de preuve ultérieures ont leur budget extérieur
original, pas un nouveauCommon. Les pics/overhead, compatibilité et durée réelle
sont UNKNOWN, pas estimés depuis printf ou les51témoins purs. Aucun créneau
réservé. La recette demande poste2G/swap0/CPU100%, pas de Cargo/j8 ; ces valeurs
sont proposées à arbitrage et doivent être attestées après vrai poste.

## 8. Modes d'appel ultérieurs et refus

| Mode | Identités et surface à qualifier après ce petit pilote |
|---|---|
|Direct e2e|Quatre sites970/989/998/1001 vers le même runner ; filets et --list inclus, flags/RAW exacts|
|Cargo target.runner|Cargo parent du runner, runner parent du test ; argv et LD réels de Cargo comparés au profil attendu ; les enfants ne sont pas couverts par ce hook|
|Shell|Shell parent du runner, pas exec du test en shell ; commande shell arbitraire et script root refusés par le profil initial|
|Common|Owner extérieur au sous-arbre, racine Popen réelle (souvent dbus), wait de la racine au Common ; root=runner seulement si réellement mêmePID/start|

Pas de code d'appelant/Cargo/canon modifié ici. Ni traceur persistant, fork/clone
universel, syscall tracing, privilège ou sandbox changée. Après detach : images
futures UNKNOWN. SDK chargé nécessite son point au PID réel séparé et reste
NOT_ATTESTED. Tentatives échouées non comptées exhaustivement ; sans EXEC confirmé,
aucune preuve positive n'est délivrée.

`native-witnesses.inactive.json` prépublie les cas et observables à coder avant
le backend. Ce sont des SPÉCIFICATIONS, NON ÉCRITES EN TESTS/NON JOUÉES, pas des
nouveaux verts. Avant toute implantation : RO du protocole/API/commandes/pins,
fermeture des liens Common et ports CP (ou isolement FIXTURE_ONLY explicitement
accepté), puis arbitrage C7. Les sources futures et témoins exécutables devront
être publiés/relus avant un GO CPU/native distinct. Rien n'est lancé par ce doc.


## 9. Delta CPd81 : origine attestée et payload canonique déclaré

Préconditions consignées au ticket avant ce delta. Une cinquième commande,
`native_recipe.py deadline-bootstrap`, émet START=time.monotonic() et
softEND=START+120 AVANT de lancer par Popen le timeout physique déclaré. Le
bootstrap vivant, de source/Q/argv/identité préautorisés, est le producteur de
cette origine ; JSON seul ou identité trouvée depuis son propre fichier ne
suffisent pas. Le coordinateur/admit-common vérifie son identité complète et
la relation parent→timeout, source/exeSHA/argv/UID/CG contre le plan indépendant.
Les ticks proc servent seulement à la non-réutilisation du PID, JAMAIS à produire
ou traduire la borne. Un second fichier exclusif timeout-started lie l'origine
scellée au véritable enfant Popen, sans réécrire le premier fichier.

L'origine précède Popen/exec : latence de montage ou de fsync consomme la borne
logique ; la minuterie physique plus tardive n'accorde pas de rallonge. Tous
les contrôles et publications gardent MIN(originalSoftEND, ENDCommon), même si
le timeout physique tourne encore. kill_end ne donne jamais du temps de travail.
Si le montage ne permet pas de prouver ce rôle émetteur et la chaîne réelle :
refus AVANT ARM/queue. Le bootstrap lui-même reste sans capacité de validation
pendant son montage ; aucun travail ne démarre avant timeout et admission.

Le StageSpec proposé déclare EXACTEMENT trois pièces : entry-ledger.json
(ledger), entry-object (executable) et entry-facts.json (data). Faits canoniques
sérialisés comme CP canonical_sha, sans newline : digest du fichier = SHA des
faits canoniques ; pas de SHA appris depuis une autre représentation indentée.
Ledger contient ce SHA et le SHA objet, puis ACK.files contient les trois SHA et
son ledger_sha256. Les faits n'incluent ni hash du ledger ni ACK futur : pas de
cycle. Le contrat statique source_contract et les SHA de faits restent distincts.
Les champs futurs/nulls et le mode fixture gardent ce plan NON ADMISSIBLE LIVE.

Deux specs ajoutées : N17 décale artificiellement procstart sans changer l'origine
émise (aucune rallonge), N18 payload faits omis/changé malgré ledger/ACK d'une
ancienne représentation (refus avant release). Elles restent NON CODÉES/NONJOUÉES.
Le raccord de finalisation CP quand Common est terminé demeure un port distinct
non fourni : pas d'appel Gate.active sur une autorité morte. FIXTURE_ONLY écrit
ses artefacts privés avec cp_ack/global_admission faux, sans valider ce port.


## 10. Delta groupé M/R/TTY : cas causaux et propriété avant HeldEntry

N04 devient un POSITIF : ouvrir FD A, remplacer le chemin T par B, exécuter le
FD original puis confirmer/coller A. Le chemin T actuel n'est pas rouvert pour
forcer un refus. N19 distingue la véritable image B au stop, injectée uniquement
en fixture contrôlée, indépendante de la déclarationA : mismatch refuse avant
corps, sans promouvoir B ni falsifier les faits. Aucun de ces deux cas n'est joué.

N06 exige le MÊME objet privé/dev/inode/UID/montage : ouverture écriture et
écriture de byte identique autorisées avant fork, écrivain fermé AVANT exec ;
puis tentative d'ouverture écriture attribuée au parent pendant le stop EXEC,
avec errno ETXTBSY observé ; enfin contrôle écriture à nouveau possible après
reap et fermeture des références exécutantes. Seul ETXTBSY causal qualifie le
profil ; EACCES/EROFS/fichier système non inscriptible/UNKNOWN ne suffisent pas.
Aucune mutation de /usr/bin/printf n'est proposée. La copie privée n'existe pas,
sa compatibilité n'est pas supposée ; tous ces contrôles nécessitent le futur GO.

Le caller possède OwnedLaunch AVANT tout fork et garde cette capacité en finally.
Immédiatement après fork, le retour PID de l'enfant naturel est enregistré AVANT
pidfd/lecture proc/attente qui peuvent échouer. L'enfant reste sur une gate pipe
jusqu'à l'admission de son identité/pidfd par le parent ; EOF de cette gate est
un ordre d'abandon, jamais une preuve d'exec réussi. Échec pré-pidfd : fermer gate
puis attendre/reaper cet enfant naturel, sans signaler un PID découvert libre.
Si la récolte n'est pas établie : UNKNOWN/recours Common sous mêmeEND, pas vert.

Protection proposée du bootstrap enfant avant gate : PR_SET_PDEATHSIG(SIGKILL)
et contrôle getppid contre le runner après installation, puis EXITKILL au stop
TRACEME. Ce sont des mécanismes de vie/mort locaux à qualifier, pas un changement
de privilège/sandbox. Ils ne prouvent pas eux-mêmes le cleanup ; N21 vise leur
fenêtre de course. Common garde son nettoyage indépendant. Après acquisition
pidfd/identité, tout arrêt suit uniquement les références attribuées. Toute
exception de start_and_hold lance/consigne le cleanup de OwnedLaunch ET laisse
ce même handle au caller ; HeldEntry=None n'est jamais «aucun enfant». Première
cause immuable, échec cleanup secondaire conservé, fin sous END original.

L'avis Recherche est un nonchevauchement, pas un transfert de propriété backend.
Archi porte cette préparation ; attribution d'implantation reste à C7. L'API
opaque .pyi ne livre aucun constructeur OS. Total21spécifications NONCODÉES/
NONJOUÉES, pas21tests ni qualification CP/ELF/SDK/Cargo.
