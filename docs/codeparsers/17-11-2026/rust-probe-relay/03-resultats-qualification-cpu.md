# Relais D45 : trente-six témoins CPU verts, aucun run SSH ou natif qualifié

Le lot **C7-PARSERS-RELAY-GREEN-D45-17nov60** est passé sur le commit exact
**d45aefbdf8a0cd4c1f2fc0a6547cc450215aae40**, propre et publié :36méthodes
exécutées,0FAIL,0ERROR,0skip,rc0. Une invocation seulement,0,187111s mesuré
extérieurement,0,095s selon unittest. La RO totale Banc deP05 était favorable
**en préparation** ; ce lot qualifie les témoins CPU, pas le raccord Linux réel.

Aucun SSH,/proc réel,signal réel,Cargo,natif,autorité,target ou corpus n'a été
appelé par les fixtures. Les copies conservatoires après le lot ne sont pas
un rejeu ni une qualification du transport. P02-P04 restent archives consommées
ou prototypes non armés ; P05 n'a reçu aucun GO de phase. REGLES reste7 et les
tickets de sonde et de relais restent ouverts.

## Commande, source et preuves

Depuis le dossier publié
`docs/codeparsers/17-11-2026/rust-probe-relay/` du worktree privé :

```sh
timeout --signal=TERM --kill-after=2s 58s python3 -B test_relay.py -f -v
```

Source,argv,cwd,allocation et started ont été écrits avant l'appel ; stdout et
stderr ouverts avant spawn, rc et elapsed enregistrés ensuite. Les36noms et OK
sont dans stderr brut ; stdout est vide. Aucun échec masqué ni skip compté vert.

Répertoire identique aux deux hôtes :
`~/.cache/codeparsers-notes/rust-relay-green-d45aefbdf8/`.
Fichiers copiés puis SHA comparés, tous concordants :

| Fichier | SHA256 |
|---|---|
| manifest.json | d8270ccf4d7ddb9ac1a425dbee6da5fc673e8928ea95e16bac27631df6688e0f |
| command.json | 412512e49ffd29252cd06f4d739672a3dfac6378a456750d17230391f1371431 |
| stdout | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| stderr | 290a445b64a9141fbbf28576a356a9428f012ca4dfffa300941d212ca19bf372 |
| result.json | 7fa12e9a1361280e20cc9043f966cf034cacb23dbc186b164b00474ed435e47d |

Le présent rapport est publié sur une branche documentaire distincte,
**cx-codeparsers-rust-relay-report**, issue deD45. La branche de l'outillage reste
épingléeD45, propre aux deux hôtes ; aucune modification du candidat ou de la
référence n'est nécessaire pour consigner ce résultat.

## Rouge avant correction, sans effacer l'histoire du harnais

Le paquet782b avait20témoins verts puis la RO totale1cdd avait révélé trois
réserves de vrais raccords. Les nouveaux témoins ont été prépubliésdda9 avant
correctif : deuxFAIL attendus (TTL launch absent et abortqueued impossible),
puis ERROR de fixturebootstrap tool_root absent. STOP, casfinish non joué.
Cette erreur de fixture reste conservée, pas transformée en défaut produit.
La fixture seule a été corrigée57fac avant GO complémentaire ; les deux cas
restants ont ensuite été vus FAIL (futureargv avant exec ; retour0 finishlate).
Manifest complémentaire2hôtes :
62abd2eb60ce11324055510d0df7386c81f2a43725937efffe9e42c53e963699.

Après correctif groupécf832, la RO totale64d7 a trouvé deux restes : cap expiré
avant abort empêchant la RPC, et bouclecancel non entrée inventant absent.
Ticket et trois témoins prépubliés598f ; GO rouge distinct, troisFAIL attendus,
0ERROR/PASSinattendu avant correctionD45. Les quatre anciens rouges non rejoués.
Manifest2hôtes :90fbc0af8a6c3fbd78c6cd9ba000f0397af97944f16ec4c452f532ab1ecb9546.
Rapport antérieur02 et ticket conservent les SHA complets et les chronologies.

## Ce que les trente-six témoins couvrent

Le moteur réel Relay et la porte réelle Gate sont importés ; transport, horloges,
identités /proc et pidfd/signal sont factices. La suite couvre l'ordre ready
avant spawn et les deux publications, contenu/hash/fsync/lien exclusif, identité
source/PID-start/UID, transition exec bornée, ancien scope refusé, copie altérée,
refus après copie, premier défaut et absence de retry.

Elle couvre aussi les vrais constructeurs/entrées CLI avec exec intercepté,
watchdog distant dans la commande SSH, annulation queued sans autorité native,
récolte des rôles attribués, délai épuisé avant la première boucle, UNKNOWN
protégé, phases déjà closes, latence RPC sans extension, fermeture/fsync tardifs
et verdict du relais distinct du natif fermé.

La réserve5s est DANS file300,phaseEND et hardEND parent2098 ; admission queued
locale jusqu'à295 et finish garde5. cleanup_cap ne dépend pas du cap opératoire
expiré. Si ce reliquat est épuisé, remote_quiet=UNKNOWN est explicite. Le relevé
final Gate n'est jamais conditionné par l'entrée dans la boucle de temps et
launcher_absent exige0vivant/0UNKNOWN plus scopequiet siCommon existe.

Ces fixtures ne démontrent ni l'armement réel du coordinateur distant, ni un
transfert SSH réel, ni l'effet de pidfd/signaux sur Linux, ni la compilation ou
l'exécution de la sonde. La RO favorable ne supprime pas ces limites. Le pilote
reste spécifique à deux stagesRust ; aucune capacité quatre publicationsD6
n'est annoncée comme livrée ou admise par ce résultat.

## P05 exacte conservée pour une admission future distincte

Préparation neuve, identique aux deux hôtes, jamais armée :
`~/.cache/codeparsers-notes/rust-probe-relay-pair-05/`.

- prepared.json :20d599625d1e1f040bd97dcc6fe89e547c178f6b4f6f7f8c0dd6631ec9c4cf4d.
- runtime-profile.json :81969f85c1cc6384ef67283fb6486cff22bbbcc8c5a795026339b7dbb13c4192.
- Dix contrôleursQ figés, toolD45, BASEfec35198b64b7e9da0496de6b01a7232fa7a8390,
  CANDc1cd48750f8178e68d24a3570bc320b458cad7d1, sources privées propres.
- Paramètres relais300/30/5/reserve5, hardphysique2098+kill2 sur les deux hôtes,
  poste32GiB/high=max/swap0/quota5CPU et native1798+2/Common1750 proposés.
- PriorEND P02 rouge/nettoyé et propriétaire réelBASE conservés. Target portable
  non touché ; nouvel ELF0c565 deP02 toujours conservé deuxhôtes et jamais joué.

Choix annoncé avant nouveau pin/RO : nouveau Cargo --no-run JSON sur FECinchangée,
cache autorisé, aucun rebuild forcé ; carrycustom reste non branché. Ce choix
et ces pins ne constituent pas un GO. Toute phase native ou calibration pour
un autre relais demande une revue de son raccord exact et une allocation
séparée. Aucun service/provider/PG/GPU/modèle ni pushmaster dans ce lot.
