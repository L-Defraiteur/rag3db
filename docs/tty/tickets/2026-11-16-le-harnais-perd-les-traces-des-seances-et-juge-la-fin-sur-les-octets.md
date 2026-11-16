# Le harnais tty perd ses traces par séance et juge l'affichage sur les octets

- **État** : ouvert — diagnostic de validation après continuation9d, aucune cause produit prouvée, aucun correctif joué.
- **Famille** : produit
- **Gravité** : coût et fiabilité du verdict de validation ; un affichage absent ne se distingue plus d'un oracle de flux brut insuffisant.
- **Atteignable en service** : non, concerne le harnais privé de qualification tty, pas un hôte partagé.
- **Touche rag3weaver** : oui, tty/mesures/mesure.py et faux_hote.py.
- **Pour** : tty.
- **Signalé par** : qualification du ticket `docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`, GO C7-TTY-CONT-9D8-16nov-RESTE, demande de diagnostic RO C7.

## Constat, preuves et limites

Produit6d/sourcecontrôleur9d, bâtiTTYrc0 ; F8 8/8 porté. Harnais tmux local
ouvert : premier rouge « après o, la fin ne paraît pas », puis indicateurs
Kitty absents malgré texte multiligne correct. Interruption Common propriétaire
au premier rouge **observé**, aucune reprise. Deuxième observation traitée dans
le ticket tmux15 existant, pas de doublon de défaut clavier.

Preuves figées `~/.cache/rag3weaver-build/archives/cx-tty-cont-9d8-raw/evidence.tar`
aux deux postes, SHA7cf79c5a681ac16900067227020d18d8d5524df817cab482e5c5dcd14812d188.
P `cx-tty-aliases-continuation-v1` : harness.log et first-red-stop.json.
Observation/TERM via pidfd851326.401709478, autorité705646/start85125068 ;
fincomposée851326.668131734, CommonTERMINÉ/restes[]/nettoye true, erreur15
explicite de l'arrêt propriétaire, pas une panne spontanée attribuée au produit.

**Absences assumées** : Terminal.recu garde ses chunks et timestamps seulement
en mémoire ; ecrire ne persiste pas les frappes/horodatages. hote_demarrer
supprime journal.jsonl/faux.pid à chaque séance. Le journal restant (3POST,
requestgo et describe) appartient à la dernière séance et ne prouve rien sur
l'accord de la séance principale. Pas de capture-pane de cette séance conservée,
pas de harness-result.json final après interruption. Aucun de ces éléments ne
peut être recréé a posteriori. La séquence de touches dans le script est un plan,
pas un journal observé. Le temps d'émission du premier rouge n'est pas connu ;
seul son premier constat par le contrôleur est horodaté.

## Analyse statique, sans essai ni API réelle

- La séance principale utilise le **terminal riche**, appelle Ctrl-G puis Échap,
  puis o ; aucun --plain ni ended-suivi-de-relance n'est exercé. Ce n'est donc
  pas le même contrat que `2026-11-16-le-mode-plain-et-la-fin-d-un-pas-relance.md`.
- Le faux hôte ne publie ended/stateended que pour answer avec approvedtrue.
  Les requêtes observées de la séance concernée n'étant plus conservées, on ne
  peut pas conclure si l'accord a été posté, la fin publiée ou son rendu absent.
  Ne pas modifier le protocole du faux hôte ni les états du produit par hypothèse.
- L'oracle extrait une phrase d'animation et cherche ses octets contigus après
  la frappe o, ou « fini sans répondre ». `Terminal.attendre` concatène les
  chunks du flux brut ; il n'applique pas les séquences VT/diff de ratatui/tmux.
  Une phrase visuellement présente peut être assemblée par des écritures
  partielles ; cette possibilité n'est pas une preuve du faux rouge observé.
- Le harnais sait déjà capturer un écran tmux (`ecran_tmux`, séance1f), mais
  cette primitive n'est pas utilisée pour ces deux assertions antérieures.
