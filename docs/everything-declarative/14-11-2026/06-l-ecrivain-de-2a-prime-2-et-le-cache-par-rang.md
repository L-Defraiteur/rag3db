# L'écrivain de 2a′-2 et le cache par (couche, rang)

Conception, écrite avant le code, au mot de c7 du 14 novembre 2026 (22 h 30). Elle prolonge :
- la page 03, § 2 bis : la couche de travail, et une génération par commit ;
- la page 03, § 8 : l'identifiant stable de couche ;
- la page 05 : la vue en prédicat, et la raison de l'absence de cache dans vue-publiee (§ 2 bis).

## 1. Deux rangs vivants par couche

- **n**, le dernier rang **publié**. Il est figé : aucune ligne visible à n ne change plus.
- **w = n + 1**, le rang **en attente** : la couche de travail. La montre le réécrit à chaque
  sauvegarde.
- **La lecture sans vue prend w** : c'est l'arbre vivant, comme aujourd'hui. Une vue (une couche, un
  appel épinglé) prend un rang publié, **jamais w**.

## 2. Les règles de l'écrivain, ligne par ligne

Elles valent pour `Scope`, pour `File` et pour leurs arêtes, sur la couche écrite.

| la ligne trouvée | le scope réindexé est… | l'écrivain fait |
|---|---|---|
| `_depuis = w` (née dans l'attente) | changé | la réécrit **sur place** |
| `_depuis = w` | disparu | l'**efface** physiquement (elle n'a jamais été publiée) |
| `_depuis ≤ n`, vivante | identique | ne fait rien |
| `_depuis ≤ n`, vivante | changé | pose `_jusqu_a = w` sur l'ancienne, écrit une **ligne neuve** `_depuis = w`, uuid `hash(clé, couche, w)` |
| `_depuis ≤ n`, vivante | disparu | pose `_jusqu_a = w` (retrait, sans effacement) |
| `_depuis ≤ n`, retirée à w | revenu à l'identique (l'édition revient en arrière) | efface la ligne neuve de w, remet `_jusqu_a` à vide |

**Les arêtes entrantes suivent** (page 03, § 2 bis, 1). Quand une ligne neuve remplace une ligne
vivante, chaque arête entrante vivante se recopie vers la ligne neuve avec `_depuis = w`, et l'ancienne
se ferme à w. Les arêtes sortantes sont celles que l'analyse du fichier réécrit déjà.

**Le grain du snapshot** prend la couche (page 03, § 3). Réindexer un fichier n'efface plus ce qui
n'est pas de w : il retire.

**Le drapeau** : la première pose de `_jusqu_a` appelle `marquer_des_lignes_invisibles`. C'est son
appelant, qui manquait (page 04, § 3).

## 3. La publication

- Au commit de la source suivie : une ligne `_generation` (couche, rang = w, commit, base, recette,
  parente épinglée), puis n := w et w := n + 1. **Aucune ligne n'est réécrite** : les lignes nées à w
  deviennent publiées par le seul rang.
- **Le cas d'un arbre sale au commit** : la couche de travail porte alors des sauvegardes que le commit
  n'a pas. Publier w, ce serait publier l'arbre et non le commit. Je propose que la génération dise
  `arbre_propre: false` quand le worktree a des changements non commités au moment de la publication,
  et qu'une vue qui l'épingle le dise. Indexer le commit lui-même, par ses objets git, revient à 2a′-3.
  **À trancher avec arbre principal et c7 avant le code.**

## 4. L'époque d'écriture, et le cache

- **L'époque** : un compteur de la couche, incrémenté à chaque écriture validée de l'écrivain, et porté
  par la `Visibilite` (donc par la vue de lecture).
