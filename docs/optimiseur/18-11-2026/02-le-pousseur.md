# Le pousseur : une file de lots déposés, poussée par trains

18 novembre 2026, session G (optimiseur). Idée de Lucie, forme de c7 et G. Oui de Lucie le 18 au soir.

## Ce qui change

Aujourd'hui, une session prête attend le « à toi » de c7, puis rejoue son lot sur la tête du moment sous
`poste push`, puis pousse. La série est voulue : chaque lot se rejoue sur ce qui est déjà sur master. La main
qui tient la file, elle, ne l'est pas. Le 18, c7 a tenu la file toute la soirée, et deux erreurs de main ont
coûté chacune un tour : un `PILE_AVANT` faux, un format de résumé.

Demain :

- la session **dépose** son lot au « prêt » et retourne à son travail ;
- le **pousseur** prend la file, repose, rejoue, pousse, et écrit « poussé » ou « rouge » ;
- la **session-pousseur** lit le journal du pousseur, prévient l'auteur et dit un rouge en clair.

Il n'y a plus de « à toi » pour le code : la file est l'« à toi ». c7 ordonne la file en éditant son fichier.

## Ce qui ne change pas

- Le lot se rejoue sur la tête du moment, rouge d'abord, et un lot sauté ou non joué est rouge.
- Un seul push à la fois sur master, sous `poste push`.
- Pas de force push, ni de branche distante effacée : une repose part sous un nom neuf.
- L'identité git est l'adresse personnelle de Lucie, et aucun commit ne mentionne d'assistant.
- Le bâti C++ reste hors du pousseur : il passe par la voie transférable ou par une fenêtre de G.
- Les trois contrôles après le push : status propre, aucun rebase en cours, HEAD = FETCH_HEAD.

## Où vit la file

Hors du dépôt, à luciepc, sous `~/.cache/rag3weaver-build/pusher/` : éditer la file ne doit jamais être un
commit de master. Les noms de fichiers et de champs sont en anglais (règle de Lucie du 16 novembre).

| fichier | rôle |
|---|---|
| `queue.tsv` | la file : une ligne par lot, dans l'ordre de passage ; c7 l'édite pour ordonner |
| `log.tsv` | le journal : une ligne par événement (pris, reposé, rejoué, poussé, rouge), jamais réécrit |
| `inbox/<author>.txt` | la boîte de l'auteur : ses lots poussés ou rouges, une ligne chacun |
| `paused` | présent : la file est en pause (la raison dans le fichier) |
| `worktree/`, `target/` | le worktree unique et son target chaud |

Une ligne de `queue.tsv` porte, séparés par des tabulations :

| champ | exemple | sens |
|---|---|---|
| `id` | `embed-parity-1` | nom unique du dépôt, choisi par l'auteur |
| `branch` | `embed-parity` | branche poussée sur origin par l'auteur |
| `head` | `4247483196` | tête testée par l'auteur ; le pousseur refuse une branche qui a bougé depuis |
| `kind` | `docs`, `scripts`, `rust`, `cpp` | ce que le lot touche ; le pousseur le recalcule et refuse un écart |
| `witness` | `e2e_code` ou `-` | filtre du témoin (`run_e2e.sh <filtre>`) |
| `witness_suite` | `e2e` ou `lib` | `SUITE_DU_TEMOIN` |
| `extra_suites` | `e2e_lectures,e2e_postgres` ou `-` | `SUITES_EN_PLUS` ; les suites exigées s'ajoutent par `required_suites.txt` |
| `parent` | `-` ou l'`id` d'un lot de la file | un lot écrit sur un autre ; il passe après lui, dans le même train |
| `author` | `embarquement` | nom de session, pour le message et la boîte |

**Déposer.** Par ssh, sous verrou de la file :
`ssh luciepc '~/.cache/rag3weaver-build/pusher.sh enqueue <id> <branch> <head> <kind> …'`. Ou par un message à la
session-pousseur, qui fait le même appel. `enqueue` refuse :

- un `id` déjà présent ;
- une branche absente d'origin, ou dont la tête n'est pas `head` ;
- un `kind` qui ne correspond pas au diff à trois points, ou un `parent` absent de la file ;
- un lot `cpp` (il passe hors du pousseur).

**Retirer.** `pusher.sh dequeue <id>` : l'auteur ou c7. Un rouge retire le lot de lui-même.

## Le pas du pousseur

Le pousseur est un script, `pusher.sh run`, en boucle sous `poste push pusher` à luciepc. Il fait un pas à la fois et
écrit chaque pas au journal, avant de le faire et après.

