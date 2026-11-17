# Contrat proposé : invocation avec image d'entrée confirmée

SOURCE/API/TÉMOINS PRÉALABLES seulement. Aucun module opérationnel, test joué,
proc/signal, prototype OS, build, native ou appelant modifié. Ticket commun
[images exécutées](../../../orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md),
arbitrage C7 **93c44be2d6**, étude45 **a5281ec4e4**. Cette proposition ne ferme pas
le ticket. RO du protocole puis autorisation CPU distincte avant exécution.

## Sens et frontière

Une invocation d'un programme **déclaré et revu** possède une image ELF d'entrée
confirmée, une copie conservée, un contexte, des sorties et un wait réel. Le
résultat appartient à l'INVOCATION, même si le programme crée un enfant ou se
remplace ensuite. Il n'affirme ni image A inaltérée, ni PID écrivain de chaque
octet, ni preuve contre un programme arbitraire inventant sa sortie. Le suivi
ultérieur reste UNKNOWN. Aucun actual B ne modifie le catalogue attendu.

Le contrat consommateur v2 TTY **44ae9af3bcb697054467d472d8581e1117764828**,
note13 et table P01–P20, a été lu : portée alignée à cet arbitrage, sans
qualification physique.

Le module futur partagé proposé est `extension/rag3weaver/scripts/execution_entry.py`.
Réservation reçue par Recherche, absence de chevauchement d’écriture déclarée ;
pas un fichier opérationnel créé. Les quatre
sites e2e et Cargo runner sont seulement la surface transitive acceptée à préparer.
Le SHA2150 ne sera pas intitulé parcours inchangé. TTY publiera son contrat borné ;
P17–P20 historiques ne sont pas comptés satisfaits ou verts. CP conserve seul
publication/ACK/Window. Aucun relais parallèle ni règle de langue dans ce module.

## Protocole opérationnel proposé pour RO, NON implémenté

Profil initial : ELF régulier local non privilégié, stockage dont l'exclusion
d'écriture pendant l'usage exécutable est démontrée. Scripts, stockage distant,
mutation par mécanisme non couvert, fd/path/identité non accessibles : REFUS.
La compatibilité du profil est à qualifier, pas un fait acquis sur l'hôte.

1. Le parent runner appartient au Common et à son autorité/END déjà accordés.
   Il valide source/phase/allocation/nonce/manifeste/catalogue/features/argv/cwd et allowlist d'environnement
   attendus AVANT lancement. La phase déclare notamment LD effectif attendu
   après transformation Cargo ; aucune valeur effective ne devient attendue.
   Les sinks sont exclusifs à l'invocation ; stdout/stderr conservent leurs
   bytes et leur séparation, sans tampon texte réinterprété ni sortie de traceur.
2. Le runner ouvre l'objet sélectionné sans rouvrir ensuite son chemin ; garde
   identité de fichier et hash attendu. Le chemin peut être remplacé : seule la
   comparaison de l'objet effectivement entré peut autoriser. Le FD ne garantit
   pas à lui seul la stabilité du contenu.
3. Un enfant de démarrage contrôlé, **monothread avant exec**, demande TRACEME
   et s'arrête. Son parent naturel runner reste propriétaire unique du wait.
   Le parent vérifie enfant/start/UID/CG/parentage et configure TRACEEXEC (et
   protection de mort du traceur à qualifier), puis permet uniquement le
   démarrage prévu vers l'objet FD. Aucun fork/clone/syscall tracing universel.
   Le bootstrap ne crée pas d'autre enfant et n'exécute pas d'autre programme.
4. Seul un statut de wait décodé **événement EXEC réussi du bon enfant** ouvre
   la confirmation. Pas un SIGTRAP quelconque, SIGSTOP, EOF, intention fexecve,
   sortie du runner ou code0. Échec d'exec, mort/disparition, événement inattendu,
   droits refusés, identité changée ou END atteint : première cause conservée,
   aucun événement positif inventé, arrêt/récolte attribués par Common.
5. L'enfant est encore arrêté avant le userland de l'ELF. Le parent ouvre son
   **exécutable réel** à cette barrière, vérifie identité/start/CG avant et après,
   compare l'objet au FD choisi et copie/hash depuis ce FD réel, pas depuis T.
   L'objet courant doit être stable sous le profil déclaré durant toute cette
   confirmation/copie. La copie est finalisée puis rehashée et liée au catalogue
   attendu avant libération ; mismatch/refus/timeout empêchent le corps du test.
   Un changement des bytes avant l'entrée sera donc refusé, même même inode.
   Deux hashes seuls ne démontrent pas l'absence d'ABA : cette propriété dépend
   du profil de stabilité à qualifier, pas de l'échantillonnage.
