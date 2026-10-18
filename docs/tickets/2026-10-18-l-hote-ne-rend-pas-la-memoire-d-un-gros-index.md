# L'hôte ne rend pas la mémoire d'un gros index : 10 Go restent dans l'échange, l'index fini

- **État** : ouvert (18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration).
  Cause non cherchée. Le correctif peut vivre dans le moteur (le gestionnaire de tampon, l'index
  vectoriel), dans l'hôte, lucivy ou l'allocateur : en cas de doute, la table du moteur.
- **Gravité** : mémoire. Rien n'est perdu ni faux, mais le poste reste chargé tant que l'hôte vit.
- **Atteignable en service** : oui, après tout gros index ou rattrapage dans un hôte qui reste
  ouvert ensuite.
- **Touche rag3weaver** : oui (l'hôte du démon, celui des sessions MCP).

## Ce que c'est

Une fois un gros index fini, l'hôte rag3weaver ne rend pas sa mémoire au système. Sur le poste
principal, l'hôte de Lucie tenait encore **10 Go dans l'échange** le 15 octobre, trois jours après
le rattrapage du 12. Il n'indexait rien depuis. L'orchestration l'a arrêté proprement le 15, à la
demande de Lucie. Avec l'effacement de 755 dossiers temporaires morts, voir
[le ticket des dossiers temporaires](2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md),
l'échange du poste est passé de 22 à 10 Go.

Le rattrapage du 12 est celui où l'hôte a monté à 44-52 Go de mémoire résidente : l'index de
`code-principal` a été lancé dans un hôte déjà figé à 30 Go (le `break` d'un garde-fou ; mémoire
de session `garde-fou-arrete-tout-le-script`).

## Recette

Aucune recette minimale pour l'instant. Seul existe le relevé du 15 octobre (orchestration,
`extension/rag3weaver/docs/11-octobre-2026-10h51/orchestration/01-les-decisions-du-matin.md`, au
15). Ce relevé ne garde ni la mémoire résidente ni la composition de l'échange (`smaps`) de l'hôte
avant son arrêt.

## Le témoin à écrire

Une sonde, sans assertion d'abord : elle mesure.

1. Un hôte neuf ouvre un backend de code. On lit `VmRSS`, `VmSwap` et `RssAnon` dans
   `/proc/<pid>/status`, et `smaps_rollup`.
2. On indexe un dépôt de taille connue sous un tampon donné (`RAG3DB_BUFFER_POOL_SIZE`), puis on
   attend la fin et le repos (plus aucune tâche).
3. On relit les mêmes compteurs à 0, 1 et 10 minutes de repos. On ferme le backend sans arrêter
   l'hôte, et on relit encore.

Le défaut est établi si la mémoire anonyme au repos, backend fermé, reste nettement au-dessus de
l'état neuf plus le tampon. La sonde se joue sous `poste mesure`, sur le jeton de l'orchestration
(une grosse indexation à la fois par poste).

## Pistes, aucune vérifiée

- **Le tampon du moteur** : les cadres déjà touchés restent à l'hôte tant que la base reste
  ouverte. Ce comportement est attendu jusqu'à la taille du tampon. La question est ce qui reste
  au-delà, et après la fermeture de la base.
- **La mémoire tenue hors du tampon** : c'est déjà le
  [ticket de la mémoire résidente au-delà du tampon](2026-10-11-memoire-residente-au-dela-du-tampon-sous-des-blobs.md),
  environ 1 Go par millier de fichiers à la première indexation. Le présent ticket en serait la
  suite : cette mémoire ne redescend pas, une fois le travail fini.
- **L'allocateur** : glibc garde ses arènes (une par fil) et ne rend une page au système qu'en haut
  du tas, sauf `malloc_trim`. Un essai consiste à appeler `malloc_trim(0)` au repos dans la sonde
  et à voir ce qui tombe. C'est un essai, pas un correctif.
- **Ce que tient l'hôte lui-même** : l'embarqueur, les caches de lucivy, les files de l'index.

## Mesuré : d'où vient le pic d'un premier index (arbre principal, 18 octobre 2026)

Luciepc, master `c6bb094fb` (lib du magasin 0fc7dd10f), worktree `rag3db-tampon`, la recette du
ticket du 11 octobre (`RAG3WEAVER_ESTIMATE_REPO=1`, `e2e_estimate
ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, base sur disque), qui passe à côté de la
borne, sous `poste lourd`, services d'embarquement locaux, régime doux. 6 318 fichiers, 57 Mo de
texte compté par la passe, 687 467 liens. Journaux : `~/.cache/rag3weaver-notes-A/tampon/` sur
luciepc. Le pic est relevé par le test toutes les 100 ms ; il tombe chaque fois pendant la pose
des relations.

| passe | plein texte | tampon | résultat | durée | pic au fil de l'eau | anonyme à 1 024 / 3 072 / 5 120 / 6 318 fichiers |
|---|---|---|---|---|---|---|
| paquets de 512 | fichiers | 2 Gio | **vert** | 69 s | 7 907 Mo | 2 359 / 4 576 / 5 569 / 6 288 Mo |
| paquets de 512 | fichiers | 4 Gio | **vert** | 68 s | 7 677 Mo | 2 365 / 4 493 / 5 265 / 5 967 Mo |
| paquets de 512 | blobs | 2 Gio | **vert** | 124 s | 9 212 Mo | 2 635 / 5 253 / 6 801 / 7 580 Mo |
| paquets de 512 | blobs | 4 Gio | **vert** | 122 s | 9 805 Mo | 2 673 / 5 609 / 7 269 / 8 405 Mo |
| paquets de 2 048, un COMMIT par paquet (le défaut d'`index`) | fichiers | 4 Gio | **vert** | 58 s | 7 982 Mo | 3 769 à 2 048 ; 6 446 à 6 144 |
| paquets de 2 048, une seule transaction (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=tous`) | fichiers | 4 Gio | **vert** | 56 s | 8 242 Mo | 3 699 à 2 048 ; 6 599 à 6 144 |

- **Ni le tampon ni la transaction ne font le pic.** Sous 2 puis 4 Gio, il bouge de 0,2 Go
  (7 907 contre 7 677 Mo) ; en une seule transaction plutôt qu'un COMMIT par paquet, de 0,26 Go
  (8 242 contre 7 982 Mo).
- **C'est l'anonyme**, qui croît d'environ **1,1 Go par millier de fichiers** et ne redescend pas
  d'un paquet à l'autre, alors que chaque paquet est validé. Le plein texte en blobs en ajoute
  environ 1,3 Go au bout du compte.
- **Le pic tombe pendant la pose des 687 467 relations**, après les paquets (« Relations, 687467
  liens à poser »), environ 1 Go au-dessus de la mémoire de la fin des paquets.
- Le pic de 8,2 Go de l'hôte de luciepc (10 Gio de tampon) est donc l'ordre de ce que fait la passe
  elle-même, pas un tampon trop grand. Les pistes à départager restent celles d'en haut, l'anonyme
  d'abord : ce que retient chaque paquet après son COMMIT (files de l'index, état des relations
  différées, caches de lucivy, arènes de l'allocateur).

## Pour le fermer

1. Écrire la sonde, puis savoir quelle part revient à chaque piste.
2. Corriger la part qui ne redescend pas (rendre au système au repos ou à la fermeture d'un
   backend).
3. Passer la sonde en témoin : au repos, backend fermé, la mémoire anonyme revient sous un seuil
   écrit dans le test, avec sa justification.
