# Cycle 3c2 : refus avant armement effectif du producteur

Résultat : **ROUGE, rc1, avant grant, santé ou writer**. Le GO distinct
C7-LIFECYCLE-3C2-17nov900 est consommé ; aucune reprise, autre allocation,
prolongation ou correction exécutée n'en découle. Source native exacte
3c2a93ca38ba5729b4414a5d813467f916cbf905 ; Qv4 prepared
69c5480da4bc212f4e435a4419c0a56ebb3b26b041e3ca7ade47dd37336aa68f.
Les deux RO totales ED/Recherche et les 84 fonctions CPU vertes restent des
preuves bornées, sans qualification native du cycle par transfert.

## Exécution autorisée et frontière atteinte

Le contrôleur a appelé sur PC `lifecycle.py --launch
3c2a93ca38ba5729b4414a5d813467f916cbf905 --allocation
C7-LIFECYCLE-3C2-17nov900`, après notification Embar avant lancement. Le vrai
poste puis le scope validation-qeenc8pd ont été acquis : memory.max32GiB,
high=max, swap0, cpu.max500000/100000. ENDcommon1247698.195278915, delai830 ;
STARTcommon1246868.195278915 est dérivé END−830, pas un timestamp POSTE inventé.
Le watchdog physique898+kill2 et la réserve RO500 restent ceux de la recette.

La copie FIN neuve v4 passe les précontrôles : clone conforme, dette0 avant
writer, promesse/pending FTS vides, cinq jeux d'IDs et générations inchangés,
digest typé des 97247 lignes vectorielles. Huit lectures SQL rc0 avec BUILD_ID
59ac réel sont présentes. Les décodages FTS readonly portent File6313/gen14,
Library398/gen9, Scope65913/gen18, ScopeContent61223/gen52 et Symbol73373/gen12.
Le digest avant a SHA86ce25e2445f9dc412d83ea84a0a1fab37e9aa9a7db70603b047010635f7d9dc.
Cela ne qualifie aucun contrôle POSTwriter : le writer n'a pas été lancé.

## Chronologie PC conservée

READY nonce a93c7255b4d945148b6d91c89be3fb51, SHA
946e93c74f2750e5e4b6e6d9f84d01c205d6851e65767e5175be4f8a68b475b4.

| Fait | Monotonic PC |
| --- | ---: |
| Observation READY avant publication/fsync | 1246927.830043659 |
| Dernière entrée owner permise, marge30 | 1247017.830043659 |
| Armement déclaré dans armed.json après imports | 1247046.19743902 |
| Refus owner REFUSED_NO_RETRY | 1247046.199404131 |
| Borne absolue de remise du grant | 1247047.830043659 |
| Premier rouge du pilote | 1247047.917505952 |
| Scope TERMINE/ROUGE observé | 1247065.858174662 |
| Bus absent et owned_alive[] vérifiés | 1247135.310606105 |

Le watcher PID2000831 est armé28.367395361s après la dernière entrée permise.
Son assertion de marge refuse avant `publish_observed_grant` et avant GET.
Son message « veille active » était erroné : Embar corrige explicitement,
poll=1 ayant été observé. Le moment Popen avant imports n'a pas été capturé ;
il n'est pas reconstitué. Aucune causalité réseau/latence de message n'est
inventée. Le contrôle de borne a fonctionné ; la disponibilité annoncée
n'était pas une preuve d'armement préalable.

Le pilote refuse pour `fresh lifecycle provider coordination deadline expired`.
Les fichiers grant, provider-before.transport, provider-before et writer-process
sont absents. Le helper owner n'est pas appelé ; aucune santé dédiée des deux
acteurs ni CLI writer, publication réponse ou shutdown n'a lieu. Cela n'atteste
pas le nombre de RPC éventuels d'autres acteurs sur le service.

## Après arrêt, sans réouverture de base

Scope ROUGE1/restes[]/nettoyétrue, autorité terminée, bus absent, enfants owned
absents. Le PID du watcher est également absent lors de la collecte. Les
empreintes des sources privées COPY02/COPYA/audit/FIN et du nouveau clone sont
identiques ; modes/répertoires inchangés, aucun WAL nouveau ou modifié. Garde
source/CLI/SDK/outils repassée en lecture de fichiers, pas d'ouverture DB.

`post-red-invariants.json` à1247244.936834634 conserve les deux scans : found[]
pour clone et FIN, chacun avec11 UNKNOWN globaux non attribués. Ces inconnus
restent des inconnus ; aucune absence GPU ou innocence d'un PID protégé n'en
est déduite. Aucun nettoyage de source, réparation, recovery, LOAD, Index,
Cargo, PG, service7970 ou original n'est effectué. La copie et P/Q/S restent
conservés. Les anciens COPYA, audit et B5 demeurent rouges.

## Archive et contrôle personnel

PC : `~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-v4-red-01`.
Copie locale : même racine, `lifecycle-v4-red-pc-local-01`.
84 fichiers manifestés,16738573 octets ;85 fichiers avec le manifeste lui-même.
Tous les84 fichiers réempreintés personnellement après copie : tailles/SHA
conformes,0écart, aucun fichier supplémentaire. Manifeste byte-identique joint :
[lifecycle-v4-red-proof-manifest.json](lifecycle-v4-red-proof-manifest.json), SHA
8087ab5db59b8f48bdfb98579f0743acecb1d7831197781a981c0fde814e55de.

L'archive contient les sorties readonly du pilote, les19 sources du paquet,
allocation et log, les pièces scope et les traces owner. L'autorité est une
copie **expurgée** explicitement marquée, sans jeton privé. Les DB/FTS conservés
dans P et le bytecode dérivé ne sont pas copiés dans l'archive. Le manifeste
owner initial06e5980bd9273aa35b50e73f26acabfe35819a7825f56cefbeb239b9f968fcd5
et ses11 fichiers sont contrôlés avant collecte. watch.py SHA
2bca2b34c0d1e6be0ae8adbaf843fef1f6b8e12352dbc6642bea187d8bab9ede
est conservé ; aucun ledger SHA prélaunch de cet acteur n'est revendiqué.

## Suite autorisée : préparation uniquement

[TICKET C7 avant correctif](../../orchestration/tickets/2026-11-17-le-cycle-demarre-avant-l-armement-effectif-du-producteur-de-grant.md),
lié au [ticket READY](../../orchestration/tickets/2026-11-16-un-grant-est-publie-apres-la-pause-ready-faute-de-borne-explicite.md).
C7 autorise une préparation coordonnée Arbre/Embar : un contrôleur PC lance et
vérifie un watcher réellement exec/armé avant DB et READY, puis supervise sa
liveness et son refus sans dépendre d'un ping humain. Même watchdog physique,
END original de la future allocation seulement, premier rouge immuable,
aucun health avant READY valide. Vrais raccords CPU rouges avant fix puis RO
couple/pin et GO distinct seront nécessaires. Aucun code correctif, runtime,
créneau ou nouveau GO n'est publié/exécuté par ce rapport documentaire.
Cycle, shutdown, durabilité générale et migration LIVE restent non qualifiés.