6. Le parent garde les faits de confirmation et copie. Le point de publication
   CP/ACK requis par le plan précède la libération ; pas de boucle CP réécrite.
   Le detach réussi libère avant corps et est conservé comme fait opérationnel.
   Il ne prouve pas une observation TracerPid0 postérieure : un programme bref
   peut déjà être fini. Échec de detach ou attente de publication dépassement :
   REFUS et cleanup sous les mêmes bornes. Aucun second GDB simultané présumé.
7. Après libération, le runner attend son VRAI enfant naturel. Un stop n'est pas
   une terminaison ; distinguer sortie normale et signal, sans transformer une
   sortie par signal en rc0 ni perdre sa nature dans un shell. Le wait et les
   sinks hashés se rapportent à l'invocation, pas à une image restée A.
8. Après le wait enfant, le runner préserve/finalise RAW et faits de
   l’invocation puis retourne. Il ne déclare PAS son ancêtre Common quiet. Un
   propriétaire extérieur préadmis attend le runner réel, puis atteste la
   clôture Common (quiet/reap du sous-arbre attribué, excluant le contrôleur
   extérieur) et sa publication. Toutes ces étapes respectent END initial et réserve. Mort du runner n'est
   pas terminée proprement par déduction ; Common conserve cause et collecte.
   Aucun reçu positif si cleanup/reap inconnu, pas de remise à zéro ni retry TTL.

Le point 5 confirme l'entrée même si la sélection précédant exec était mutable :
la stabilité du fichier réellement exécuté pendant la barrière reste un prérequis
à démontrer sur le profil admis. Sans cette garantie : STOP, pas fallback vers
hash du chemin après disparition. Aucun memfd/relocation silencieux proposé.
Le chargeur dynamique et SDK réel nécessitent une attestation séparée au bon
PID/point, sans inventer des libs chargées dès le stop exec.

## Réemploi de sources, pas de preuves ou délais historiques

Lecture source seulement de `garde-target-union/superviser-elf.py`,
`capture-stable.py` et génération de commande de démarrage dans `driver.py`
(état présent dans cette branche) : séparation parent naturel/wait et capture
exec/main/libération sont des précédents de frontière. Leurs chemins, flags,
budgets et reçus ne sont ni importés ni rejoués ; leur callback GDB ne devient
pas un producteur générique. Le nouveau bootstrap/TRACEME reste à implémenter
et qualifier après revue. Common6439 ne réclame pas artificiellement TracerPid0.

