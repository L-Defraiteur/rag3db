# La fenêtre de la bascule des bases de Lucie

15 novembre 2026, arbre principal. Ce que la bascule fait sur chacune des trois bases, dans quel ordre, et
combien de temps chaque étape demande ; puis le retour arrière, et la question posée à Lucie.

## Les trois bases

| base | poste | fichier de base | `.fts` |
|---|---|---|---|
| `code-rag3db-depot` | portable | 3,55 Go | 1,4 Gio |
| `code-lourd` | luciepc | 1,37 Go | 730 Mio |
| `code-principal` | luciepc | 1,13 Go | 620 Mio |

Tailles relevées le 15 novembre à 14 h 50.

## L'ordre, base par base

1. **Annonce** aux sessions qui lisent la base : le backend s'arrête pendant la fenêtre.
2. **`maintain` de lucivy** par l'hôte (4.3.8, hôte rebâti sur e95a1b8c6 ou plus récent) :
   `fts-live-ids`, la lecture du graphe, puis `maintain` avec la purge des orphelins (recette de hotfixs)
   et `compact: true`. Il rend de la place avant la copie (4,93 → 1,35 Go mesurés par hotfixs sur
   `code-rag3db-depot`). Il tient le backend en écriture environ 20 s par grande table.
3. **L'instantané** : base, wal, extensions et `.fts` ensemble, avec le manifeste de copie. C'est le
   retour arrière.
4. **1b**, la migration des versions (`rag3weaver-backend migration versions --base …`), par lots de 2 000.
5. **Le repassage de l'écrivain des générations** (rang 0 et position) sur la base migrée.
6. **La vérification** : les requêtes de la copie, identiques avant et après ; le backend rouvert par
   l'hôte.

## Les durées

Mesurées sur `code-rag3db-depot` :
- **1b : 861 s**, sur la copie du 14 novembre (page `14-11-2026/01`) ;
- **le repassage par l'écrivain : environ 45 min**, dix fois le chemin d'avant (2 656 s contre 254 s au
  banc). Le ticket est
  `everything-declarative/tickets/2026-11-15-le-repassage-d-une-base-par-l-ecrivain-coute-dix-fois-le-parent.md`.

Pour les deux bases de luciepc, j'extrapole **au prorata de la taille du fichier de base**. C'est une
approximation : le coût suit le nombre d'occurrences et de versions, pas les octets, et luciepc n'est pas
le portable. J'y ajoute une marge de moitié.

| base | 1b | repassage | maintain + copie + vérification | **fenêtre, marge comprise** |
|---|---|---|---|---|
| `code-rag3db-depot` | 15 min (mesuré) | 45 min (mesuré) | ~10 min | **~1 h 30** |
| `code-lourd` | ~6 min | ~17 min | ~5 min | **~45 min** |
| `code-principal` | ~5 min | ~14 min | ~5 min | **~35 min** |

Les deux bases de luciepc peuvent passer l'une après l'autre dans la même fenêtre (environ 1 h 20), ou
en parallèle du dépôt, qui est sur l'autre poste. Si le ticket du repassage est corrigé avant la bascule,
la fenêtre se raccourcit d'autant : l'écrivain retomberait vers 5 min sur le dépôt.

**Le chiffre de luciepc se mesure avant la vraie fenêtre**, par la répétition sur copie (la première
des trois conditions). La table ci-dessus en sera corrigée.

## La politique (Lucie dit oui, c7 la fixe, 15 novembre)

Lucie : « mes bases sont là pour ça » ; elle laisse la politique à c7.
- **Quand** : ce soir, dès que la file du jour est vidée et que l'écrivain d'ED a atterri (bff6c7409,
  c'est lui que le repassage demande). Le dépôt au portable et les deux bases de luciepc **en
  parallèle**, chacune par son script, sous `poste lourd timeout`, après le `maintain` de lucivy. Les
  hôtes des deux postes sont rebâtis au-delà de e95a1b8c6.
- **La répétition sur copie d'abord**, aujourd'hui, pour les vrais chiffres de luciepc. Si elle casse,
  la bascule attend le correctif, pas Lucie.
- **Chaque base a son journal de bascule** : les heures, les comptes avant et après par table, le relevé
  à blanc de lucivy avant et après. Son état d'index dit « basculée le … ». Chaque fermeture est
  annoncée aux sessions avant.
- **Le repassage** est la première synchronisation d'un hôte qui porte l'écrivain, sur une base passée
  aux générations (`migrer_vers_les_generations`, après 1b). Le journal de l'index dit « première
  synchronisation par l'écrivain : N fichiers à repasser ». Compter environ 0,35 s par fichier (ED,
  d'après banc), hors vecteurs, qui ne se recalculent pas.
- **Les orphelins de lucivy se relèvent au moment de la bascule** : les listes de hotfixs de la nuit ne
  servent plus. La recette est celle de hotfixs (`docs/coeur-cpp-hotfixs-tickets/13-11-2026/outils/`,
  `fenetre-purge.sh compter`) : les vivants du `.fts`, moins les offsets de la table du graphe, sur
  Scope et File. Base ouverte, l'ordre fait la sûreté : le jeton d'abord, le graphe ensuite, puis
  `maintain` avec `max_purge_ratio` à 0,01.

## Le retour arrière

- **Pendant la fenêtre**, si une étape échoue ou si la vérification diffère : on remet l'instantané de
  l'étape 3 (quelques minutes, le temps de la copie) et on rouvre.
- **Après** : l'instantané et l'ancien index sont gardés **72 heures**, puis retirés base par base au mot
  de c7, sans redemander à Lucie. Une base que Lucie a fait rouvrir sans plainte en 72 h est acquise. Le
  retrait rend environ 5 Go par base.

## Ce qui attend encore

- la répétition sur copie, avec les vrais chiffres de luciepc (vers 17 h, le 15 novembre) ;
- le ticket « une recherche à versions prend des Gio » fermé ;
- l'écrivain d'ED (bff6c7409) sur master.