- Vue/clavier/animations/fauxhôte/harnais/thèmes sont sans diff entre927 et6d.
  Le lot d'alias change commandes/main/README, pas ces mécanismes. Absence de
  diff ne prouve pas l'absence de régression ; elle interdit l'attribution par
  simple coïncidence. L'aide CLI anglaise n'est imprimée qu'en cas d'option
  refusée ; les arguments français du harnais sont acceptés et le bâti est vert.
- Les seules API jouées sont celles du faux hôte privé17990. Ni l'hôte927 ni
  ses bases n'ont été interrogés ; aucune incompatibilité API927 établie.

## Plan de diff proposé, pas exécuté

1. `mesures/mesure.py` : un dossier exclusif par séance ; persister chunks bruts
   avec timestamps, frappes réellement relayées, commande/env non secrets,
   chronologie des assertions ; conserver les journaux du faux hôte avant
   d'ouvrir la séance suivante. Pas de suppression rétrospective des rouges.
2. Pour l'accord : exiger et archiver le POST answer/approved/reply_to ainsi que
   la ligne ended/state du faux hôte, séparément du rendu. Puis juger le rendu
   sur un écran reconstruit/capture-pane privée pendant la séance, sans tester
   une chaîne ANSI contiguë comme si c'était une image.
3. Réutiliser la primitive capture-pane existante avant les premières séances,
   garder le brut en complément. Tests de fragments VT factices et d'isolement
   de journaux avant tout runtime ; ne pas affaiblir les assertions produit.
4. Seulement après revue/autorisation et reçu frais : scénario ciblé borné,
   comparaison baseline avant aliases et candidat si nécessaire ; pas de
   rejeu F8/build sans changement de source justifié. Tout défaut produit
   confirmé aura son ticket/rouge avant correctif.

## Critères de fin

Les traces par séance survivent à une interruption et au démarrage de la suivante ;
les oracles distinguent protocole, état, écran et transport. Un témoin discriminant
rouge puis vert sur flux fragmenté, et un scénario d'accord privé qualifié,
permettent une attribution concrète des observations. Aucun vert déduit du
présent diagnostic ; aucune nouvelle allocation ou ancien END réactivé.

## Reprise du correctif de harnais autorisée, avant diff

C7 autorise les traces append par séance, raw/input/chrono/journal fauxhôte/env
non secrets avec fsync aux assertions/arrêts ; oracles protocole et écran
séparés, primitive capture-pane déjà utilisée1f. Témoins hermétiques publiés
avant essai : frame correcte dont le raw est fragmenté refusée par l'ancien
observateur, acceptée par l'observation d'écran ; journaux conservés entre
séances. Pas de requalification du run perdu ni de correctif produit.
SourceRustTTY reste inchangée. L'END852082.791694931 est constaté expiré à
now852108.599496 : aucun run natif possible sur ce budget, aucune nouvelle
allocation inventée. Prochain paquet RO devra refuser allocation absente/expirée.

## Correctif de harnais publié et témoins observés

Contre-exemple24aef900cb publié avant essai : deux échecs hermétiques réels,
frame correcte fragmentée refusée par l'oracle raw et ancien texte raw accepté
malgré écran courant différent. Correctif7daf342c95 : oracles tmux par
capture-pane déjà présente, protocole answer-approved/reply_to/ended/state
jugé séparément ; trois objets sont donc distincts : transport, protocole, écran.

Quatre témoins hermétiques passent ensuite : ces deux discriminants, isolation
append des traces/raw/input/screen/journal entre phases et refus de réutiliser
un dossier, indépendance du verdict protocole et écran. Aucun terminal, Cargo,
hôte, tmux ou natif lancé pour ces essais ; le run perdu reste non qualifié.

Trace par séance privée : chunksraw et frappes réellement écrites horodatés,
commande/env non secrets, journal fauxhôte non supprimé ; captures persistées ;
fsync aux assertions, aux arrêts normaux et à SIGTERM. Hôte faux local détenu
par Popen, arrêté via ce propriétaire, aucun nom de processus partagé. Modes
SSH/mosh conservent leurs oracles historiques raw et ne sont pas requalifiés
par ce correctif ciblé tmux ; captures tmux hors local restent voie existante.