Références primaires : [ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html)
pour parent traceur/événement exec/détachement ;
[fexecve](https://man7.org/linux/man-pages/man3/fexecve.3.html) pour objet FD et
limite de mutation ; [execve](https://man7.org/linux/man-pages/man2/execve.2.html)
pour remplacement et ETXTBSY ;
[fs/exec.c v6.18](https://github.com/torvalds/linux/blob/v6.18/fs/exec.c) et
[kernel/fork.c v6.18](https://github.com/torvalds/linux/blob/v6.18/kernel/fork.c)
pour modèle d'installation du fichier exécutable/exclusion d'écriture. Référence
source Linux explicite, pas noyau hôte relevé ni qualification du stockage.

## Contrat pur et témoins prépubliés

`execution_entry.pyi` décrit seulement `verify_transcript` : futur vérificateur
pur d'une séquence de faits modélisés. `test_execution_entry.py` vise ce symbole
futur ; aucun module d'implémentation n'est fourni ici et aucun import/test n'est
exécuté. Quarante fonctions de témoins sont écrites, NON JOUÉES. Ces témoins
n'attestent jamais le noyau ou les événements réels. Le
producteur opérationnel devra construire les faits depuis ses propres primitives,
pas accepter un transcript fourni par le programme testé comme preuve d'OS.

Séquence modèle obligatoire : `selected`, `spawned`, `exec_stop`, `copied`,
`published`, `released`, `waited`, `finalized`, `runner_waited`, `common_closed`. IDs invocation/clock stables,
séquences strictement croissantes/temps non décroissants et finis. Toute phase
jusqu'au wait doit rester < operation_end ; clôtures finale et externe < end, avec
operation_end <= end-cleanup_reserve. Champs manquants, types incorrects,
floats non finis et événements inattendus refusés. Événement `failure` conserve
le premier code ; des événements cleanup après ne l'effacent jamais.

| Fait | Données minimales attendues |
|---|---|
|selected|artifact/source/phase/allocation/nonce/manifest/catalogue/features/argv/cwd/environment/stdout/stderr/authority/profile identiques au plan ; objet dev/inode, hash choisi|
|spawned|runner PID/start distinct enfant PID/start ; PPID enfant=runner, UID/CG autorisés au plan de lancement lié|
|exec_stop|kind EXEC_SUCCESS (modèle seulement), enfant/start/CG/UID inchangés, objet réel dev/inode/SHA=objet choisi, stabilité qualifiée, argv/cwd/env attendus|
|copied|source_object=objet réel de exec_stop, copie complète, SHA avant/après=artifact ; aucune réouverture du chemin|
|published|Référence CP exacte verified independently : prepared/plan/phase/allocation/nonce/sequence/ledger SHA/entry facts SHA/ACK receipt SHA, invocation/copied SHA/catalogue ; pas booléen ACK|
|released|même enfant, detach réussi avant userland selon protocole, pas promesse d'absence de traceur future|
|waited|même enfant/start, parent runner, kind exit/signal, code exit>=0 ou signal>0 ; pas stop|
|finalized|RAW sinks exacts liés/hashés et clôture de l’invocation seulement, sans quiet Common anticipé|
|runner_waited|Vrai wait du runner par propriétaire extérieur préadmis, PID/start exacts, après finalized|
|common_closed|Propriétaire extérieur common_owner exact, autorité liée, quiet/reap/publication attestés après runner_waited sous end|

Les champs runner_pid/runner_start/uid/cgroup du plan sont fournis par admission
Common indépendante, non choisis par le transcript. Le PID enfant est lié au vrai
lancement par le producteur, pas par déclaration du test. La présente API de
revue est NON ADMISSIBLE en production.

## Portée et décisions restantes

| Exigence | Portée retenue |
|---|---|
|Contenu/catalogue/source/features/copie|Image d'entrée de chaque invocation déclarée ; B inconnu refusé|
|PID/start|Enfant réel et parent runner séparés ; pas identité des futurs exec|
|RAW/rc|Invocation du programme revu, pas PID écrivain par octet ni image immuable|
|P01–P16|Table de correspondance détaillée à publier avec le nouveau contrat TTY ; aucune coche verte par ce document|
|P17–P20|Couverture ultérieure UNKNOWN et non condition implicite du lot anglais selon C7 ; aucun point annoncé satisfait|
|SDK|NOT_ATTESTED par ce contrat ; attestation séparée au PID réel|
|Confinement/descendants|Arrêt et ressources Common ; aucun inventaire exhaustif d'images prétendu|

RO requise : protocole de stabilité/barrière, type d'identité Common/ACK et sinks,
propagation Cargo, puis surface des quatre appels. Coût capture/copie/hash,
latence startup, waits/cleanup, SDK et compatibilité stockage UNKNOWN. Aucun
budget d'exécution proposé comme acquis. Aucun appelant ni canon modifié ici.

## Delta après réserves CP377c94, avant implantation

Les réserves ont été consignées au ticket avant ce delta. Le booléen ACK est
remplacé par une référence CP exacte : prepared, plan, phase, allocation, nonce,
séquence, SHA ledger, SHA faits d'entrée, SHA reçu ACK. Le futur adaptateur CP
vérifiera cette référence avec les primitives de publication du propriétaire CP ;
le modèle pur compare une référence déjà vérifiée indépendamment du programme
et du transcript soumis. Ce n'est PAS une API CP inventée ou une vérification de
signature/lecture de reçu livrée. Une table d'adaptation au schéma CP réel reste
à relire avant implémentation ; aucune référence n'est fabriquée depuis actual.
La liaison SHA faits d'entrée doit porter sur leur représentation canonique
réelle choisie avec CP, pas sur une chaîne arbitraire du programme testé.

`finalized` est la clôture de l'invocation (enfant wait/RAW), pas celle de Common.
Le contrôleur extérieur Common attend ensuite le runner puis clôt le sous-arbre.
L'identité common_owner vient de l'admission et doit être extérieure à ce
sous-arbre ; la validation de modèle ne qualifie pas cette appartenance OS.
La fonction de revue pure consomme une séquence composite : son dernier segment
vient du contrôleur, jamais du runner qui attesterait sa propre disparition.
La production du reçu terminal ne doit pas démarrer de tâche dans le sous-arbre
qu'elle déclare quiet. Absence/refus de clôture externe reste rouge/incomplet.
Six témoins additionnels écrits, NON JOUÉS : ACK autre phase/autres faits/booléen,
closer runner interdit, clôture avant wait runner, clôture externe manquante.


## Delta après réserve TTY377 : fenêtre d'origine

Consignée au ticket avant correction. Le plan contient désormais `start` Common
original et `issued` d'émission de l'autorité indépendante, avec son identifiant
et même horloge. Contrôler tous les nombres finis et
start <= issued < operation_end <= end-cleanup_reserve < end, réserve positive ;
chaque fait doit être >= issued, jamais seulement inférieur à END. La phase
ne recrée ni start ni END depuis son propre premier événement. Les faits avant
émission sont refusés avant tout effet dépendant ; une émission hors fenêtre
n'accorde pas de nouveau budget. Trois témoins écrits, NON JOUÉS, portent ces
cas. Total40 fonctions ; aucune erreur d'import n'est une preuve rouge TDD.
