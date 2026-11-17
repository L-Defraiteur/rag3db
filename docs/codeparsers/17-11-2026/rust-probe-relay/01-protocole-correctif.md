# Relais autonome pour les deux publications : contrat avant implantation

Préparation corrective autorisée par C7 au ticket relais du17novembre, sans test,
Cargo, nouveau run natif, autorité ou ACK à P02. Ce document et les témoins du
moteur réel sont publiés avant son implantation. Aucun résultat de test inventé.

## Responsabilités et déclenchement

Le coordinateur Python tourne sur le portable depuis la copieQ hors W/T. Avant
le SSH de phase, il publie sa disponibilité et son identité PID/start/UID avec
SHA du script et de prepared.json, initialise son moteur de relais et ses logs.
Il lance ensuite SSHexec de la phase PC en concurrent, sans attendre une
notification à la session. Le transport SSH réel est le seul adaptateur réseau ;
les tests factices remplaceront le transport et l'horloge, pas la logique de
relais, d'ordre ou de publication. Il n'y a aucun service installé ni processus
partagé détaché.

Sur le PC, gate.py lit passivement les preuves privées et ne transfère jamais
le jeton d'autorité ni un environnement complet. Avant chaque export et chaque
publication ACK, il exige : source et copies de contrôleurs aux SHA figés,
prepared.json exact, bon hôte/UID, absence de first-red/relay-abort/classification
et de verdictCommon final, autorité active, identité Common et identité du
pilote courant PID/start/UID/exe/argv/cgroup exactes, scriptSHA du pilote inchangé,
échéance non atteinte. Le pilote inscrit son identité avant la compilation.
Les checks se répètent après les opérations et avant le lien final d'un ACK.
Aucun ACK ne peut être écrit sur une phase rouge, terminée ou expirée.

La copie des JSON complets suit fsync puis link exclusif depuis un fichier
provisoire ; le nom visible n'est jamais celui d'un fichier encore en écriture.
Les binaires et inventaires sont transférés vers des fichiers provisoires,
rehashés et seulement publiés après nouveau contrôle de la phase active.
Le ledgerbuild est lu complet et copié avec le nouvel ELF AVANT --list ; le
ledgerruntime et les deux inventaires sont copiés et vérifiés AVANT test.
Un ACK reste un fait de copie/hash lié à phase,source,preparedSHA,ledgerSHA,ELFSHA,
hôte et identité du coordinateur ; pas une autorité pouvant relancer une phase.

## Horloges, bornes et arrêt

Les horloges absolues des deux postes ne sont jamais comparées. Le gate PC
calcule remaining=MIN(ENDCommon−15,ENDtimeout)−monotonicPC après ses checks. Le
coordinateur transforme cette DURÉE en borne locale conservative à partir du
début local de la requête, pas de sa fin ; chaque appel SSH/export/link consomme
cette borne et sa propre limite (5s contrôle,30s transfert proposés).

La phase sous vrai poste garde1798+2/Common1750 et le budget propre du futur GO.
L'attente du poste est distincte : plafond local proposé300s, à autoriser avec le
lot correctif, sans le compter comme du temps natif. Le coordinateur demande
l'état pendant cette attente, sans lire une ancienne autorité comme nouvelle.
Une fois active, il ne peut rallonger sa borne locale. Sa fin doit arriver dans
la borne externe de phase, avec marge d'arrêt. La requête d'état finit par
signaler la phase terminée, jamais une nouvelle opportunité de publication.

Sur SIG, première erreur brute, timeout ou fin prématurée du SSH de phase :
refuser toute nouvelle copie/ACK, écrire le premier défaut immuable, arrêter et
récolter uniquement ses enfants SSH attribués. Si la phase est encore active et
le délai le permet, publier un refus distant borné ; le pilote l'observe dans
ses boucles et Common assure sa fermeture. Aucune recherche/tuerie par nom,
aucune recréation d'autorité. Les stdout/stderr de chaque appel et l'argv exact
sont conservés ; les échecs ne sont pas requalifiés en attente.

## Témoin factice du moteur réel à qualifier plus tard

Les tests importent Relay du futur relay.py et remplacent exclusivement le
transport, l'horloge et les pauses. Ils exigent disponibilité avant startphase,
ordre build→ACKbuild→runtime→ACKruntime, pas de publication avant hash, durée
locale dérivée d'une duréePC, latence de requête soustraite, refus avant toute
copie/ACK après expiration ou rouge, arrêt au premier transport défaillant,
SIG/finphase et récolte des enfants. Aucun SSH, /proc réel ou ELF exécuté.
Leur publication n'autorise pas à les jouer. Un budget CPU correctif distinct
et une RO totale du couple/pins seront demandés sur les sources publiées.