Paquet `prepare_harness_review.py` prépare copies/hash/sourcepropre, F8porté,
ELFtty déjà bâti rehashé et sourceRust inchangée (seuls3fichiers mesuresPython
changent sous extension/rag3weaver). Sa gate native refuse ENDoriginal expiré,
refuse de transformer préparation en nouvelle allocation ; aucun GO réactivé.
Avant tout natif futur : revue de l'intégration, commande/env privés et budget
neuf explicitement décidé par C7, pas de fenêtre inventée dans le paquet.

## Réserve unique de revue97c, avant correction

Mémoire : approval_and_end accepte tout reply_to non vide ; le faux hôte publie
ended/stateended pour tout answerapprovedtrue et ne valide pas l'identifiant.
Un answer reply_to faux/mesure:999, avec approval réelle faux/mesure:0 et
ended/stateended, ferait donc passer l'oracle à tort. Le préfixe seul vérifié
plus loin ne suffit pas. Corréler exactement à l'unique approval attendue de
**cette phase** ; contre-exemple publié avant fix et essai hermétique. Aucun
changement produit ni assouplissement du faux hôte, pas de nouveau runtime.
Revue97c fin totale : un seul point, autres contrôles bornément reconnus.

## Identifiant d'accord corrélé exactement, réserve97c corrigée

341624693f publié avant essai :6tests,3échecs observés (identifiant étranger
au bon préfixe, approval absente et approvals multiples). Le correctif exige
une seule approval avec id non vide dans la boîte de cette phase, et un POST
answerapprovedtrue dont reply_to égale **exactement** cet id. ended et state
ended restent requis indépendamment. Aucun changement du faux hôte/du produit.
Le paquet de relecturev2 est distinct de v1 ; gate native toujours fermée sur
l'ENDoriginal expiré. Aucun témoin natif ni validation du run perdu déduit.

## Protocole exécutable harnais seul demandé après RO8534

Mémoire8534 fin totale favorable, réserve de corrélation levée ; aucun natif
joué. C7 autorise la préparation d'un protocole **HARNAIS SEUL**, même ELF déjà
bâti et sourceRust/SDK figés, F8 et build portés, nouveaux P/Q/S. Borne proposée
**1200s au total**, file de poste, Common et finalisation comprises ; ce n'est
pas une allocation. END852082.791694931 expiré reste historique et fermé.
Préflight provenance/quiet sous vrai poste, premier rouge observé arrêté
immédiatement via autoritéCommon propriétaire, final après TERMINE/restes[]/
nettoye/hash physiques/quiet/publication dans nouvelle échéance éventuelle.
Publier source/pilote/paquet puis RO avant tout GO distinct.

## Réserve C7 sur protocole5b43 : budget total, avant correction

HOLD, aucun run. Ticket coordination a483d0fdc9 : subprocess.run(timeout)
borne le poste/Common seulement ; après retour, snapshot/hash/scan/Git et
publication du parent restent hors watchdog. Le contrôle d'horloge refuse
un VERT tardif mais ne garantit pas une exécution totale bornée à1200s.
Correctif groupé demandé : watchdog extérieur englobant file, poste, Common,
toute finalisation et publication ; entrée ultime ne fait que transmettre
le rc. Témoin sur vraie construction et finalisation bloquée, publié avant
essai hermétique, rouge avant correction. Pas d'ajout implicite au budget,
aucun natif/ancienEND réactivé ; nouveau Q/pin et RO totale avant GO.

Le premier essai71d57 du témoin a échoué en0,046s sur son rc, sans atteindre
la finalisation bloquée : env manquait dans le contexte AST factice. Ce n'est
pas le rouge du budget. Fixture corrigée/publiée avant le nouvel essai ; aucune
correction du pilote n'a encore été faite, revue groupée Mémoire en cours.

## Watchdog extérieur total, correction groupée après fin de revue5b43

Mémoire fin totale rectifiée : une réserve, celle du budget global C7 ; aucun
autre point en file. Témoin c2192e3fa8 publié avant essai : la vraie queue AST
du main bloque dans son snapshot3s et dépasse effectivement le budget factice
0,2s (3,0419s observées), sans Common ni natif. Le premier échec de fixture
reste documenté séparément, non présenté comme rouge de cette propriété.