- **Le cache des ensembles admis** (`EnsemblesPoses`, déjà posé sans contenu dans vue-publiee) :
  - la clé est (couche, rang, époque si le rang est w, l'empreinte du filtre de l'utilisateur, la
    cellule) ;
  - la valeur est l'ensemble des offsets admis pour le lexical (`Arc<[u64]>`), et l'ensemble des
    racines admises pour la projection des k résultats (`Arc<HashSet<String>>`) ;
  - **un rang publié a un cache définitif** : ses lignes visibles ne changent plus, puisqu'un retrait
    plus tard pose `_jusqu_a = w > n`. Ce cache se retire avec la rétention ;
  - **le rang w a un cache par époque**, refait à la première recherche après une écriture ;
  - il est borné : un petit nombre d'entrées par couche (quelques filtres), dont la plus ancienne sort
    la première.
- C'est ce cache qui ferme le ticket de l'état B
  (`2026-11-14-la-visibilite-dans-le-predicat-du-masque-coute-et-mene-a-l-oom.md`) côté lexical.
  La visibilité ne se rejoue plus par requête, mais par époque.

## 5. Le dense sous visibilité

Le dense ne peut pas recevoir un ensemble d'offsets. `QUERY_VECTOR_INDEX` ne filtre que par un graphe
projeté, et ce graphe se recalcule à chaque requête sur sa connexion (le cœur, page 05, § 2). Sous
visibilité, la projection courte porte donc le prédicat de visibilité, et c'est elle qui coûtait cher
dans l'état B.

**La proposition : le post-filtre du dense, mesuré.**
- La recherche HNSW **sans** projection, k′ = 4k candidats.
- Ils sont filtrés côté Rust par l'ensemble des morceaux admis, tiré du cache (l'équivalent de
  `allowed_chunk_ids_for`).
- Si moins de k candidats passent, k′ double, jusqu'à un plafond. Au-delà du plafond, ou quand le
  cache dit que la part admise est petite (moins de 20 % de la table), on revient à la projection
  courte.
- **Le risque est le rappel**, et non la justesse : un résultat admis ne sort jamais du post-filtre ;
  il faut seulement aller le chercher assez loin. Le juge de banc tient la parité au régime
  « optimisation » contre la projection, sur les trois formes, avant que le post-filtre soit le défaut.

## 6. Les témoins, rouges d'abord

1. **L'ancre garde son passé** : après une réindexation qui change `f` et un commit, la génération
   d'avant rend l'ancien `f`, et la courante le neuf. Aucune ligne n'est effacée.
2. **Aucune base ne change d'uuid** : une base de 2a′-1 garde tous ses uuid. Ses lignes sont la
   génération 0.
3. **Les arêtes entrantes suivent** : `A` (`a.rs`) appelle `f` (`b.rs`) ; `f` change ; `usages(f)` rend
   `A` à la génération courante comme à celle d'avant.
4. **La couche de travail** : dix sauvegardes du même fichier ne publient aucune génération ; un commit
   en publie une. Une lecture sous une vue de l'ancre à son rang publié ne voit pas les sauvegardes en
   attente.
5. **Un retour en arrière s'efface** : sauvegarder une édition, puis revenir au texte publié, laisse la
   base identique à avant l'édition.