1. **Relire.** Il relit `queue.tsv` et `paused` avant chaque train, jamais pendant : une édition de c7 vaut pour
   le train suivant. Si `paused` existe, ou si master est tenu (`master-tenu`), il attend sans rien prendre.
2. **Prendre le train.** Il prend les lots en tête de file, dans l'ordre, jusqu'à `k` lots (paramètre, 4 au
   départ). Un lot `rust` est suspendu si la lib épinglée est plus vieille que le dernier commit du moteur sur master
   (voir plus bas) ; les lots `docs` et `scripts` passent.
3. **Reposer.** Dans le worktree unique, il part de `origin/master`, puis cherry-pick les commits de chaque lot
   (`origin/master...<head>`), dans l'ordre du train, sur une branche neuve `pusher/<date>-<n>`. Un conflit sort le
   lot du train : rouge « conflit » rendu à l'auteur. Le pousseur ne résout jamais rien à sa place.
4. **Rejouer.** Il rejoue une fois le train entier, selon le plus lourd des `kind` :
   - `docs` seuls : aucun rejeu (comme aujourd'hui) ;
   - `scripts` : le témoin de chaque script touché (`tools/poste/test_<nom>.sh`), puis `check_tickets.py` ;
   - `rust` : `passe_epinglee.sh` sous `poste lourd` et `POSTE_MEM_MAX=32G`, avec l'union des témoins, des suites
     en plus et des suites exigées, puis `check_features`, comme une session le fait aujourd'hui.
5. **Pousser.** Au vert, `push_garde.sh` sur ce worktree et ce journal, comme aujourd'hui, puis les trois
   contrôles. Les commits arrivent sur master tels que reposés : un par commit d'auteur, rien de fusionné.
6. **Écrire.** Une ligne au journal et une dans la boîte de chaque auteur : « poussé <id> <hash> » ou
   « rouge <id> <raison> <journal> ». La session-pousseur lit le journal et prévient l'auteur.
7. **Se recopier.** Si le train a changé `push_garde.sh`, `passe_epinglee.sh`, `poste`, `pusher.sh` ou leurs
   tables, le pousseur les recopie au cache des deux postes (`.avant-<id>`, `install`, sha256), joue leurs témoins
   contre la copie, puis repart. Un rouge ici met la file en pause et appelle c7.

## Les rouges

| rouge | ce qui se passe |
|---|---|
| **conflit** à la repose | le lot sort du train et de la file ; les autres continuent |
| **code** au rejeu (témoin, suite, check_features) | bissection du train : rejouer la première moitié, puis l'autre, jusqu'au lot fautif ; il sort de la file, les autres sont poussés |
| **environnement** (code 124, poste tenu, service occupé, réseau) | un seul rejeu du même train ; au second rouge, le train sort entier et c7 est appelé |
| **push** refusé (master a bougé) | nouvelle repose sur la tête du moment, comme une course perdue aujourd'hui |

La bissection coûte un rejeu par moitié. Les rouges de code sont rares (zéro le 18 au soir), donc le train est
presque toujours poussé d'un seul rejeu. Un rejeu de 40 minutes pour quatre lots Rust, au lieu de quatre rejeux
de 40 minutes, change l'ordre de grandeur.

Un rouge d'environnement se distingue d'un rouge de code par sa trace :

- le code de sortie 124 ;
- « poste tenu » dans le journal du poste ;
- un service qui refuse au point 1 ;
- un fetch qui échoue.

Tout le reste est un rouge de code. Dans le doute, c'est un rouge de code : l'auteur relit, et cela ne coûte
qu'un dépôt de plus.

## La lib plus vieille que master

Un lot C++ atterrit par la voie transférable sans bascule. La lib épinglée par `passe_epinglee.sh` est alors plus
vieille que master, et un lot Rust rejoué sans `RAG3WEAVER_MOTEUR_ANCIEN` se lierait à une lib qui n'a pas le
moteur de master.

Le pousseur compare le commit de la lib épinglée au dernier commit qui touche le moteur sur `origin/master`. S'il
est plus vieux :

- il **suspend** les lots `rust`, qui restent dans la file sans en sortir ;
- il laisse passer les lots `docs` et `scripts` ;
- il écrit « lib en retard » au journal, et la session-pousseur le dit à c7.

La bascule reste une fenêtre de G. `RAG3WEAVER_MOTEUR_ANCIEN` reste l'exception, posée à la main par c7 sur
un lot nommé, jamais la règle du pousseur.

## Les créneaux

Un créneau, c'est un worktree, un target chaud et un train rejoué. Avec un seul créneau, le train suivant attend
la fin du précédent.

Avec N créneaux, les trains sont spéculatifs : le créneau 1 rejoue A, le 2 rejoue A+B, le 3 rejoue A+B+C. Si A
est vert, A est poussé ; si A+B est vert, B l'est aussi, et ainsi de suite. Au rouge de A, le créneau 2 rejoue B
seul. Chaque créneau a ses ports d'hôtes e2e pris libres ; le service d'embarquement est partagé.

Pourquoi pas « le créneau 1 rejoue A, le 2 rejoue B » : B serait poussé sur une tête, master+A, qu'il n'a jamais
vue.

**Ce que luciepc porte**, relevé le 18 novembre à 20 h :

| ressource | total | ce qu'une passe Rust prend |
|---|---|---|
| mémoire | 93 Go ; 41 disponibles pendant la passe F300 du banc | plafond 32 Go (`POSTE_MEM_MAX`, la règle des validations) |
| cœurs | 24, 2 laissés libres ; au plus 4 lourds | 22 / 4 = 5,5 cœurs par lourd |
| disque | 868 Go libres | un target chaud : 49 Go (target-G) ; les autres targets de session, 15 à 23 Go |

**Conclusion : un créneau au départ, deux au plus.** Deux passes de 32 Go font 64 Go. Elles tiennent sur 93 Go
quand rien d'autre ne tourne, pas en même temps que la F300 ou une mesure. Trois créneaux (96 Go) ne tiennent
jamais.

Un second créneau prend un lourd de plus, donc cinq cœurs et demi aux autres sessions, et 25 à 50 Go de
disque pour son target chaud. Il ne paie que si des trains attendent derrière un train en cours. On l'ouvre sur
mesure : le journal donne l'attente de chaque lot, et l'on décide à partir de cette attente, pas avant.

**Le pic réel** d'une passe n'est pas encore mesuré. Le plafond de 32 Go est une enveloppe, pas une mesure.
Le lot 1 relève `memory.peak` du scope de chaque rejeu dans le journal. Si le pic réel est nettement sous 32 Go,
deux créneaux reviennent à la discussion, avec des chiffres.

## Décrit, pas activé

Ces deux modes ne s'activent que si le train laisse encore de l'attente mesurée :

- **Les lots disjoints.** Du code est arrivé sur master, mais le lot n'a ni fichier ni suite exigée en commun avec
  lui : il est poussé sans rejeu.
- **La veille de master.** Après chaque train, un rejeu de master, non bloquant. S'il rougit, la file se met en
  pause et les deux lots en cause sont nommés.

## Les lots

| lot | contenu | témoin, rouge d'abord |
|---|---|---|
| **1** | `pusher.sh` : `enqueue`, `dequeue`, `run` pour un train d'**un** lot ; la repose dans le worktree unique ; le rejeu par `kind` ; le journal ; la boîte par auteur ; `memory.peak` au journal | `tools/poste/test_pusher.sh` : un dépôt jetable et un origin local, `push_garde.sh` et `passe_epinglee.sh` remplacés par des doublures ; dépôt refusé (id doublon, tête bougée, kind faux, cpp), docs poussés sans rejeu, scripts rejoués, conflit rouge, rouge de code, rouge d'environnement rejoué une fois, `paused` respecté |
| **2** | le train de `k` lots et la bissection | train de quatre lots dont le troisième rouge : trois poussés, un rendu |
| **3** | la suspension des lots Rust sous une lib en retard | lib plus vieille que le moteur de master : `rust` suspendu, `docs` poussé |
| **4** | la session-pousseur : lire le journal, prévenir les auteurs, dire un rouge en clair | essai à blanc sur un journal rejoué |
| **5** | N créneaux, si le journal montre de l'attente | à écrire après la mesure |

Le pousseur réel ne tourne qu'après les lots 1 à 3, à un « oui » de c7. D'ici là, le lot 1 se joue contre son
témoin seulement.

## Le débit, dit en clair

- Un lot de docs ou de scripts passe en quelques minutes.
- Un train Rust prend 30 à 45 minutes : témoin, `e2e_lectures`, `e2e_code`, suites exigées, `check_features` sur
  huit jeux de features, lib.
- Sans train, c'est 1,5 à 2 lots Rust par heure au plus, et c'est le plafond. Avec le train, quatre lots Rust
  passent dans le même rejeu, si aucun n'est rouge.
- Un second créneau ne double pas le débit : il ne sert que lorsque des trains attendent.