Entrée ultime désormais remplacée par exec de `/usr/bin/timeout` autour du
**worker entier**, entrée/file/poste/Common/snapshot/hash/scan/Git/publication.
Elle ne fait rien après exec et transmet donc uniquement le rc du watchdog.
Borne globale proposée1200 : arrêt doux calculé à END−6s, KILL au plus5s plus
tard, marge d'entrée1s **incluse** dans1200, aucun +5s ajouté au budget.
Le worker reçoit le START de l'entrée et garde le même END ; il refuse un
parent qui n'est pas le watchdog. Common reste borné au reste après poste
moins15s, pour finaliser avant l'échéance. Les scans et la publication restent
à l'intérieur du watchdog, et leurs checks d'échéance restent conservés.

Nouveau Q/P/Sv2 et TMPttyhr2, ancien paquet5b43 intact. Second témoin sur le
vrai main : exec du watchdog avec le worker privé, aucune finalisation après.
Réessais hermétiques seulement après publication ; aucun GO/runtime natif.

Préparation a0ea : refus avant tout runtime/autorité, chemin historique de reçu
flsb10fe renommé par erreur continuation-common-v2 au lieu de v1 lors du
renommage du nouveau paquet. Qv2 partiel conservé, aucun prepared.json produit.
La propriété watchdog a passé ses2témoins hermétiques (0,146s) ; ce refus est
un défaut distinct de préparation, pas un run rouge ou un vert de harnais.
Référence historique désormais constante PREVIOUS_SCOPE/v1, test de séparation
des numéros ; nouveau Q/P/Sv3 et TMPttyhr3. Aucun ancien dossier réécrit.

## GO C7-TTY-HARNESS-91E5-16nov1200 — premier rouge phase0008, preuves complètes

Source/pilote91e5ed5fc11e565f238478d946d3e994146e1e8d, Qv3/preparedcf99…ab95.
Harnais seul sur ELFcb24, aucune F8/build/Cargo rejouée. Nouvelle allocation
START854767.847527584/END855967.847527584 ; Commonreste1184 après poste.
Phases2/5/6 passées sans rouge : protocoleaccordOK et captureécranfin, puis
les deux variantes Kitty avec captures, saisie multiligne jugée. Ceci valide
ces observations nouvelles seulement, sans réinterpréter le run perdu9d.

Premier rouge phase0008, séance !/fauxsudo : invite_vuefalse, retour_vufalse,
consignes1 mais consigne_porte_le_retourfalse ; marqueur de mot de passe
**FACTICE** vu dans journal et raw (booleans true), pas dans la consigne.
Aucun lancement sans frappe relevé. Cause encore non attribuée au produit.
Les raw/input horodatés, journal fauxhôte et checkpoints de cette phase
existent désormais, contrairement au run9d. Aucun secret réel n'a été saisi.

Arrêt automatique dès première ligneROUGE après flush/fsync : pidfd autorité
Common841941/start85476802/scriptQ/cgroup vérifiés, observation854883.759418159.
Scope `cx-tty-harness-only-common-v3/validation-j4l20f5g` TERMINÉ,
ROUGE code1/signal15 explicite, restes[], nettoye true, pic71847936octets/oom0.
Composé ROUGE final854883.900733566, avant END, aucun retry ni nettoyage.
Wpropre ; harnais ENTIER NON ACQUIS. P `cx-tty-harness-only-v3/observations/`
garde8phases et `first-red-stop.json`, Qcomposé et tous les reçus conservés.

Défaut contrôleur identifié AVANT toute correction : deux lignesROUGE sont
arrivées avant arrêt effectif ; le lecteur a tenté une seconde open(x) du même
first-red-stop.json et levé FileExistsError. Le premier marqueur et le premier
signal sont bien conservés, Common a quiescé. Prévoir un arrêt idempotent/une
seule émission du signal puis attente de terminaison, témoin discriminant
avant fix. Aucune correction ni attribution bang implicite sur ce run.