6. **Le cache suit l'écriture** : une recherche filtrée, puis une écriture qui ajoute un scope, puis la
   même recherche, qui rend le scope neuf (l'époque a changé). Et la même recherche refaite sans
   écriture ne résout rien (le cache sert).
7. **Le drapeau tombé ne ralentit pas** : sous la sonde, l'état B reste dans les 20 % de l'état A. C'est
   une sonde sous `poste mesure`, pas un test de la passe.
8. **La non-fuite étendue** : le témoin 0 de 2a′-1 joue aussi `outline`, ou dit pourquoi il ne le peut
   pas (page 03, § 7).
9. **La parité de banc** au bit près, contre master, sur les trois formes, sous le drapeau tenu.

## 7. Ce qui se mesure avant le code

- **La mesure bornée des déplacés** (arbre principal, page 03, § 4) : sur les 50 derniers commits de
  master de code-lourd, rejoués sur une copie, le nombre d'arêtes entrantes recopiées par commit, et la
  part des seuls déplacés. Si les déplacés font plus de la moitié des recopies, **et** qu'un commit
  médian dépasse quelques milliers d'arêtes, la position sort de l'identité de la ligne. C'est elle qui
  décide de la forme de la ligne neuve du § 2 ; elle passe avant tout.
- **La mémoire du cas filtré sous le prédicat** (ticket de la mémoire résidente, cas ouvert) : où vont
  les 20 Gio, par la sonde de la vue, avant que G mesure sur l'hôte réel.

## 8. L'ordre

1. La mesure des déplacés, et la décision sur la position.
2. La question de l'arbre sale au commit (§ 3), tranchée.
3. Les témoins 1 à 6, rouges.
4. L'écrivain (§ 2), la publication (§ 3), l'époque et le cache (§ 4).
5. Les témoins verts, la parité de banc.
6. Le post-filtre du dense (§ 5), mesuré ; le témoin 7 sous `poste mesure`.

## 9. La mesure des déplacés, et sa décision (14 novembre 2026, 22 h 52)

Sonde `tests/sonde_deplaces.rs` (branche mesure-deplaces, 03f6fc2c9), à luciepc, sur les 50 derniers
commits d'origin/master (987bdd49e). Pour chaque fichier de code touché, les scopes avant et après sont
analysés et appariés par leur clé. Les arêtes entrantes sont lues sur une copie de la base d'avant les
versions : c'est un ordre de grandeur, pas un compte exact.

- **44 849 arêtes recopiées, dont 38 467 par les seuls déplacés, soit 86 %.**
- La médiane par commit vaut 0, car la plupart des commits ne touchent que des docs. **La médiane des
  commits de code est de 1 500 arêtes**, et le maximum de 12 444 (cbb529c7e, un commit de masse de 1b).
  Les suivants : 194a4e572, 9 967 arêtes dont 8 908 par 414 déplacés ; 00e28eee6, 6 820 ;
  9fd946ec3, 5 839.

**La décision** (arbre principal, avec c7) : **la position reste dans l'identité de la ligne pour
2a′-2.** À la lettre, le seuil ne la sort pas : plus de la moitié des recopies sont bien des déplacés,
mais le commit de code médian est sous « quelques milliers ». La recopie se paie une fois, à la
publication, jamais à la sauvegarde.

86 % de ces recopies n'achètent pourtant qu'un numéro de ligne. C'est une dette, nommée, avec deux
conditions :
- **l'écrivain dit dans l'état, à chaque publication, le compte des recopies** : le total, et la part
  des déplacés ;
- **on redécide au-delà de 20 000 recopies sur un commit réel.** La position sort alors de
  l'identité, vers un petit enregistrement par génération (page 03, § 2 bis).

## 10. Le `Symbol` délié (14 novembre 2026, 23 h 40)

Le ticket de banc `docs/banc/tickets/2026-11-14-un-symbol-que-plus-rien-ne-relie-reste-apres-le-retrait-de-son-fichier.md` :
après le retrait d'un fichier, ses `Symbol` exclusifs restent, sans relation.
- `Symbol` reste partagé, sans colonnes de génération (page 03, § 2 bis, 3). Il n'est **pas fermé à la
  publication** : une génération plus ancienne peut encore le relier.
- **Un `Symbol` est visible à un rang s'il a au moins une relation visible à ce rang.** C'est un
  prédicat pour les lecteurs qui l'atteignent directement (la recherche `Symbol`, le rendez-vous par
  nom), comme le masque des versions.
- **Son effacement physique revient à la rétention**, quand plus aucune génération gardée ne le relie.
- Un nom qui revient retrouve le même uuid (`hashsafe`), sur une ligne qui n'a jamais été effacée.
- **Le témoin de plus** : après le retrait d'un fichier et une publication, une recherche `Symbol` sans
  vue ne rend pas ses `Symbol` exclusifs, et une vue au rang d'avant les rend.

## 11. L'écrivain sur tous les chemins d'écriture (15 novembre 2026, 1 h 20)

