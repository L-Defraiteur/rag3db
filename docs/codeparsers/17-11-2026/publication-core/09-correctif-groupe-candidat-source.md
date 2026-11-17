# Correctif groupé : candidat SOURCE, aucun vert ni admission physique

Les dix rouges8792 puis le rouge causal RPCf3d sont conservés deux hôtes AVANT
ce diff (rapports07/08). CA1C garde son FAIL de montage historique. Le ticket
ledger portait les réserves et critères avant correction. Cette branche neuve
ne modifie ni leurs sources ni leurs archives. Aucun import/parser/test du
candidat, SSH de phase, Common réel, natif, acteur, target ou corpus joué.

## Origine temporelle et deux watchdogs

Le tick de naissance reste une identité PID. Plus aucune conversion
procstart/CLK_TCK en échéance dans le moteur ou le pilote. `make_window` scelle
CLOCK_MONOTONIC/hôte/start/soft_end/kill_end, prepared/phase/rôle/source et identité
réelle de l'émetteur AVANT son exec en timeout. Après exec, la même identité
PID/start/UID relie l'origine au timeout réellement observé ; argv/exe/cgroup
et source/pin sont vérifiés séparément. Aucune identité physique future n'est
annoncée comme observée avant le handoff.

- LOCAL : relay_main émet peer-origin avant son exec du timeout local.
- PC : la commande SSH entre par Gate.arm, émet whole-origin puis exec le vrai
  timeout, qui lance Gate.launch. Ready passe par launch-input exclusif.
- Après poste : phase-arm du pilote émet phase-origin puis exec le timeout de
  phase. Compose lie sa vraie observation à cet original ; Common vient après.

Les octets d'origine sont liés par SHA aux rendez-vous bootstrap et outer.
La file finit à originalStart+queue, jamais maintenant+queue au rendez-vous
suivant. MIN(wholeSoftEND,phaseSoftEND)−maintenant−marge borne la demande entière
Common et son maximum déclaré. Le Gate refuse un Common qui dépasserait ces
bornes : pas de clamp silencieux d'un END déjà trop grand. KILL ne donne pas
de temps de travail. WindowEvent garde owner vivant, ENDop original et one-shot.

Pour un profil à un seul watchdog, une origine whole peut porter aussi la borne
de phase UNIQUEMENT si PID/start/UID/exe/argv/CG et limites préautorisées désignent
exactement le même objet. Cela permet la composition du futur profil global,
sans supposer un deuxième processus ni réémettre une nouvelle origine. Ce
raccord global réel n'est pas livré/qualifié ici ; six rôles et Q restent HOLD.

## Fermeture AVANT import, puis exécution des mêmes octets

La primitive nouvelle `scripts/publication_loader.py` unifie les trois entrées
Rust relay/Gate/pilot et les chargements de contrôleurs par le noyau. Elle porte
la frontière sémantique octets approuvés → module exécuté, sans boucle métier.
Coût : un contrôleur de plus à copier/épingler et un petit prélude dans chacune
des trois racines. Le SHA LITTÉRAL du loader figure dans ces racines ; le prélude
lit ses octets une fois, vérifie ce SHA puis compile/exécute CES octets.

En Q, --pin indépendant est obligatoire avant import du moteur/legacy/guard.
Le loader vérifie prepared, rôle de l'entrée, chemins canoniques/UID et fermeture
des contrôleurs ; il conserve leurs octets vérifiés. load_module compile ces
snapshots, pas une réouverture du chemin après empreinte. Les imports ultérieurs
demandés par le noyau portent également le SHA attendu. Les caches de moteur Q
sont confrontés à l'empreinte de la version chargée.

La racine de confiance reste EXPLICITE : entrée/interpréteur de bootstrap et
pin du prepared viennent de l'admission externe revue, pas du même fichier
non vérifié qui choisirait sa propre autorité. Le bootstrap n'est pas une sandbox
contre du code Python arbitraire déjà exécuté dans le processus. Les imports
SOURCE des chemins de dépôt revus servent à préparer/qualifier les outils ;
ils n'accordent aucune phase en source. Les entrées runtime exigent Q et le
prepared/pin, avec leurs contrôles de rôle ; pas de mode fake/skip du moteur.
Les anciens Q sans loader/origines ne deviennent pas compatibles par défaut.

L'événement Window consigne toute la fermeture déclarée des contrôleurs, loader
compris ; les hashes source ne sont toujours pas une attestation SDK dynamique
ni une preuve des images ELF exécutées. Ce dernier producteur appartient à Archi.

## RPC, récolte et clôture finale

Un appel possède un END local capturé AVANT les contrôles. Une réserve est
retirée du temps de communicate ; le timeout distant répartit soft/KILL DANS son
cap, et les TERM/wait/KILL/wait locaux ne reçoivent jamais de grâces hors END.
stop prend l'échéance originale, et les enfants/logs sont clos avant le verdict.
Une absence non observée reste UNKNOWN. Les preuves finales d'annulation sont
encore relevées même sans tour de boucle ; une observation tardive ne qualifie
pas une clôture positive.

La même exception initiale du RPC est réémise ; les erreurs de récolte/fermeture
restent secondaires dans result.cleanup, avec remote_quiet=UNKNOWN quand non
attesté. Un échec d'écriture du constat ne remplace pas non plus l'objet initial.
Après finish/stop, le résultat du relais utilise MIN(phase_deadline,hard_end),
y compris après fsync et close. Le verdict natif n'est pas réécrit pour le relais.

## Fixtures portées, assertions conservées

Les fixtures portent le loader réel, les pins explicites et les enveloppes
modélisées cohérentes. Leurs timestamps et argv sont des DONNÉES, pas un backend
ou une preuve OS. Le pin Gate reste le vrai pin corrigé8792. Les octets adverses
sont refusés à la vraie frontière d'import ; aucune suppression d'assert SHA.

Le témoin historique du préfixe timeout SSH suit maintenant la véritable
frontière arm→exec : son assert remote[:4] est conservé, avec assertion de l'entrée
arm et de l'origine scellée avant exec. Execve ET dup2 y sont simulés. La fixture
stop accepte deadline ; les ticks du cas d'attente expirée restent identités,
son horloge est9000→9004 (même durée qu'avant). Assertions des36/3/8/10/1 gardées.

`test_publication_candidate.py` propose une sélection exacte **58 cas** :
36 hérités +3 plans +8 Window +10 gardes +1 causalité RPC. Aucun de ces cas n'est
joué sur ce candidat. Pins source/loader/tests/interpréteur/cwd dans
`publication-correction-pins.json`. Revue TOTALE M/TTY/C7 avant nouveau GO CPU
vert distinct. La sélection elle-même sera vérifiée par unittest, pas annoncée
comme un résultat depuis le seul comptage source. Aucun GO natif/global Q/LIVE
ne découle d'un futur vert de ces fixtures.
