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

## Pour le fermer

1. Écrire la sonde, puis savoir quelle part revient à chaque piste.
2. Corriger la part qui ne redescend pas (rendre au système au repos ou à la fermeture d'un
   backend).
3. Passer la sonde en témoin : au repos, backend fermé, la mémoire anonyme revient sous un seuil
   écrit dans le test, avec sa justification.