Branche `ecrivain-2a-prime-2`, tête `cd3f1e978` ; lib entière verte (1974 passés, 0 échec, 16 ignorés).

- **Les chemins** : `reingest_file`, `reingest_files`, `remove_file` et `sync_source` d'une source déjà
  indexée passent tous par l'écrivain, générique par table : `Scope` et `File` suivent les mêmes
  règles (§ 2). `sync_source` ne finit plus par une fin d'instantané, qui effaçait ce qu'aucun paquet
  n'avait marqué, lignes publiées retirées comprises : il réécrit la source par paquets, puis retire
  les fichiers que la couche voit encore et que la source ne liste plus. Une première indexation garde
  le chemin d'instantané : rien n'y est publié.
- **Le compte des recopies** : chaque publication pose `aretes_recopiees` sur sa ligne `_generation`,
  puis le compte repart de zéro. **Limite dite** (c7, 15 novembre, 1 h 1x) : ce compte vit en mémoire
  jusqu'à la publication, il compte donc **depuis l'ouverture** du processus. Un redémarrage entre deux
  publications perd les recopies faites avant lui. Pas de persistance ce soir. **La part des
  déplacés** (§ 9) n'est pas encore comptée à part : c'est le total seul.
- **Une ingestion directe** (`Catalog::ingest_code_jusqu_a` appelé hors des chemins ci-dessus) ne
  décide aucune naissance : sous les générations, une ligne née au rang en attente et sa clé nue s'y
  doubleraient. Seuls les témoins l'appellent ainsi, et ils passent d'abord par le pas de l'écrivain
  (`code_sync::preparer_par_generations`).
- **Rouvrir une ligne** pose `_jusqu_a` à un NULL typé INT64 : un NULL nu prend STRING à la conversion
  vers le moteur (`sans_inconnu`, voulu), et le lieur refuse alors de le changer en INT64. Ce n'est pas
  un défaut du moteur.
- **Les témoins** 1 à 5 et 7 à 11 (`code::tests_de_l_ecrivain`), chacun vu rouge d'abord ; le 6 vient
  avec le cache. La parité et le compte des recopies attendent le jugement de banc.

### Ce que les suites ont trouvé (15 novembre 2026, 1 h 15 à 1 h 50)

La lib entière sous `rag3db-native,code` était verte (1974/0/16) ; e2e_code, `check_features` et la
lib sous le jeu le plus large ont trouvé huit défauts de plus, tous du lot, tous corrigés sur
`ecrivain-2a-prime-2` :

1. **Le miroir `CONSUMED_BY`** va de l'appelé vers l'appelant : sur une ligne retirée, c'est une arête
   sortante, que l'oubli ne défait pas et que l'ingestion du fichier ne repose pas. Il ne suivait pas
   la ligne neuve ; il suit maintenant, compté dans les recopies. (`usages` ne le lit pas : banc l'a
   vérifié, la parité ne l'aurait pas vu.)
2. **`scopes_deleted`** comptait les lignes publiées retirées pour un remplacement : il ne compte plus
   que les clés disparues, comme sans générations.
3. **L'empreinte indexée d'un fichier** (la fraîcheur de `read_file`) se lisait par l'uuid de sa clé
   nue : une ligne `File` née après la première porte sa couche et son rang (`uuid_for_in`), la
   lecture ne la trouvait pas. Elle se lit par ses champs, sous la visibilité (`find_by_field`). C'est
   le seul lecteur qui dérivait un uuid d'une clé.
4. **Deux aides d'e2e_code** (`edges`, `key_of`) lisaient l'histoire brute, lignes retirées et leurs
   arêtes comprises : elles lisent au rang lu, par `Catalog::condition_visible(alias)`, public.
5. **`COUCHE_DU_BACKEND`** vivait dans `code`, que l'identité (`uuid_for_in`, catalogue) lit sans sa
   feature : il vit dans `generations`.
6. **`Catalog::naissances`** n'est lu qu'avec la feature `code` : mort sans elle, refusé sous
   `-D warnings`.
7. **La synchronisation par générations ignorait la demande d'arrêt** entre deux paquets : une
   fermeture attendait la fin d'un rattrapage à l'ouverture (89 s,
   `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`). Elle s'arrête à la même
   frontière que celle d'instantané, avec le même message de suspension.

8. **Un rattrapage à l'ouverture réécrivait toute la source** (trouvé par la lib sous le jeu le plus
   large, le même témoin qu'en 7, encore rouge après lui : 75 s contre 20 s au parent). La
   synchronisation par générations réécrivait chaque fichier, inchangé compris, en un seul paquet ; la
   demande d'arrêt ne se lisait jamais. Sous l'écrivain, un fichier dont l'empreinte indexée (sa ligne
   `File` visible au rang en attente) égale celle du disque ne changerait aucune ligne : il est sauté.
   Sauf après une passe en masse interrompue (sa marque `relations_pending` posée) : ses liens
   manquent, tout se repasse. La passe coûte le diff, pas la source.