## CARRY de la référence : proposition séparée, pas un mode implicite

L'ELF0c565ace3bf6bd0d972447466df2842610a9d76a6f59c5bc2928599bd7f8b63b
compilé sous fec351 peut éviter un rebuild uniquement sur GO le nommant :
compiler-artifact JSON unique lié à sonde source/features/profil, sourcesfec et
parserbd inchangés, SHAsonde/Cargo.lock/Toml/scripts/SDK/corpus/profil égaux,
copies binaires horsT sur les deux hôtes, ancienCommonTERMINE/nettoyé et
propriétaireBASE vérifiés avant entrée puis après poste. Pas de transfert de
fingerprints ni adoption fictive ; nouvelle phase/nouveau ledger/deux nouveaux
ACK AVANT toute invocation. P02 reste un refus, aucune ancienne autorité valide.
La revue doit épingler le manifeste de carry et ses preuves ; le candidat aura
ensuite son nouvel ELF compilé sous son propre GO conditionnel. Cette première
préparation du relais ne branche pas automatiquement ce carry.

## Paquet implanté pour relecture, non exécuté

Le couple relay.py/gate.py est raccordé au pilote existant : nouveau prepare
copie les dix contrôleurs dans Q ; tool_source est indépendant du candidat
c1cd. --prior-root permet de figer la fermeture rougeP02 et le propriétaire
BASE réel, sans réarmer cette archive. Les noms finaux JSON/ledgers du pilote
sont atomiques, les identités superviseur/pilote sont inscrites et le premier
refus distant est observé pendant compilation, attente et test. L'ancienne
commande manuelle peer-ack a été supprimée de cette version du pilote.

Le transport ouvre les logs avant chaque SSH, désactive les connexions SSH
partagées, conserve argv/stdout/stderr/rc et n'arrête que ses propres Popen.
Les contrôles distants sont aussi enveloppés par le vrai timeout PC
(signalTERM/kill1s), avec5s maximum pour les métadonnées et30s pour les données.
Le coordinateur porte une borne totale2100s =300s attente poste +1800s phase,
et réduit chaque copie au temps restantPC reçu avant le RPC. Ce ne sont pas
1800s natifs renouvelables par publication. La phase distante garde les bornes
post/Common déjà relues. Le refus du coordinateur reste une preuve distincte,
jamais une réécriture d'un résultat natif déjà fermé.

Les tests publiés ont été complétés : données de ledger réellement sérialisées,
hash de copie et d'inventaires, refus au retour d'une copie si PCrouge, absence
de lien après fsync si garde refuse, Gate.active réel avec faux /proc/scope,
PID réutilisé, source changée, autorité terminée/verdict final/PCexpiré, et
Gate.acknowledge réel qui recontrôle l'état après fsync avant link.
Aucun de ces tests n'a encore été exécuté ; il ne s'agit pas d'une preuve verte.

carry-proposal.json fige les pièces de provenance de0c565 (JSONcompiler-artifact,
source/profil/scripts/SDK/corpus, SHA des preuves P02 et ownerBASE requis).
Le statut reste proposal_only_not_executable ; aucun modecarry n'est branché.
Il faudra une décision explicite et une revue du chemin carry avant tout GO
qui s'appuierait sur l'absence de Cargo.

## Relecture et budget correctif proposés

Demande : RO TOTALE du couple, du delta pilote depuisc1cd et du protocole
horloges/arrêt/deux liensACK ; pas seulement une revue du moteur générique.
Après publication des SHA, proposition d'UN lot CPU correctif **distinct** :

```sh
timeout --signal=TERM --kill-after=2s 58s python3 -B docs/codeparsers/17-11-2026/rust-probe-relay/test_relay.py -f -v
```

L'option-f active failfast du vrai unittest.main(). Les témoins remplacent
transport/process_records/horloge, sans SSH, /proc réel, Cargo, ELF natif ou
autorité réelle. Aucun GO n'est déduit de cette proposition. Première erreur,
échec ou timeout : STOP et preuves, aucun retry implicite. Les11anciens tests
CPU ne sont pas rejoués par cette proposition. Aucun budget natif n'est demandé
sur un paquet non qualifié ; la nouvelle paire et les pins seront relus avant
un éventuel GO futur de phase. P02 ne reçoit aucun ACK.