Leçon : la lib sous un seul jeu de features ne suffit pas à un lot qui touche l'identité ou
`sync_source` ; la passe épinglée entière (le témoin, les suites, `check_features`, la lib sous le
jeu le plus large) est le minimum.

## 12. Le plan du code de l'écrivain, tel qu'écrit avant le code (15 novembre 2026, 0 h 20)

Les témoins 1 à 5 sont rouges pour la bonne raison sur 9ef56697f (la publication écrite, l'écrivain
non) : la réindexation réécrit sur place la ligne publiée. Le code, dans l'ordre :

1. **L'identité** (`Catalog::uuid_for_in`) : la couche et le rang de naissance entrent dans le
   hachage **seulement quand les données les portent** (une couche autre que `backend`, ou un rang de
   naissance au-delà de 0). Une ligne de la génération 0 garde l'uuid de sa seule clé.
2. **La naissance de chaque clé**, décidée par l'écrivain avant l'ingestion :
   - une ligne en attente existe : son rang (elle se réécrit sur place) ;
   - une ligne publiée vivante, et l'enregistrement identique champ à champ : le rang de la publiée
     (rien ne change) ;
   - une ligne publiée vivante, et l'enregistrement différent : le rang en attente w (une ligne neuve) ;
   - aucune ligne : w.

   La table de ces naissances sert aux **huit calculs d'uuid par clé de l'ingestion** (les bouts des
   arêtes, `code.rs`), qui passent par elle au lieu de la clé seule. Une clé d'un autre fichier, bout
   d'une arête, se lit sur sa ligne vivante, en une requête par lot.
3. **Le retrait, à la place de l'effacement**, pour le grain du fichier :
   - une ligne publiée qui change ou disparaît reçoit `_jusqu_a = w` ;
   - une ligne née en attente qui disparaît s'efface ;
   - un retour à l'identique efface la ligne en attente et rouvre la publiée (`_jusqu_a` à vide).
4. **Les arêtes** : les arêtes entrantes vivantes d'une ligne retirée se recopient vers la ligne
   neuve, et leur nombre est compté, puis dit à la publication (§ 9). L'oubli des arêtes du fichier
   (`oublier_les_aretes_du_fichier`) ne touche plus que les lignes écrites à w, jamais une ligne
   retirée : son histoire garde ses arêtes sortantes.
5. **Les chemins**, dans cet ordre : la réindexation d'un fichier (`reingest_file_retenu`, celle des
   témoins), puis celle d'un lot (`reingest_files`), le retrait (`remove_file`) et l'indexation
   entière (`sync_source`). `File` suit la même règle après `Scope`.
6. **Hors de ce lot** : les colonnes de génération sur les arêtes elles-mêmes. Une arête recopiée est
   une arête neuve vers la ligne neuve ; l'arête de la ligne retirée reste, et sa ligne retirée la rend
   invisible par le prédicat du nœud atteint.
