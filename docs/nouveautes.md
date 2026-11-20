# Nouveautés

- 20 novembre 2026 — **Le chat de l'IDE dit quand l'index n'est pas prêt, puis quand il l'est.** Sous un outil qui a répondu avant que l'index ne rattrape, une ligne dit en clair combien de temps il faut encore pour le texte et pour les vecteurs, ou pourquoi les vecteurs manquent ; l'outil n'est pas refusé. Une fois l'index à jour, une seule ligne le dit dans le chat.
- 19 novembre 2026 — **Une seule notification quand l'index d'une vue est prêt.** Quand une recherche ou une lecture
  a répondu « index pas prêt », le serveur MCP de l'agent surveille cette vue. Quand elle est prête, il envoie une
  seule notification (« l'index de la vue … est prêt : texte et vecteurs ») ; rien à chaque réindexation. La veille
  sonde l'hôte à l'estimation annoncée, puis toutes les 5 s. Elle renonce à trois fois l'estimation, ou au bout de
  30 min. Ses réglages se déclarent dans le manifeste, sous `search.ready_watch`.

- 19 novembre 2026 — **Une recherche qui lit les vecteurs attend aussi les vecteurs dus.** `search_code` en hybride
  ou par le sens attend que les vecteurs de sa vue soient calculés quand c'est court (sous `search.index_wait_s`,
  6 s par défaut). Sinon, elle répond tout de suite et dit les deux estimations à part : « index pas prêt : texte
  environ 0 s, vecteurs environ 40 s ». En bm25 seul, elle n'attend que le texte. Sans service d'embarquement, elle
  n'attend pas et dit pourquoi. usages, outline, read_file et grep n'attendent jamais les vecteurs.

- 19 novembre 2026 — **Deux fenêtres de l'IDE sur le même chat suivent le même tour.** Une fenêtre ouverte au milieu d'un tour montre son texte depuis le début, et non seulement les mots arrivés après elle : le worker partagé du flux le garde pendant le tour. Une relecture d'une fenêtre, partie avant un tour d'une autre, ne l'efface plus.
- 19 novembre 2026 — **Arrêter un appel coupe aussi la requête qu'il fait au moteur.** Un appel arrêté (par l'IDE,
  `stop_run` ou son `deadline_ms`) allait au bout de la requête que son nœud avait déjà lancée, par exemple un plein
  texte flou de 30 s. Sa lecture est maintenant interrompue en vol : il rend la main en moins d'une seconde, et les
  appels voisins sur le même backend vont au bout des leurs.

- 19 novembre 2026 — **Le chat de l'IDE ne fait plus disparaître un tour.** Une relecture de la conversation
  partie avant un tour et revenue après lui remettait un écran sans ce tour, jusqu'au rechargement suivant. La page
  n'applique plus une relecture dépassée par un tour.
- 19 novembre 2026 — **Un paquet d'index ne casse plus sur un ALTER validé pendant lui.** Chaque paquet prend ses
  tables dès son `BEGIN` (`CALL lock_tables`). Un ALTER validé entre-temps le refuse avant toute écriture : le
  paquet est annulé sans abîmer le catalogue et recommence sur place, trois fois au plus, et le journal d'index le
  dit. Un moteur sans `lock_tables` ne prend rien, comme avant.
- 19 novembre 2026 — **La ligne d'attente d'un outil, dans la boîte de l'équipier, est en anglais**, comme les autres lignes de l'hôte que le modèle relit à chaque pas : « "read_file" has been waiting for N s … ».
- 19 novembre 2026 — **Le modèle lit le texte de ses lectures, pas leur enveloppe.** `read_file`, `grep_files` et `list_files` lui rendent le texte qu'ils ont lu, et non plus leur réponse JSON entière ; un échec, lui, reste rendu en entier, pour qu'il ne disparaisse jamais derrière un texte partiel.

- 18 novembre2026 — Dans le terminal de l'agent, ce qu'on tape pendant une commande `!` ne se garde plus jamais dans sa sortie, même tapé avant que la commande coupe l'écho : un mot de passe tapé avant l'invite de `sudo` se voit à l'écran, comme dans tout terminal, mais n'entre ni dans la consigne envoyée à l'équipier, ni dans le journal.

- 19 novembre 2026 — **La page du chat de l'IDE dit ce qu'un tour attend.** Pendant qu'un outil attend, sa ligne d'état dit « propose attend… (N s) » ; pendant que le fournisseur retient le modèle, « attend le fournisseur… (reprise dans N s) » ; le compte tourne dans la page, sans rien demander à l'hôte.

- 19 novembre 2026 — **Un outil lent du chat de l'IDE dit qu'il attend.** Au-delà de `teammate.tool_wait_notice_s`, un tour du chat publie `Waiting` puis `Resumed` sur son run, comme un pas de l'équipier : la page et le terminal peuvent nommer ce qu'ils attendent.

- 20 novembre 2026 — **Une base dont des conteneurs sont écartés de la recherche ne rebâtit plus son plein texte à
  chaque ouverture.** Depuis 2.12, un namespace replié n'a plus de document de plein texte, mais la garde des comptes
  comparait les documents à toutes les lignes : elle voyait un écart et rebâtissait l'index à chaque réouverture (22 s
  sur la base du dépôt). Elle compte maintenant les seules lignes candidates. Une base indexée avant 2.12 rebâtit une
  fois, à sa première ouverture.
- 19 novembre 2026 — **Un tour qui attend le dit sur le flux.** Un outil qui attend au-delà de `teammate.tool_wait_notice_s`, ou un modèle que le fournisseur fait patienter, publie `Waiting` avec la cause (`tool` ou `model`), le nom de l'outil ou du modèle et le temps déjà attendu, puis `Resumed` quand il reprend. Le terminal et l'IDE peuvent ainsi nommer ce qu'ils attendent.
- 19 novembre 2026 — **Une valeur citée se définit, et seule une définition changée fait repasser ses fiches à revoir.** `define_ref` écrit la définition d'une valeur déjà citée (la valeur d'une catégorie, par exemple) ; les fiches ancrées à cette valeur passent « à revoir », leur contenu intact. Une citation (`add_ref`, `ancrer_fiche`) ne les touche plus et n'efface jamais une définition. `review_anchored` prend un filtre `change` pour cela.
- 20 novembre 2026 — **Un paquet du dépôt importé par son nom se résout par l'import, et les réexportations de Python et de TypeScript se suivent.** Dans les tests d'un crate (`use rag3weaver::Catalog`), un import par le nom d'un paquet npm du dépôt, ou par un `__init__.py` qui réexporte (`from monpaquet.lib import Catalog`), le lien vers la définition était deviné par le nom ; il passe maintenant par l'import, lu dans les manifestes (`Cargo.toml`, `package.json`). `export { X } from './x'` et les imports nommés d'un `__init__.py` sont des réexportations, comme `pub use` en Rust. Sur le crate rag3weaver entier, 5 493 liens passent de « nom » à « import » et 325 s'ajoutent, aucun ne se perd. Les bases repassent une fois (règles de l'analyse 11).

- 20 novembre 2026 — **Un namespace replié ne passe plus devant la classe qu'on cherche.** Un `namespace` ou un
  `module` qui porte à lui, hors de ses lignes de structure (ouvertures, plis des conteneurs imbriqués, déclarations
  avancées, accolades fermantes et leurs commentaires), moins de 80 caractères n'est plus candidat à la recherche :
  ni plein texte, ni vecteurs. Il reste dans le graphe, `outline` et `usages`. Le seuil est une propriété de la base
  (`workspace.container_min_chars`). Sur les en-têtes du moteur, `rag3db_database` passe au rang 8 de « rag3db » (absent
  du top 20 avant). Les bases repassent une fois à leur prochaine synchronisation (règles de l'analyse 10), et le
  journal d'index compte les conteneurs écartés.
- 20 novembre 2026 — **`usages` d'un type compte les appels `Type::méthode()`.** Un appel comme `Catalog::new()`, `PhysicalTypeID::INT64` ou `Catalog.create()` se reliait à la méthode, mais pas au type lui-même : un scope qui ne nommait le type que par cet appel n'apparaissait pas dans `usages Catalog` (41 sites perdus dans les tests de rag3weaver). Le type d'un appel qualifié compte désormais comme un usage, en Rust, C++, Python, TypeScript et C# ; sur le moteur, 3 664 arêtes de plus, 3 845 sur rag3weaver/src, aucune retirée. Les bases repassent une fois à leur prochaine synchronisation (règles de l'analyse 9).

- 19 novembre 2026 — **Une fonction ou une classe nommée comme un mot de JavaScript est une référence, dans tous les langages.** C++, C# et Go reprenaient la liste des mots vides de JavaScript, Python sa liste des noms intégrés : une fonction `of` ou `function`, une classe `Symbol` ou `Map` du projet n'étaient reliées à rien. Les mots vides et les noms intégrés des huit langages viennent maintenant d'une seule source (`data/stop_words/`, un fichier par langage, tirés des grammaires tree-sitter, des mots-clés et du module `builtins` de Python, du `builtin.go` de Go).

- 19 novembre 2026 — **Une variable locale C ou C++ n'est plus prise pour une référence.** `auto total = …`, `int count;` ou `for (auto& item : …)` restaient parmi les références de leur fonction, et se reliaient à une fonction ou un namespace du même nom dans le fichier. Sur le moteur, 73 arêtes fausses partent, et 63 appels qu'une locale homonyme masquait (`auto lock = groups.lock();`) rejoignent leur classe par le type ; `function` n'étant plus un mot vide du C++, ses 79 locales ne se relient pas non plus au `namespace function`. Un appel de méthode sur une locale garde son lien par le nom.
- 19 novembre 2026 — **Le plein texte ne perd plus de mots à la maintenance nocturne (lucivy 4.3.12).** La
  compaction du dictionnaire retirait le texte d'un mot qu'aucun document ne citait plus, mais le mot restait dans
  une table d'attente qui n'était jamais vidée ; un fichier réindexé qui contenait de nouveau ce mot reprenait
  l'ancien id, sans texte, et ne sortait plus en recherche sur ce mot. L'index de code de la base du dépôt en avait
  perdu environ 7 500. Les textes morts restent maintenant un cycle en sursis, et la note de maintenance dit ceux
  qu'elle reprend. Les mots déjà perdus reviennent au rebâti du plein texte.
- 19 novembre 2026 — **Une invocation lit sous la vue de son contexte.** Un outil qui en invoque un autre, dans un
  autre backend (`InvokeToolNode`, `context: {"target": …, "view": …}`), lit désormais sous la vue que porte son
  contexte, au lieu d'être refusé (« vue contextuelle non disponible »). La réponse dit la vue lue. Un `view` explicite
  dans les arguments l'emporte, une vue inconnue de la cible est refusée en se nommant, et sans contexte il n'y a jamais
  de vue implicite.
- 20 novembre 2026 — **Une recherche par clé primaire ne lit plus de mémoire rendue pendant un point de reprise.**
  Le point de reprise d'une table de nœuds remplaçait la liste des blocs d'un groupe pendant qu'une recherche par
  clé primaire la parcourait : sous vingt écrivains, un lecteur pouvait mourir. Le remplacement attend désormais
  les lectures en cours, comme un ajout.

- 19 novembre 2026 — **Après une annulation qui n'a pas pu finir, une écriture déjà ouverte ne valide plus.** Quand
  l'annulation d'une transaction échoue en cours de route, la base demande à être fermée et rouverte, et refusait déjà
  tout `BEGIN` et tout `CHECKPOINT`. Une transaction d'écriture ouverte avant pouvait pourtant encore valider, et une
  validation forcée écrivait alors cet état à moitié annulé dans les fichiers. Son `COMMIT` est désormais refusé par
  le même message (« A rollback of this database could not complete… ») : on annule, on ferme et on rouvre la base ;
  rien de validé n'est perdu, le journal est rejoué à la réouverture. Le point de reprise automatique qu'une autre
  écriture avait demandé n'est pas écrit non plus par le dernier lecteur qui part.
- 19 novembre 2026 — **Une base neuve connaît le débit de son service d'embarquement dès le premier index.** Tant que
  l'index n'a pas mesuré son propre débit, l'hôte lit le débit que le banc a mesuré pour ce poste et ce service
  (`~/.config/rag3weaver/embedding-rates.json`, sous la clé `{modèle}:{adresse}@{poste}`), compté à moitié. Il dit
  d'où vient le débit : « débit de l'index », « débit du banc × 0,5 (mesuré le …) » ou « débit inconnu ».

- 19 novembre 2026 — **La conversation d'un chat se range en mémoire au fil de l'eau.** Chaque tour du chat de
  l'IDE s'écrit dans la mémoire déclarée par le dépôt (`bilans.memoire`). Quand les tours pas encore rangés
  dépassent `memory.slice_tokens` (50 000 jetons par défaut), une tranche se range en fond, sans retarder le chat.
  Quand la conversation approche de la fenêtre du modèle (`memory.compact_at`, 80 % par défaut), ou sur demande, le
  chat se compacte : il repart d'un contexte recomposé depuis ce qui est rangé, avec les deux derniers tours tels
  quels (`memory.keep_turns`), et la numérotation des tours continue. Un tour qui déborde quand même la fenêtre est
  rangé, puis rejoué une fois sur le contexte compacté, au lieu de finir en erreur.

- 18 novembre 2026 — **Un service posé à la main dit pourquoi il s'est arrêté.** Une déclaration `models.embed` (ou
  une autre capacité) peut nommer le journal de son service, `"log": "/chemin/absolu/du/journal.log"`. Quand le service
  ne répond pas et que la dernière ligne de ce journal est « ✗ arrêt : <cause> », l'hôte dit « service d'embarquement
  arrêté : <cause> » au lieu de « service déclaré mais pas joint », comme il le faisait déjà pour un fournisseur
  lancé par une activation.

- 18 novembre2026 — **En Python et en C, une fonction nommée `default`, `from` ou `of` est un appel.** Les mots ignorés de Python et de C étaient ceux de JavaScript ; ils viennent maintenant du langage (les mots-clés de Python, les mots réservés de la grammaire C). Sur les fichiers Python du dépôt, 3 liens de plus ; le C ne change pas. **Toutes les bases repassent une fois** (pointeur codeparsers : les mots vides en données, `72ab7a8`).

- 19 novembre 2026 — **Une nouvelle demande à l'équipier ne remplace plus la précédente.** Ce que l'équipier doit
  faire, c'est l'ensemble des demandes reçues depuis sa dernière fin de travail : la première sous « Request »
  (avec son heure), les suivantes sous « Follow-up ». Une précision envoyée en cours de route s'ajoute à la
  demande au lieu d'en prendre la place.

- 19 novembre 2026 — **Le pas suivant d'un équipier voit ce que le pas d'avant a fait.** Chaque pas reçoit un bloc
  « Previous step » : les fichiers lus et écrits, les commandes lancées et ce qu'elles ont rendu, dans la limite de
  `teammate.carry_over_chars` (8 000 caractères par défaut). L'équipier ne relit plus un fichier ni ne relance un
  test pour retrouver ce qu'il savait déjà au pas d'avant.
- 19 novembre 2026 — **Un outil de lecture relit une génération passée.** Sur une base de code à versions (le défaut
  d'un index neuf depuis le 14 novembre), chaque index fini publie une génération, et les outils de lecture
  (`search_code`, `usages`, `outline`…) prennent `view: {"layer": …, "rank": …}`. Sans rang, ils lisent la dernière
  génération publiée ; avec un rang, l'état de cette génération-là. La réponse dit la vue lue : sa couche, son rang, et
  s'il y a des lignes en attente au-dessus. Une couche ou un rang inconnu est refusé en le nommant. Une base d'avant les
  versions refuse `view` et se dit « à migrer ».
- 19 novembre 2026 — **Un point isolé que rejoint un nuage reste la première réponse à son vecteur.** Un index
  vectoriel bâti en parallèle pouvait perdre de vue un point loin de tout, inséré au tout début du bâti : une fois
  d'autres points posés près de lui, la recherche de son vecteur exact rendait un voisin à sa place (environ une fois
  sur trois au-delà de quatre fils). Le bâti commence désormais par une amorce de points insérés par un seul fil,
  réglée par l'option `seed_points` de `CREATE_VECTOR_INDEX` (1 000 par défaut, 0 pour le bâti d'avant). Elle ne
  change que le bâti, pas l'index : de 3 à 6 % de temps en plus, sans perte de rappel.
- 19 novembre 2026 — **Un L1 dit de quels tours il vient, et un tour dit ce qu'on en a retenu.** Un L1 rangé depuis
  une conversation porte son run et son intervalle de tours, que `recall_apports` rend ; la Source de la tranche
  aussi, avec son résumé. `get_slice` rend une tranche, par son chemin ou par un de ses tours, avec ses L1 et leurs
  fiches ; `list_slices` rend les tranches d'un run dans l'ordre ; `trace_turn` rend d'un tour sa tranche, les L1
  qui le couvrent et leurs fiches. Un tour pas encore rangé appartient à une tranche « ouverte ».

- 19 novembre 2026 — **Les tours d'une conversation expirent après 30 jours.** `purge_turns` efface les tours plus
  anciens que leur durée de vie (30 jours par défaut, ou le nombre de jours donné), avec leur plein texte. Les
  tranches rangées et les L1 qui en sont tirés restent. Seule une entité qui déclare son expiration peut ainsi perdre
  des lignes : un outil qui voudrait expirer des fiches est refusé, en nommant l'entité.

- 19 novembre 2026 — **On retrouve un tour passé par ses filtres ou par ses mots.** `list_turns` liste les tours du
  plus récent au plus ancien, filtrés par run, rôle, outil, statut et date : « l'appel d'archive en erreur d'hier,
  avec ses paramètres et son résultat ». `search_turns` cherche dans leurs mots, et c'est le seul outil qui les voit :
  les rappels ordinaires (`recall_fiches`, `recall_apports`) ne rendent jamais un tour. `read_turns` relit les tours
  d'un intervalle, chacun avec son état : encore indexé, ou disparu.

- 19 novembre 2026 — **Les tours d'une conversation s'écrivent dans la mémoire, et l'archiviste range une tranche par
  ses numéros de tours.** Chaque message d'un tour (la demande, la réponse, chaque appel d'outil avec ses paramètres et
  son résultat) devient une ligne de la mémoire par `put_turn`. Les secrets reconnus (clés et jetons de fournisseurs,
  `Bearer …`, clés privées) sont masqués avant tout enregistrement ; les paramètres et le résultat d'un outil sont
  bornés à 4 000 et 8 000 caractères, la taille d'origine dite. `render_turns` rend une tranche dans une seule forme,
  et `archive_slice(run, from_turn, to_turn)` la range en deux temps : les sujets de la tranche, puis chaque sujet sur
  sa fiche. Elle garde la tranche comme Source, et rend son chemin et jusqu'à quel tour le run est rangé.
- 19 novembre 2026 — **Un accord ne se publie qu'après son reçu.** Une décision sur une carte d'accord écrit
  d'abord son reçu dans le journal des décisions, puis publie l'accord. Sans journal, rien n'est publié, et l'erreur
  le dit : un accord ne semble jamais appliqué sans l'être. Un reçu dont la publication échoue est marqué « non
  posé ».
- 19 novembre 2026 — **Une vue par branche et par worktree.** Une branche suivie a sa couche d'histoire : sa vue lit
  les fichiers tels qu'à son commit, lus dans les objets git, sans checkout ni écriture dans aucun worktree. Un
  worktree suivi a sa couche de travail : sa vue lit ses fichiers non commités, et le reste depuis sa branche. Un
  backend qui déclare `tracked.writes.every_ms` tient ces couches à jour en fond, au plus `tracked.writes.max_work_layers`
  couches de travail (32 par défaut ; au-delà, la couche neuve est refusée en nommant le réglage). Le sélecteur
  `worktree:<motif>` prend les worktrees par motif de chemin (`*` et `?` dans un segment, `~` déplié).

- 19 novembre 2026 — **Une vue sur une branche ou un worktree disparu le dit.** Elle est refusée avant toute lecture,
  par son nom comme par son id : « vue « … » : couche finie le 2026-11-19 14:02 CET (sa ref ou son worktree a disparu) ».
  Avant, par son id, elle se lisait encore comme si la couche vivait.

- 19 novembre 2026 — **Un index fini publie sa génération.** Une vue sans rang lit désormais ce que lit un appel sans
  vue, et non plus l'état de la base au moment de sa migration vers les générations.
- 19 novembre 2026 — **`usages` répond deux fois plus vite pendant un premier index, et un arrêt brutal ne laisse
  plus de contenu dérivé manquant.** Un premier index pose les relations avant le texte complet du code : `usages`
  rend sa réponse complète en 12,2 s au lieu de 23,9 s sur rag3weaver/src, en 40,5 s au lieu de 74,9 s sur le dépôt,
  et l'index entier ne ralentit pas. Si l'index meurt avant de rendre ce texte, la base le sait : l'état dit
  « à rebâtir », et la prochaine recherche ou la reprise le rend avant de répondre.
- 19 novembre 2026 — **Un appel n'attend plus derrière un index sans le dire.** Une invocation d'un outil d'un autre
  backend ne prend plus le verrou du catalogue pour lire sa cellule : elle répond pendant qu'un index écrit. Une
  recherche en cohérence `Strict` attend le catalogue au plus 10 s, puis répond « occupé : une écriture (une
  indexation) tient le catalogue… », avec quoi faire : relancer, ou chercher sans `Strict` pour l'état validé.

- 19 novembre 2026 — **Un ADD ou un DROP de colonne attend les lectures ouvertes de sa table.** Une transaction qui
  lit une table la retient jusqu'à sa fin : un `ALTER TABLE … ADD`, `… DROP` ou un `DROP TABLE` l'attend, et la
  lecture relit ses colonnes au lieu d'être refusée. L'attente est bornée par `alter_lock_timeout` (5 s par défaut,
  et jamais plus que `lock_timeout`) ; pendant qu'un ALTER attend, les lectures neuves de la table attendent derrière
  lui, comme sous PostgreSQL. Un ALTER qui renonce dit qui il attendait : « reader 12 (read-only, open for 3400 ms) »
  ou « writer 15 (open for … ms) ». Un COMMENT, un RENAME ou un DROP d'index n'attendent pas les lectures.

- 19 novembre 2026 — **`CALL lock_tables([...])` : un paquet prend ses tables d'avance.** En tête d'une transaction
  explicite, la transaction tient chaque table nommée comme si elle y avait déjà écrit. Un ALTER validé depuis le
  `BEGIN` la refuse par « Schema changed, retry: » avant toute écriture : on annule et on recommence sur place. Une
  fois la prise faite, un ALTER ou un DROP de ces tables attend la fin du paquet ; deux paquets ne s'attendent pas.
  Une table inconnue est nommée ; en autocommit, l'appel est refusé.

- 19 novembre 2026 — **`usages` ne montre plus la déclaration d'un conteneur retiré.** Une classe retirée par une
  édition ne laissait pas partir ses déclarations : `usages bar` montrait encore « déclaré dans Foo — foo.h:3 »
  après la synchronisation qui avait retiré `Foo`. Les usages ne lisent plus que les lignes visibles de la base.
- 19 novembre 2026 — Dans le terminal de l'agent, ce que l'hôte attend se dit à côté de l'animation : « read_file README.md attend… (25 s) » pendant qu'un outil attend, « gemini-3.8-flash attend le fournisseur… (reprise dans 3 s) » pendant que le fournisseur retient le modèle, au lieu d'un « à toi » faux.
- 19 novembre 2026 — **L'analyse du code suit les imports d'un fichier sans une expression régulière par import.** Pour chaque scope et chaque import externe pas encore relevé, codeparsers compilait `\bnom\b` ; il cherche maintenant le mot entier directement, avec la même frontière. Sur 300 imports par 300 scopes : 12 s → 0,5 s ; sur rag3weaver/src, l'analyse passe de 4,9 à 4,5 s (une mesure, à luciepc). Les arêtes ne changent pas (sonde de marque, au couple près). **Toutes les bases repassent une fois** (pointeur codeparsers : le mot entier, `f164cb6`).
- 19 novembre 2026 — **L'analyse du code est 40 % plus rapide, dans tous les langages.** Pour chaque identifiant, l'extraction demandait à tree-sitter le parent d'un nœud, et tree-sitter le cherche en redescendant depuis la racine du fichier ; l'extraction lit maintenant les parents dans un index bâti en un seul parcours de l'arbre. En release : rag3weaver/src 4,1 → 2,4 s, le moteur 1,5 → 0,9 s, `tools/` 0,25 → 0,15 s. Les arêtes ne changent pas (sonde de marque, au couple près, sur Rust, C++, Python et TypeScript). **Toutes les bases repassent une fois** (pointeur codeparsers : le cache des parents, avec le suivi des imports, `828f855`).
- 19 novembre 2026 — **L'analyse du code est 40 % plus rapide, dans tous les langages.** Pour chaque identifiant, l'extraction demandait à tree-sitter le parent d'un nœud, et tree-sitter le cherche en redescendant depuis la racine du fichier ; l'extraction lit maintenant les parents dans un index bâti en un seul parcours de l'arbre. En release : rag3weaver/src 4,1 → 2,4 s, le moteur 1,5 → 0,9 s, `tools/` 0,25 → 0,15 s. Les arêtes ne changent pas (sonde de marque, au couple près, sur Rust, C++, Python et TypeScript). **Toutes les bases repassent une fois** (pointeur codeparsers : le cache des parents, avec le suivi des imports, `828f855`).

- 19 novembre 2026 — **L'analyse du code suit les imports d'un fichier sans une expression régulière par import.** Pour chaque scope et chaque import externe pas encore relevé, codeparsers compilait `\bnom\b` ; il cherche maintenant le mot entier directement, avec la même frontière. Sur 300 imports par 300 scopes : 12 s → 0,5 s ; sur rag3weaver/src, l'analyse passe de 4,9 à 4,5 s (une mesure, à luciepc). Les arêtes ne changent pas (sonde de marque, au couple près). Il part avec le cache des parents ci-dessus, pour une seule repasse.
- 19 novembre 2026 — **Le premier index d'une base passée aux versions tient sous le tampon du moteur.** La recopie
  des arêtes vers les lignes neuves lit chaque bout par sa clé : sur la base du dépôt, le repassage entier finit sous
  8 Gio (17 min, 7 Gio au plus pour tout le processus), là où il remplissait le tampon dès son premier paquet.
- 19 novembre 2026 — **Un équipier qui rend un compte rendu au lieu du texte demandé est relancé.** Quand un pas se dit réussi sans avoir écrit de texte, et que ses rapports ne font que décrire le travail, le modèle de la tâche juge si la demande attendait un texte. S'il manque, l'équipier repart une fois pour l'écrire, puis attend le superviseur ; si le juge ne répond pas, la boîte dit « fin sans texte, à vérifier ».
- 19 novembre 2026 — **Un équipier qui rend un compte rendu au lieu du texte demandé est relancé.** Quand un pas se dit réussi sans avoir écrit de texte, et que ses rapports ne font que décrire le travail, le modèle de la tâche juge si la demande attendait un texte. S'il manque, l'équipier repart une fois pour l'écrire, puis attend le superviseur ; si le juge ne répond pas, la boîte dit que la fin est à vérifier.
- 19 novembre 2026 — **Un appel n'attend plus sans fin l'ouverture de son backend.** Arrivé pendant qu'un backend s'ouvre (son schéma et ses index se chargent, parfois des dizaines de secondes), un appel attend au plus 5 s, puis répond que le backend s'ouvre et depuis combien de secondes, sans rien faire : on recommence dans un moment.
- 19 novembre 2026 — **Le plein texte d'un backend se bâtit sans positions par défaut.** Un manifeste sans
  `fts_positions` crée ses index lucivy sans positions. Mesuré sur la base du dépôt : −29 % de place (866 → 613 Mo),
  toutes les requêtes sous 100 ms, et les mêmes réponses, documents et spans compris ; l'indexation va 30 % plus vite.
  `fts_positions: true` dans le manifeste, ou `ftsPositions: true` dans la configuration d'une entité, garde les
  positions. Un index qui existe garde son schéma jusqu'à son rebâti.
- 19 novembre 2026 — **Le cache du fournisseur se lit, et un compte absent n'est plus un zéro.** L'usage d'un appel
  au modèle dit les jetons d'invite servis par le cache (compatible OpenAI, Gemini, Anthropic, et `timings.cache_n`
  de llama-server), et dit si le fournisseur l'a rapporté. Un run dit si l'un de ses appels l'a fait.
- 19 novembre 2026 — **Une décision par le modèle profite du cache du fournisseur.** Le tour système ne porte plus
  que l'instruction et la forme de la réponse ; le texte puis les étiquettes candidates ferment le tour utilisateur.
  Deux décisions sur le même texte commencent par les mêmes octets, et le fournisseur ne relit pas le texte entier
  à chaque jeu de candidats. Un gabarit garde son propre ordre.
- 19 novembre 2026 — **Une instruction refusée pour un schéma changé recommence d'elle-même.** Hors d'une
  transaction explicite, une instruction que le moteur refuse par « Schema changed, retry: » est rejouée par la
  connexion, trois fois au plus ; au-delà, l'erreur dit les essais. Dans un `BEGIN … COMMIT`, le refus remonte tel
  quel.
- 19 novembre 2026 — **Le journal plafonné d'une commande en fond garde sa fin.** Quand le journal dépassait son
  plafond, ce que la commande écrivait pendant la troncature se perdait sans être compté, et une commande qui
  finissait à ce moment perdait la fin de sa sortie. La fin est gardée, et l'en-tête compte tout ce qui est parti.
- 19 novembre 2026 — **Une base de code sur PostgreSQL a les générations.** Comme sur rag3db, une base neuve garde
  l'histoire de ses lignes : une édition retire au rang en attente et garde ce qui est publié. La visibilité s'y
  lit par un index.
- 19 novembre 2026 — **La chauffe du service d'embarquement chauffe ce qu'il calcule vraiment.** Elle passe par le même
  découpage que les requêtes (au plus 32 textes, sous le budget d'octets), au lieu des lots de 128 que conseille
  granite. granite-278m démarre ainsi sur l'iGPU de luciepc, où un lot de 128 longs textes faisait paniquer burn à la
  chauffe (le défaut de burn reste, à l'amont).
- 19 novembre 2026 — **La pose d'un service d'embarquement se prouve contre une tolérance mesurée.**
  `scripts/embed_parity.py` embarque un jeu fixe de 100 textes seuls, puis en lots de 8, 32 et 128, et dit l'écart
  entre lots : au plus 5,5e-5 pour granite-278m, 1,3e-3 pour bge-m3 (mesuré le 19 novembre). Une pose garde le bit près
  pour un même lot, et reste sous deux fois cette tolérance entre lots.

- 19 novembre 2026 — **Une requête de recherche passe devant les lots d'index au service d'embarquement.** Une requête
  de 4 textes au plus n'attend plus que le lot d'index en cours, jamais les suivants, et pas le trou d'une rafale sur
  une carte partagée. Mesuré pendant un index : la latence d'une requête au 95e centile passe de 116 ms à 38 ms, sans
  perte de débit pour l'index.

- 19 novembre 2026 — **Le journal d'index nomme les morceaux que le modèle a coupés.** Un service d'embarquement dit
  désormais quels textes son modèle a tronqués à sa fenêtre ; le journal d'index nomme les morceaux concernés (fichier
  et ligne) : « 3 morceau(x) coupé(s) à 512 jetons par le modèle : src/x.rs:120 (morceau 4), … ». Le compteur du
  service (`truncated_texts`) ne compte plus sa chauffe, qui disait 169 coupes à chaque démarrage de granite.

- 18 novembre 2026 — **La fin d'un index dit une libération ou une dette qu'elle n'a pas pu faire.** Une erreur en rendant les réclamations d'embarquement s'ajoute à la dernière ligne du journal, avec sa table et sa cause, sans en effacer la cause de fin ; une dette restante illisible se dit « dette inconnue », plus jamais « 0 ».

- 18 novembre 2026 — **Un modèle déclaré dit `access` et `provider`.** `access` dit la manière d'y accéder (`local`, `service`, `compatible`), `provider` le fournisseur (`openai`, `vertex`, `anthropic`, `llama_server`). L'ancienne forme, `provider` pour la manière et `protocol` pour le fournisseur, se lit encore **jusqu'au 29 novembre 2026** ; un modèle qui mêle les deux est refusé, en nommant les deux mots. `/model` rend les deux mots neufs.

- 18 novembre 2026 — **Un llama-server déclaré a le budget d'un modèle local.** Déclaré `compatible`/`openai` à une adresse de ce poste (`127.0.0.1`, `localhost` ou `::1`), un modèle de langage repart après une compaction avec la moitié de sa fenêtre, et non plus avec les 100 000 jetons d'un modèle du cloud.
- 19 novembre 2026 — Dans le terminal de l'agent, la réponse que l'hôte recopie dans la boîte ne s'affiche plus une seconde fois quand elle a déjà défilé : elle se reconnaît à son champ `origin`, et non plus à un texte français.

- 18 novembre 2026 — Dans le terminal de l'agent, ce qu'on tape pendant une commande `!` ne se garde plus jamais dans sa sortie, même tapé avant que la commande coupe l'écho : un mot de passe tapé avant l'invite de `sudo` se voit à l'écran, comme dans tout terminal, mais n'entre ni dans la consigne envoyée à l'équipier, ni dans le journal.

- 18 novembre 2026 — Dans le terminal de l'agent, ce qu'il attend se dit dans le style du thème : pendant une écriture, le verbe de la cause se déforme en glyphes du thème, un caractère à la fois ; pendant une réflexion ou une lecture, la cause prend la couleur du halo. Ce qu'elle nomme (outil, cible, modèle, durée) reste lisible.
- 18 novembre 2026 — **Un fournisseur mort au démarrage est nommé.** Le refus disait « « rag3weaver-embeddings » est
  mort avant de répondre » quel que soit le programme lancé par l'activation (une fixture, un fournisseur tiers, une
  copie renommée). Il nomme désormais le programme à côté du type de service : « « rag3weaver-embeddings »
  (fournisseur.py) est mort avant de répondre (code 70) ». De même pour un fournisseur qui vit sans répondre.
- 18 novembre 2026 — **Les relations d'un premier index se posent plus vite.** Quand une seule paire d'un paquet de liens existait déjà, tout le paquet passait par le chemin lent ; seules ces paires-là y passent maintenant. La pose des liens tombe de 7,5 à 4,6 s sur rag3weaver et de 26,5 à 17,8 s sur le dépôt, et les arêtes ne changent pas. Les bases ne repassent pas.

- 18 novembre 2026 — **En Python et en C, une fonction nommée `default`, `from` ou `of` est un appel.** Les mots ignorés de Python et de C étaient ceux de JavaScript ; ils viennent maintenant du langage (les mots-clés de Python, les mots réservés de la grammaire C). Sur les fichiers Python du dépôt, 3 liens de plus ; le C ne change pas. **Toutes les bases repassent une fois** (pointeur codeparsers : les mots vides en données, `72ab7a8`).

- 18 novembre 2026 — **En Rust, `Settings::default()` est un appel, et `std::fs::remove_file(…)` n'est plus un appel du projet.** La liste des mots ignorés de Rust était celle de JavaScript : `default`, `new` ou `from` n'étaient jamais relevés. Elle vient maintenant de la documentation de std. Et un chemin de la bibliothèque standard ne se relie plus à une fonction homonyme du dépôt, même avec un appel du projet sur la même ligne. Sur rag3weaver, 287 appels de plus sont reliés et 258 liens faux disparaissent. **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 8 ; pointeur codeparsers : les mots vides en données, `34c74e4`).

- 18 novembre 2026 — **Un texte trop dense pour bge-m3 est coupé, plus de panique sur la carte.** 11 000 « é » passent
  sous le plafond de 12 000 caractères, mais font 11 002 jetons, plus que les 8 194 positions de bge-m3 : le calcul
  paniquait sur la carte et le service s'arrêtait. bge-m3 coupe désormais chaque texte à sa fenêtre en jetons (8 192)
  et compte la coupe ; le service dit sa fenêtre et ses coupes dans son identité, et `RAG3WEAVER_EMBED_MAX_TOKENS`
  la resserre.

- 18 novembre 2026 — `rag3weaver-tty --plain --demande …` attend aussi un run que l'hôte reprend juste après sa fin (une consigne arrivée pendant le dernier pas) : la fin d'une demande, c'est une fin qu'aucune ligne ne suit depuis 2 s, la tâche n'étant plus en cours.

- 18 novembre 2026 — **Une méthode définie hors du fichier de sa classe est une méthode.** `void Foo::f() {}` dans un `.cpp`, quand `class Foo` est dans un `.h` de la même source, s'écrit `method` et non plus `function` : l'arbre compact montre une méthode. Une limite : si le `.h` est indexé après le `.cpp`, dans un lot suivant, la définition reste une fonction jusqu'à la prochaine réécriture du `.cpp`.

- 18 novembre 2026 — **Un embarquement par un fournisseur `compatible` peut estimer et indexer.** Depuis la clé du
  débit par adresse et par poste, un fournisseur `compatible` (une API d'embarquement au contrat OpenAI) n'avait pas
  d'adresse : l'estimation échouait, et un index de 512 morceaux et plus finissait en échec une fois ses vecteurs
  écrits. Il dit désormais son URL de base, sans les identifiants, et son débit se note sous elle. Un débit qui ne
  peut pas se noter se dit au journal sans faire échouer l'index.

- 18 novembre 2026 — Le terminal de l'agent a ses commandes et ses options en anglais, à côté des françaises qui marchent toujours : `/help`, `/steps`, `/name`, `/task`, `/repo`, `/log`, `/clear`…, et `--repo`, `--task`, `--request`, `--log`, `--steps`… Tab complète maintenant l'argument de `/task` et de `/repo`.

- 16 novembre 2026 — **L'ordre des ex æquo tient aussi d'une base réécrite à une base neuve.** Deux résultats au même score se départagent par leur chemin, leur ligne et leur nom (une version par son identité de contenu), et non plus par un identifiant que la réécriture d'une ligne change. Une base qui a son histoire et un index neuf du même arbre rendent la même liste.

- 16 novembre 2026 — **Une fonction définie plusieurs fois donne le type de celle que le fichier importe.** `let host = hote(…); host.hosted()`, où `hote` existe dans quatre modules avec quatre types de retour, n'était relié à rien ; c'est maintenant la méthode du type que rend le `hote` importé. Sur rag3weaver, 86 appels de plus sont reliés par le type. **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 7 ; pointeur codeparsers : le module du retour, `bd13385`).

- 16 novembre 2026 — **Deux expressions d'une même ligne ont chacune leur fichier.** Deux générateurs ou compréhensions Python sur une même ligne (`print(sum(… for x in A), sum(… for y in B))`) partageaient une seule arête vers leur fichier : l'une des deux restait sans fichier dans le plan et dans `usages`. Les 27 cas de la base du dépôt (scripts Python, une lambda de spdlog) n'en ont plus. **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 6 ; pointeur codeparsers : les octets du nœud, `d754ee5`).

- 16 novembre 2026 — **Une locale d'un motif de tuple ou redéclarée dit son type.** `let (mut catalog, _store) = make_pair(); catalog.drain()` vise la méthode du bon type, et `let outil = …?; let outil = outil.bind()?;` ne perd plus le type de `outil` : chaque `let` vaut là où il est écrit. Sur rag3weaver, 245 appels de plus sont reliés et 278 passent du nom au type. **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 5 ; pointeur codeparsers : les liaisons situées, `6291939`).

- 16 novembre 2026 — **Une ligne très longue ne fait plus un morceau géant.** Le découpage du code par lignes coupe une ligne plus longue que la taille d'un morceau (1 500 caractères pour le code) en morceaux de cette taille, et le contexte autour d'un morceau ne dépasse plus cette taille de chaque côté. Un fichier de données d'une ligne faisait un morceau de 53 000 caractères, que le vecteur ne savait pas chercher. Le code ordinaire se découpe comme avant ; une base déjà indexée garde ses anciens morceaux jusqu'à la réécriture de leur contenu.

- 16 novembre 2026 — **Une recherche lexicale rend le même ordre d'une base à l'autre.** Deux résultats au même score sortent désormais dans un ordre fixé par leur identité (le contenu ou la clé), et non par la place de leurs documents dans l'index ; la coupe aux k premiers prend en compte tous les ex æquo avant de trancher. Deux bases au même contenu rendent la même liste.

- 16 novembre 2026 — **Le journal d'index dit enfin ce qui a été coupé.** La ligne de nouveautés de la coupe au plafond
  (828ff28153) promettait que le journal d'index dise combien de textes avaient été coupés : il ne le disait pas,
  l'avertissement partait sur le bus d'événements du catalogue, que le journal ne lit pas. Désormais, après chaque
  passe, le journal écrit « N texte(s) coupé(s) à M caractères avant l'envoi au service « nom » », et aussi les
  textes tronqués par le modèle à sa fenêtre de jetons, qui n'y étaient jamais dits non plus. Deux index lancés dans
  la même milliseconde ont chacun leur journal.

- 16 novembre 2026 — **Un texte trop long ne fait plus tomber l'embarquement de tout son lot.** Depuis le 15 au
  soir, un seul texte de plus de 12 000 caractères (un fichier de données d'une ligne énorme) faisait refuser par
  le service chaque lot de 512, et un index neuf du dépôt restait sans aucun vecteur. Désormais, le client coupe
  chaque texte au plafond que son service déclare, et le journal d'index dit combien il en a coupé ; le service
  coupe lui-même pour un client d'avant, au lieu de refuser le lot. Ce qui dépassait le plafond n'est pas
  cherchable par le vecteur ; la ligne, elle, reste entière en base.

- 16 novembre 2026 — **Un membre d'une bibliothèque n'est plus pris pour un nom du projet.** `process.platform`, `path.join(…)` ou `process.env` dans du JavaScript étaient reliés à un `platform`, un `join` ou un `env` du dépôt. Le membre d'une bibliothèque standard ou déclarée (provenance `builtin` ou `dependency`) ne se relie plus par le nom, et un membre lu au niveau d'un fichier garde son propriétaire. Un paquet du projet garde ses liens. **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 4 ; pointeur codeparsers : le membre au niveau du fichier, `edfc72d`).

- 16 novembre 2026 — **Les outils de recherche de l'agent se choisissent mieux.** `search_code` se dit l'outil à prendre d'abord pour une question en mots, un « où est décidé… » ou un nom incertain, et `grep_files` l'outil d'une chaîne exacte seulement (descriptions en anglais, gabarits `code` et `atelier`). Au-delà de 1 000 lignes trouvées, `grep_files` refuse sans rien rendre, en disant le total, le seuil (`refuse_above`) et les dossiers où restreindre par `path_prefix`. `git rev-parse`, `git ls-files` et `git describe` tournent sans demander d'accord.

- 15 novembre 2026 — **Le repli d'une carte pleine peut aller vers une autre activation.** Avec
  `fallback: {"activation": "<nom>"}`, l'embarqueur d'une carte pleine (ou trop lente) lance ou rejoint une activation
  déclarée, sur sa propre carte et à sa propre `address`, au lieu d'un service posé à la main. Le repli doit être le
  même fournisseur, au placement près : seuls le périphérique, l'adresse et les variables de carte peuvent différer,
  et tout autre écart est refusé au chargement, nommé. Sa recette est attestée, et l'état le dit. Sa carte se mesure
  aussi : si les deux sont pleines, l'activation principale se fait, et l'état dit les deux mesures. Un repli pris pour
  le débit qui retombe juste après son retour double sa tenue, jusqu'à `fallback_hold_max_factor` (16 par défaut).

- 16 novembre 2026 — **Une méthode héritée se trouve par le type dérivé.** `newEntry->setOID(oid)`, où `newEntry` est un `TableCatalogEntry` et `setOID` une méthode de sa base `CatalogEntry`, n'avait pas d'arête : le type lu ne cherchait que dans la classe elle-même. Il cherche maintenant dans ses bases, la plus proche d'abord, et s'abstient si deux bases portent le nom. Sur le moteur et ses tests, 804 appels de plus sont reliés par le type. `RAG3WEAVER_INHERITANCE_DEPTH` règle la profondeur (4 par défaut). **Toutes les bases repassent une fois** (`REGLES_DE_L_ANALYSE` monte à 3).

- 16 novembre 2026 — **La synchronisation d'une base à générations retrouve ses garde-fous.** Une source tronquée (plus de fichiers disparus que `maxMissingRatio`) ou vide est de nouveau refusée sans `force` ou `allow_empty`, et rien n'est retiré ; le rapport dit les fichiers et les scopes qui partent ; le plan seul ne retire rien ; un fichier écarté à la lecture (généré, minifié) quitte l'index ; et `files_unchanged` compte les fichiers que l'écrivain n'a pas réécrits parce qu'ils n'avaient pas changé.

- 16 novembre 2026 — **Un appel C vers l'API C se voit.** `examples/c/main.c` appelait `rag3db_connection_init` et les autres fonctions de `src/c_api/` sans qu'aucune arête ne se pose : une source `.c` ne relevait pas ses `#include`, et `rag3db.h` perdait ses prototypes derrière sa garde `extern "C"`. Les appelants C de l'API, et ceux qui la voient par l'en-tête, sont maintenant dans `usages` et `impact`. **Toutes les bases repassent une fois** : `REGLES_DE_L_ANALYSE` monte à 2, pour toute la pile de la résolution d'arêtes de codeparsers (homonymes, marque « nom », receveurs, appels C).

- 16 novembre 2026 — `rag3weaver-tty --plain --demande …` reste attaché jusqu'à la fin de sa demande, et son journal porte les appels d'outil : un script peut le lancer puis lire le journal. Il rendait la main en 3 s quand une demande précédente de la même tâche avait laissé sa fin dans la boîte ; il ne suit plus que le flux de sa tâche.

- 15 novembre 2026 — **Une réindexation garde l'histoire d'un dépôt** : sur une base à générations, une ligne publiée qui change n'est plus réécrite sur place, elle est fermée, et la ligne neuve naît au rang en attente avec les arêtes qui y arrivaient ; un fichier supprimé est retiré, pas effacé ; dix sauvegardes ne publient rien ; un retour au texte publié efface l'attente. Le coût suit le diff : un fichier dont le contenu et la version de l'analyse (parseur, règles, gabarit) n'ont pas changé est sauté. **Chaque base repasse une fois en entier** à sa prochaine synchronisation (environ dix fois le temps d'un index ordinaire, 45 min sur ce dépôt, tant que l'écrivain travaille fichier par fichier), puisque ses fichiers n'ont pas encore de version de l'analyse, et de nouveau à chaque montée de `REGLES_DE_L_ANALYSE`, que `check_features` exige d'un lot qui change l'analyse. La publication dit les arêtes recopiées depuis l'ouverture.

- 15 novembre 2026 — **L'agent d'une tâche ouverte se renomme** : `POST /ide/teammate/name {backend, task, agent_name}` (le `/nom` du terminal) donne un nouveau nom à l'agent sans rouvrir la tâche ; le tour suivant le dit, et il est gardé avec la tâche. `""` lui rend le nom du backend ; un nom vide d'espaces ou sur plusieurs lignes est refusé en nommant `agent_name`.

- 15 novembre 2026 — **Un modèle de langage que le binaire ne sait pas joindre n'empêche plus d'ouvrir le backend.** Bâti sans la feature `openai-llm`, un backend qui déclare `models.llm` s'ouvre quand même, comme un backend sans service d'embarquement s'ouvre en plein texte seul ; son reçu dit « modèle de langage <nom> indisponible : la feature openai-llm n'est pas dans ce binaire », et c'est le tour de chat qui refuse, avec la même phrase. Le refus « pas de forme compatible » nomme les features.

- 15 novembre 2026 — `/nom <x>` dans le terminal de l'agent nomme l'agent de la tâche (`/nom -` lui rend le nom de son dépôt), et chaque ligne de l'agent porte son nom : `[Algareth · compte rendu]` au lieu de `[équipier · compte rendu]`.

- 15 novembre 2026 — **L'agent d'un dépôt a un nom et un personnage** : `agent: {name, persona}` au manifeste du backend. Le nom (« équipier » par défaut) est celui que l'équipier dit ; la persona est un personnage bref, ses motivations et ses goûts, techniques compris, par exemple « Tu as goût aux beaux paradigmes et aux abstractions ; tu préfères une interface claire à une astuce. » (600 caractères au plus). Elle colore le ton et peut peser sur un choix de conception, sans jamais primer sur la demande ni sur une règle. Une tâche surcharge les deux, et peut les recevoir à sa création (`POST /ide/taches {…, agent_name, agent_persona}`). Un nom vide ou sur plusieurs lignes, une persona trop longue : refusés à la lecture, en le disant.

- 15 novembre 2026 — Toute tâche reçoit le socle d'un agent de code, quel que soit le manifeste de son backend : lire par 200 lignes, remplacer un texte unique ou les lignes a à b, insérer, écrire un fichier neuf, chercher par motif, lancer une commande derrière la porte ; l'outil que le backend déclare sous le même nom gagne, et une tâche sans droit d'écrire n'en reçoit que la lecture.

- 15 novembre 2026 — **Le service d'embarquement ne meurt plus en silence.** Un calcul qui panique sur la carte (une
  réservation de mémoire refusée) rend un 503 nommé au client, au lieu d'une connexion coupée. Le service répond
  ensuite à toutes ses requêtes en vol, puis s'arrête avec le code 70 en disant sa cause en dernière ligne de son
  journal. Il ne sort plus jamais avec un 0 muet que systemd relançait sans rien dire, vingt et une fois en une
  après-midi pour le bge-m3 de luciepc. Un texte de plus de 12 000 caractères (`RAG3WEAVER_EMBED_MAX_CHARS`, dit dans
  l'identité) est refusé par un 413 nommé, sans calcul. Un fournisseur lancé par une activation, arrêté ainsi, est
  dit « service d'embarquement arrêté : <cause> ».

- 15 novembre 2026 — L'équipier ne se dit plus « fait » sans rien rendre : une demande où il n'a rien laissé (ni ligne dans la boîte, ni fichier écrit, ni plan clos) est relancée une fois en citant son step_done, puis attend l'utilisateur au plus teammate.reply_wait_s avant un échec nommé ; `expects: "none"` garde une demande silencieuse. Dans une tâche, il parle comme un agent de code et à l'utilisateur, son droit d'écrire se relit à chaque ouverture, et `git status` marche sous le bac.

- 15 novembre 2026 — **Une chaîne de méthodes du projet dit son type.** `let files = self.teammate_files()?; files.append(…)`, `let c = ctx.cancellation(); c.wait_timeout(…)` ou `GraphTool::from_mermaid(t).map_err(…)?` visent maintenant la méthode du bon type, et non plus un homonyme par le seul nom. Sur rag3weaver, environ deux mille arêtes de plus sont résolues par le type ; 123 faux liens de `.collect()` partent. **Toutes les bases repassent une fois** (pointeur codeparsers : les chaînes de méthodes, `65b1a93`).

- 15 novembre 2026 — **Une méthode standard ou une variante ne va plus à l'homonyme d'un autre fichier.** `reste.is_zero()`, `results.split_off(n)`, `params.join(", ")` sur une locale dont le type ne se lit pas, et `LogicalTypeID::STRING` ou `Flow::Stop`, étaient reliés par le seul nom à une méthode ou une fonction du même nom ailleurs dans le dépôt. Ils s'abstiennent maintenant, et `usages` les montre « par le nom » sans les attribuer. Sur le moteur, 2 329 fausses arêtes partent ; sur rag3weaver, 646. Une méthode propre au projet garde son lien. **Toutes les bases repassent une fois** (pointeur codeparsers : la marque « nom », `fad1b6f`).

- 15 novembre 2026 — **Le socle d'agent de code s'agrandit** : `write` crée un fichier neuf en entier et rend son diff (un fichier qui existe est refusé, sauf `force`), et `glob` rend les chemins d'un motif (`**/*.rs`) sans ce que `.gitignore` écarte. Avec `read_file` (200 lignes numérotées, « lignes a-b sur N »), `replace_text`, `replace_lines`, `insert_lines` (qui exigent une lecture récente des lignes visées), `grep_files` et `list_files`, c'est le jeu de fichier qu'une tâche reçoit pour travailler comme un agent de code ordinaire.

- 15 novembre 2026 — **Une requête trop longue ne fait plus tomber le service d'embarquement.** Avant d'être
  embarquée, une requête de recherche est coupée à `models.embed.query_max_chars` caractères (6 000 par
  défaut), sur une frontière de mot, que ce soit en dense, en dual ou en creux. La recherche dit la coupe, et
  le plein texte garde la requête entière. Une requête de 8 000 jetons demandait 4 Go d'attention à la carte,
  et avait fait redémarrer vingt et une fois le service bge-m3 de luciepc. Un plafond au-dessus de
  `context_tokens × 3` est refusé au chargement, et l'état dit la valeur et sa source.

- 15 novembre 2026 — **Carte pleine, l'embarqueur se replie vers un service déclaré.** Une activation
  peut déclarer la mémoire qu'elle prend sur sa carte (`required_mib`, plus `margin_mib`) et un débit
  minimal (`min_throughput`, sur `window_secs`). Avec `fallback: {"service": "127.0.0.1:7879"}`, une
  carte pleine ou trop lente fait servir l'index par ce service, modèle et dimension vérifiés, au lieu
  de charger une seconde copie du modèle sur une carte déjà prise (l'index de 904 s du 14 novembre).
  Le repli tient au moins `fallback_hold_secs`, puis revient à la carte quand elle a la place. Le
  journal d'index dit d'où vient chaque passe de vecteurs, et chaque repli ou retour avec sa cause.
  L'état d'index le porte aussi.

- 15 novembre 2026 — Le terminal de l'agent ne montre plus de mot d'état nu ni de verdict de l'hôte (« attend », « fini : fait sans plan », « étape faite ») : l'animation du thème tourne dès que l'équipier part, avec à côté ce qu'il attend (« le modèle répond… (gemini-3.8-flash, 12 s) », « run_command cargo check travaille… (40 s) ») ; quand c'est ton tour, chaque thème a son animation de repos ; une phrase dit seulement ce qui change quelque chose pour toi : une fin sans réponse, une erreur.

- 15 novembre 2026 — Dans le terminal de l'agent, une demande d'accord vit dans la zone vivante, le temps qu'on y réponde : la ligne d'état la dit même quand l'équipier s'anime, `o` ou `n` y répond, et l'historique n'en garde qu'une ligne, « accordé : … » ou « refusé : … », que la réponse vienne du terminal ou de l'IDE. Deux boutons, Oui choisi d'avance et Non, se choisissent par ← → et se valident par Entrée ; la demande se dit encore une fois le pas fini. Une proposition de déclarations du dépôt principal attend de même : `o` l'applique, `n` la rejette.

- 15 novembre 2026 — Le terminal de l'agent ne garde dans l'historique que la demande et la réponse : les appels d'outil, les étapes faites et les mots de l'hôte défilent sur une ligne discrète de la zone vivante, et restent lisibles par Ctrl-O et le journal ; `--etapes` ou `/etapes` remontrent tout.

- 15 novembre 2026 — Sous tmux, Maj-Entrée passe à la ligne dans le terminal de l'agent si tmux est réglé pour les touches étendues en CSI u (`set -g extended-keys on`, `set -g extended-keys-format csi-u`) ; le terminal lit le réglage et le dit quand il manque. Le tmux par défaut garde son repli dit, Alt-Entrée.

- 15 novembre 2026 — `/model` dans le terminal de l'agent : la liste des modèles de langage que déclare le manifeste de la tâche (le défaut et celui de la tâche marqués), et `/model <nom>` pour en changer ; l'hôte garde le choix avec la tâche et son chat le suit (`GET /ide/teammate/models`, `POST /ide/teammate/model`) ; un choix devenu inconnu se voit, et le chat refuse alors en disant de choisir.

- 15 novembre 2026 — Le terminal de l'agent a ses commandes : `/aide`, `/theme`, `/tache`, `/depot`, `/journal`, `/effacer`, `/context` (le remplissage du contexte du modèle) et les autres ; `!<commande>` la lance là où tourne le terminal, dans un pseudo-terminal où `sudo` demande son mot de passe, et sa sortie part à l'équipier, sans jamais le mot de passe ; Tab complète les commandes, leurs arguments, les exécutables et les fichiers ; l'historique des envois survit à la séance.

- 15 novembre 2026 — Les thèmes du terminal et de l'IDE deviennent des palettes nommées en OKLCH (huit couleurs par thème, un mode sombre ou clair) : leurs couleurs et leur coloration syntaxique en sont dérivées par un seul script, qui part de la couleur de palette la plus proche de la teinte voulue, tient le contraste WCAG ou refuse, et rougit si un thème a été retouché à la main. Les cinq thèmes sont convertis au plus près : écart moyen du code de 0,013 à 0,029 en OKLab, fond, texte et halo inchangés.

- 15 novembre 2026 — **Plusieurs modèles de langage dans un manifeste** : `models.llm` peut être une liste de modèles nommés, chacun avec son fournisseur, sa fenêtre (`context_tokens`) et son plafond de sortie (`max_output_tokens`), un seul `default: true`, et le budget du contexte qui repart après une compaction (`restart_budget_tokens`, 100 000 jetons par défaut en cloud, la moitié de la fenêtre en local) ; la tâche retiendra un nom, `/model` les montrera. Un manifeste d'aujourd'hui, avec un seul objet, se lit comme une liste d'un modèle par défaut, nommé d'après son modèle : rien à changer. Les erreurs se disent à la lecture : nom manquant ou en double, aucun ou plusieurs défauts, une liste ailleurs que sous `llm`, un nom inconnu avec la liste des noms déclarés.

- 15 novembre 2026 — Le terminal de l'agent montre ce que l'équipier lit et écrit : chaque appel d'outil numéroté, l'aperçu coloré d'une lecture dans la langue du fichier (un .md en prose, ses blocs de code dans leur langue), une écriture en lignes retirées et ajoutées, les blocs de code de la boîte colorés, et un extrait coupé qui le dit ; Ctrl-O ouvre un appel en grand dans un écran à part, l'historique intact derrière.

- 15 novembre 2026 — Un ALTER TABLE ou un DROP TABLE attend désormais les écritures ouvertes sur sa table, comme sous PostgreSQL, au lieu de passer sous elles. Une écriture n'écrit plus jamais une ligne sur l'ancien schéma : 100 lignes sur 100 ne lisaient pas la colonne ajoutée avant le correctif. Une écriture dont la table a changé de colonnes depuis le départ de sa transaction est refusée par un message qui commence par `Schema changed, retry:`, et se recommence ; c'est le geste que rag3weaver apprendra. De deux ALTER d'une même table, le second attend le premier, est refusé ainsi, puis passe relancé (il était refusé tout de suite avant). Un CREATE_VECTOR_INDEX tient la table en partagé : il n'arrête aucun écrivain, et un commentaire ou un index ne fait rien recommencer. Le prix, à savoir : un ALTER posé pendant une indexation en masse attend la plus longue transaction d'écriture (un paquet de 2 048 fichiers, ou le chargement final des relations), soit des minutes, et les écrivains qui arrivent attendent derrière lui ; en service, l'hôte ne fait d'ALTER qu'à l'ouverture ou à une montée de schéma. Le refus d'un ALTER qui attend trop longtemps nomme l'écrivain qu'il attendait. (branche `alter-attend-les-ecrivains`, cœur c++)

- 15 novembre 2026 — La saisie du terminal de l'agent s'écrit sur plusieurs lignes : la zone du bas grandit avec elle jusqu'à huit lignes, puis défile autour du curseur ; Maj-Entrée passe à la ligne là où l'émulateur sait la distinguer (kitty, WezTerm, foot, Ghostty), Alt-Entrée partout, et la saisie vide dit lequel. Sous tmux, qui fait de Maj-Entrée une Entrée, le terminal le dit ; ticket ouvert.

- 15 novembre 2026 — Le cœur Rust du pont navigateur : le protocole de l'extension Chrome (révision 5 du fork de jackwener/browsermcp), les baux d'onglet et deux gardes avant tout envoi, un onglet rendu à l'humain ne se reprenant que par un nouvel emprunt et les passe-partout restant fermés par défaut — sans extension ni route encore.

- 15 novembre 2026 — Les couches d'un dépôt se déclarent par un seul objet, `tracked` dans le manifeste du backend : une racine (calculée, vide ou déclarée) et des sources à sélecteur git (`branch:`, `refs:`, `worktree:`, `commit:`, `tasks`), chacune avec sa parente, son histoire et sa couche de travail ; même un cas que personne n'avait prévu se déclare. Sans rien déclarer, le worktree du backend et les tâches de l'atelier ; l'état de chaque backend dit sa racine, ses couches, leur parente (déclarée, inférée parmi les suivies, égales, ambiguë), les flux multi-env (develop → staging → main) nommés au lieu d'une parente fausse, et un identifiant stable par couche qu'un renommage de branche ne change pas. Une source `refs:`, un motif large, ne s'écrit qu'à la citation par défaut : cent branches ne coûtent pas cent écritures par fetch (`844d01247`, puis ce lot).

- 14 novembre 2026 — Le terminal de l'agent, rejoué contre un vrai hôte : une demande ne s'affiche plus deux fois sous tmux, « fini » ne se lit plus à l'ouverture d'une tâche qui repart, tous les genres de la boîte sont en français (compte rendu, étape faite…), la prose passe à la ligne entre deux mots, et le repli sous mosh garde sa commande entière. Le même essai a trouvé que l'équipier rend parfois un compte rendu au lieu du texte demandé : ticket pour archi.

- 14 novembre 2026 — La saisie du terminal de l'agent devient un éditeur de ligne : flèches, Début et Fin, sauts de mot, Suppr, Ctrl-W, Ctrl-U, Ctrl-K, historique des envois aux flèches, saut de ligne par Alt-Entrée, collage d'un bloc, Ctrl-C qui vide avant de quitter, AltGr gardé sous Windows ; prouvé au clavier à travers ssh, tmux et mosh. Plus de rouge uni : la prose de l'équipier prend la couleur du terminal, avec le Markdown courant (titres, puces, gras, italique), et les en-têtes de la boîte gardent l'accent du thème, en français.

- 14 novembre 2026 — Sous mosh, qui ne garde aucun historique au-delà de l'écran, le terminal de l'agent ouvre de lui-même un journal de la séance en texte brut, et dit dans sa zone vivante la commande qui le relit (`less <journal>`) ; `--journal <fichier>` fait de même partout. Rien à installer ni à régler ; les journaux de plus de sept jours partent d'eux-mêmes. Rouge puis vert sous mosh par le harnais, la latence inchangée (47 ms).

- 14 novembre 2026 — Le terminal de l'agent se tient sous tmux et sous mosh sans rien exiger : affichage de 14 à 48 ms en médiane, écho de 0,3 à 17 ms, couleurs des cinq thèmes justes (tmux les ramène en 256 quand il le faut), position du curseur rendue par le multiplexeur. Seule dégradation corrigée : Échap attend 500 ms sous tmux, Ctrl-G arrête désormais sans délai ; l'historique perdu sous mosh a son ticket.

- 13 novembre 2026 — Le terminal de l'agent est mesuré à travers ssh, dans les deux sens entre deux machines : environ 47 ms de l'hôte à l'écran, 11 à 20 ms pour l'écho d'une frappe, 3 Ko/s pendant un pas, le redimensionnement, les touches et les couleurs des cinq thèmes vérifiés. La mesure a trouvé un vrai défaut : sans réponse du terminal à la position du curseur, la vue paniquait ; elle passe maintenant en texte brut en le disant (`ec4f7aa6d`).

- 13 novembre 2026 — On répond à l'équipier depuis le terminal : sa demande d'accord pour une commande s'affiche avec la commande, `o` l'accorde et `n` la refuse ; sa question se répond par la saisie ; et son état (attend, en erreur, fini) vient de l'hôte. Prouvé par le vrai chemin de l'hôte avec un faux équipier qui attend un accord (`212893a17`).

- 13 novembre 2026 — Le terminal de l'agent de code change d'univers : au thème « daemon » s'ajoutent **forêt**, **cyberpunk**, **cyborg** et **alien** (`--theme <nom>`), chacun avec ses trois animations (réfléchit, lit, écrit), sa palette du code et un jeu ASCII, tous tenus au même contraste minimal contre leur fond ; la règle a relevé d'un cheveu trois couleurs de daemon (`56deb90ea`, `87061f03c`).

- 13 novembre 2026 — Un index physique natif neuf BM25 sans modèle dense peut ingérer ses contenus dérivés, publier son état et reprendre le même programme après interruption, avec fermeture/réouverture et recherches vérifiées ; raccord synthétique qualifié, sans migration des index existants ni projection de coordonnées fichier (`b0a5177c0`).

- 12 novembre 2026 — Deux réponses à la même carte d'accord ne republient plus un droit ; un usage CetteFois consommé reste consommé et un refus gagnant n'est pas annulé par un oui concurrent.

- 12 novembre 2026 — Les gabarits notebook et validated-result conservent BGE-M3/1024 mais laissent chaque poste fournir ses adresses de service ; une adresse explicite dans une copie reste prioritaire. Les configurations utilisateur existantes ne sont pas migrées.

L'inverse d'une feuille de route : ce qui est **atteint**, en une ligne par grande
étape, jamais par commit. Une ligne entre quand une capacité nouvelle est sur
master, prouvée, et qu'on peut la montrer. Les titres sont faits pour accrocher,
le reste de la ligne dit en une phrase ce que ça change ; le hash dit où.

## Novembre 2026

- **15 nov. — Chaque surcharge garde ses arêtes, et un paramètre C++ son nom.** Des homonymes de même
  signature (les constructeurs d'une classe, deux `impl Trait for …`) avaient un seul identifiant dans
  l'analyse, et leurs arêtes se rabattaient sur le dernier : sur le code de rag3weaver, 65 scopes n'avaient
  pas de `DEFINED_IN`, il n'en reste aucun. Un paramètre C++ par pointeur ou par référence (`Foo* p`,
  `const Foo& r`) n'était pas lu : les signatures le montrent maintenant, et son nom n'est plus pris pour une
  référence (22 177 fausses références en moins sur le moteur). **Toutes les bases repassent une fois**
  (pointeur codeparsers : homonymes et paramètres C++, `ff82d22`).
- **15 nov. — Les appels à `T::new()` se voient enfin.** `new` était un mot vide en Rust (copié de la
  liste de JavaScript, où c'est un mot-clé) : aucun appel à un constructeur `new` n'était relevé, et
  `usages` de `Inner::new` ne rendait rien. Sur le code de rag3weaver, les relations vers un `new` passent
  de 542 à 2 894, et les scopes qui en appellent un de 403 à 1 596 ; un échantillon trouve le `new` du bon
  type à chaque appel. Le même correctif vaut pour Python et C. Et `fmt/format.h` retrouve ses parents :
  une macro seule sur sa ligne (`FMT_BEGIN_NAMESPACE`) n'ouvre plus une fonction anonyme, ses 45 fonctions
  sont de nouveau sous leur namespace, 481 scopes au lieu de 455 (codeparsers `5f7c837`).
- **15 nov. — Les dépendances d'un vieux projet Python se lisent aussi.** `setup.cfg` (`install_requires`
  en lignes de continuation, `extras_require`), `setup.py` (la liste `install_requires=[…]` écrite en
  littéral), le `Pipfile` (`[packages]`, `[dev-packages]`) et les groupes de PEP 735
  (`[dependency-groups]`, un `include-group` n'étant pas un paquet) ; côté Node, les `bundledDependencies`.
- **15 nov. — Les dépendances de conan, de CMake et d'un workspace Go se lisent.** Les `[requires]` d'un
  `conanfile.txt`, les `requires` d'un `conanfile.py`, les `find_package` et `FetchContent_Declare` d'un
  `CMakeLists.txt` deviennent des dépendances de C et C++ ; `go.work` se lit comme `go.mod` (ses `use`
  notés, ses `replace` suivis). Un port vcpkg garde son nom tel quel : le dossier de ses en-têtes n'est
  écrit nulle part dans le registre (ticket « source à trouver »).
- **15 nov. — Une méthode se retrouve sous son type, même hors de son bloc.** Un récepteur Go générique
  (`func (p *Pile[K])`) nomme son type `Pile` (deux de ses méthodes n'en avaient aucun) ; un bloc `impl`
  Rust porte le nom court du type (`impl<U> Foo<U>` → `Foo`) et le dit (`extends_type`) ; le chemin écrit
  du propriétaire est gardé (`parent_path` : `kz::Foo`, `crate::a::Barre`) ; une définition C++ hors classe
  est une méthode quand sa classe est dans le fichier ; `impl crate::Outil for …` et `impl Conv<u8> for …`
  se disent IMPLEMENTS. Deux colonnes neuves sur Scope, vides sur une base d'avant (codeparsers `5885e13`).
- **14 nov. — `~/lib/b` et `#utils/a` sont du projet, plus des paquets inconnus.** Les `paths` d'un
  `tsconfig.json` (commentaires et virgules finales compris) sont lus comme alias du dépôt, et les imports
  internes `#…` de Node sont toujours du projet ; un alias que les manifestes ne disent pas (celui de vite)
  se déclare : `workspace.imports: { javascript: { local_prefixes: ["@/"] } }` (codeparsers `7c2c5aa`).
- **14 nov. — Un `using` C# trouve son paquet NuGet.** Les `using` se lisent enfin (le parseur C# n'en
  trouvait aucun) et chacun a sa bibliothèque et sa provenance, sans rien deviner : `using
  Newtonsoft.Json.Linq` est une dépendance du `.csproj` (le paquet `Newtonsoft.Json`, au plus long
  préfixe), `System.Text.Json` est la bibliothèque standard (417 espaces tirés de la doc .NET), un espace
  du `RootNamespace` est du projet ; un alias (`using J = …`) lie son nom. Se lisent : les `.csproj`,
  `packages.config` et `Directory.Packages.props` (codeparsers `4194456`).
- **14 nov. — `import yaml` se reconnaît dans `PyYAML`.** Un nom de distribution Python qui n'est pas celui
  de son module (PyYAML → yaml, scikit-learn → sklearn, Pillow → PIL, beautifulsoup4 → bs4…) se relie
  désormais par une table livrée de 1 094 distributions (tirée de pipreqs, sa source et sa date en tête).
  Un dépôt déclare les siens dans son manifeste (`workspace.imports: { python: { distributions:
  { "mon-paquet": ["mon_module"] } } }`), qui complètent la table ou la remplacent (`replace: true`) ;
  une langue sans règles d'import est refusée en la nommant (codeparsers `85fe233`).
- **14 nov. — Un import se juge aux manifestes de sa langue, et Go garde sa casse.** Chaque manifeste
  range ses noms sous la langue qu'il décrit, et `go.mod` se lit (le module, ses `require`, ses `replace`
  suivis : un module remplacé par `../local` est du projet). Un chemin Go garde sa casse et ses tirets
  (`github.com/Azure/…` n'est pas `github.com/azure/…`), et les imports Go produisent enfin leurs
  bibliothèques. **À savoir** : une dépendance déclarée pour une autre langue ne compte plus — un nom
  que seul `package.json` déclare ne fait plus d'un import Python ou Rust une dépendance (codeparsers
  `6ba8478`).
- **14 nov. — Un fournisseur se déclare une fois, et chaque capacité le cite.** Le manifeste déclare
  ses activations par leur nom (`activations: { "bge-m3": {…} }`), et le creux, le relecteur et
  l'OCR d'un même processus le citent (`"activation": "bge-m3"`). Ils partagent un seul processus et
  une seule éviction : chacun tient son bail, et le groupe s'arrête quand plus aucun ne le tient.
  Déclaré à la demande, le groupe ne démarre qu'au premier appel d'une de ses capacités. Un nom cité
  qui n'existe pas est refusé au chargement.
- **14 nov. — Une écriture validée ne disparaît plus quand un autre COMMIT échoue.**
  Sous plusieurs écrivains, un COMMIT refusé (tampon plein, journal non synchronisé) était annulé
  après avoir rendu la main ; un autre écrivain qui validait juste à ce moment pouvait perdre ses
  lignes avec l'annulation. Le COMMIT refusé s'annule désormais avant de laisser passer quiconque
  (`373b5477b`, à poser avec la lib suivante).
- **14 nov. — Un fournisseur qui plante se relance, et le dit.** Un modèle déclaré `maintenir` ou
  `a_demande` dont le fournisseur meurt en plein travail est relancé une fois par le calcul suivant,
  identité revérifiée. Si la relance échoue, le refus dit les deux temps : parti, puis relancé une
  fois sans réponse, et quoi faire. Le `/sante` du relancé compte ses relances (`relances: n`), et un
  fournisseur qui meurt en boucle se voit sans lire un journal.
- **14 nov. — Le plein texte ne perd plus de mots, et une recherche filtrée classe comme sa vue.**
  lucivy 4.3.5 : une fusion de fond lancée entre deux commits ne fait plus perdre au dictionnaire les
  mots des segments neufs (un document pouvait sortir des recherches floues, et des exactes pour un
  mot qu'il était seul à porter). lucivy 4.3.7 : une recherche restreinte à des identifiants
  (`allowed_ids`) classe ses résultats comme si l'index ne contenait qu'eux, au lieu d'un score
  faussé par la taille du dépôt. lucivy `v4.3.5`, `v4.3.7`.
- **14 nov. — rag3weaver ne dépend plus d'openssl au bâti.** Le TLS passait déjà par rustls,
  compilé dans le binaire ; deux dépendances tiraient encore openssl sans s'en servir. Elles n'en
  tirent plus rien : un poste passé à openssl 4 bâtit à neuf sans rien installer ni régler, la CI et
  les utilisateurs aussi, et douze paquets quittent le lock (`015172c00`).
- **14 nov. — Chaque lourd a sa part de CPU, le poste garde ses cœurs libres.** `poste lourd` donne à chaque travail un quota CPU (les cœurs
  du poste moins deux, partagés entre les places) et le même nombre de fils aux outils (`CARGO_BUILD_JOBS`,
  `RAYON_NUM_THREADS`, `RUST_TEST_THREADS`) : quatre lourds qui saturent ne débordent plus des cœurs utiles,
  sans baisser le nombre de lourds.

- **14 nov. — Un modèle déclaré à la demande ne prend la carte que quand on s'en sert.**
  Un modèle déclaré `a_demande` ne démarre qu'au premier calcul : ouvrir le backend ne lance rien, une
  recherche par les mots seuls non plus. Il rend la carte après `rendre_le_bail_secs` sans appel,
  puis s'arrête après `inactivite_secs` sans usage, et revient seul au prochain index ou à la
  prochaine recherche dense, identité revérifiée. Joué en vrai par l'hôte de luciepc sur granite-107m
  (card2) : démarré pendant le premier index, la carte rendue 30 s plus tard puis libérée à 60 s,
  revenu en 4 s au second index (`2ed42a6cd`, `5286273ef`).
- **13 nov. — Un fournisseur peut tourner sur l'autre poste.** Le manifeste déclare l'hôte distant
  (`hote`, `adresse_distante`) : l'activation lance le modèle là-bas par ssh, dans une unité
  `systemd --user` à son nom (`rag3weaver-fournisseur-<modèle>-<port>`), le joint par un tunnel ssh
  et vérifie son identité. Un ssh en échec se refuse net, sans attente. Lucie voit et arrête ces
  unités d'une commande, et les services posés à la main ne sont jamais touchés. Joué en vrai du
  portable vers luciepc (`6eff62969`).
- **14 nov. — Une recherche par clé ne manque plus une ligne validée pendant un point de reprise.**
  Un lecteur admis pendant l'écriture d'un point de reprise pouvait chercher une clé déjà validée
  et ne pas la trouver ; la recherche par clé attend maintenant, quelques millisecondes au plus, que
  l'index ait recopié ses pages et publié son nouvel état (`0c41f9fe6`, posé le 14 nov.).
- **14 nov. — Un COMMIT sous un index vectoriel garde les lignes de toutes les tables.**
  Une transaction qui écrivait dans plusieurs tables, dont une portant un index vectoriel, pouvait
  valider sans les lignes de certaines tables de nœuds ; toutes sont maintenant validées ensemble
  (`2ac8080ca`, posé par la lib `0c41f9fe6` le 14 nov.).
- **14 nov. — Une fonction d'un fichier minifié n'embarque plus la ligne entière.**
  Le contenu d'un scope est borné aux octets de son nœud (codeparsers les donne désormais), et non à
  ses lignes : dans un bundle d'une seule ligne, chaque fonction portait la ligne entière, 844 Mo de
  trop dans l'index mesuré par lucivy. Le code ordinaire garde ses versions ; les fichiers concernés
  changent de versions à leur réindexation, jamais en silence.

- **14 nov. — Un même contenu ne s'embarque plus qu'une fois.** Une base de code neuve range chaque
  contenu de scope comme une version partagée : une fonction identique dans deux fichiers, ou
  déplacée, n'est embarquée qu'une fois, et chaque occurrence garde ses lignes et ses relations. Une
  base d'avant ouvre comme avant et se dit « à migrer » (`e62151904`).
- **14 nov. — Une tâche est une couche.** Chaque dépôt déclare ses branches suivies ; une tâche y
  entre avec sa parente, et l'état d'index dit les couches de chaque dépôt, leur parente et les
  ambiguës (`0672d624f`).
- **13 nov. — En JavaScript, un mot entre apostrophes ne passe plus pour un usage.** `external: ['fs',
  'path']` ne relie plus la configuration au `require("path")` : codeparsers b5c91e1.
- **13 nov. — Plusieurs onglets de l'IDE, un seul flux.**
  Les onglets d'un navigateur partagent le flux de l'hôte par un `SharedWorker`, qui porte aussi
  un petit état partagé entre les pages (la forme des fenêtres détachées à venir). Six onglets
  ouverts ne prennent plus les six connexions que le navigateur ouvre vers l'hôte ; une page
  quittée ferme son flux, et l'IDE n'attend plus une minute après quelques rechargements.

- **13 nov. — Une tâche de l'équipier ne se dit plus faite à mi-chemin.**
  Un pas qui dit ce qui reste n'est jamais une fin : le pas suivant part, la boîte le dit.
  Une request peut déclarer ce qu'elle exige (`expects: report` ou `approval`, par l'IDE, la
  boîte de Codex ou `mail send --expects`) : sans le report ou l'accord du superviseur, la
  tâche le dit et attend sans appeler le modèle.

- **13 nov. — Le grep situe ses matches dans les symboles.**
  `symbol_hierarchy_depth`, optionnel et à zéro par défaut, ajoute le contenant
  puis ses parents par inclusion de plages indexées, dans les outils code et atelier.
  Positions UTF-8, fraîcheur, ambiguïtés et coupures de budget restent visibles ;
  la couverture dépend des scopes réellement indexés. Livré d3f5a264d (source qualifiée6786c03b5).

- **13 nov. — Le manifeste peut démarrer son modèle dense local.**
  `models.embed.activation` compose une acquisition partagée, l’annulation
  indépendante d’un demandeur, le contrôle de l’identité et une chauffe visible.
  Première politique : maintenir le fournisseur local ; activation distante et
  éviction restent à venir. (`138268398`)

- **13 nov. — Les déclarations complètes ont leurs publications durables.**
  L'API Rust crée un catalogue dédié neuf, y conserve les valeurs et leurs
  reçus atomiquement et distingue l'événement de publication de l'empreinte
  du contenu. A→B→A garde trois publications ; un replay exact rend le reçu,
  historique si la tête a avancé. Identité étrangère, schéma hors contrat et
  entrée non stable refusent ; la migration physique reste une suite distincte.
  (`31db55abb`)

- **13 nov. — Le chat explore, code, teste et reprend sa tâche dans l’IDE.**
  Sur un projet Git isolé, Gemini via Vertex exécute un vrai test nommé, relit
  le diff et reprend la même conversation depuis la page. Le bac à sable lit
  les métadonnées vérifiées du worktree et l’histoire de son dépôt, sans ouvrir
  les administrations voisines ni ajouter d’écriture au principal.
  [Preuves et recette](everything-declarative/12-11-2026/04-parcours-agent-ide-vertex.md).


- **12 nov. — Explorer les symboles sans leurs corps.** `navigate_symbols` expose dans le
  dépôt et le relais contextuel de l'atelier l'arbre d'un fichier, les parents/enfants
  par plages et les membres partiels réellement indexés. Homonymes, limites, plages
  ambiguës et index périmés sont nommés ; aucune exhaustivité AST ou d'héritage promise.


- **12 nov. — Le diagnostic observe aussi les relations persistées.** Les
  configurations, définitions de relations et le journal sont lus dans le même
  snapshot RO ; une table absente reste distincte d'un ensemble vide. Le lecteur
  partage les décodeurs du catalogue et refuse les métadonnées malformées. Le
  rapprochement avec les déclarations garde séparés faits observés et fournis ;
  aucune génération durable ni migration appliquée n'en découle. (`f19e3cbb0`)

- **12 nov. — Un plan reconnaît les déclarations complètes qui lui ont été fournies.**
  L'API Rust compose et valide les schémas et configurations, lie les recettes
  et modèles fournis, puis compare leurs empreintes versionnées au plan figé. Un manque ou
  un contrat inconnu refuse ; cette correspondance ne prouve ni le stockage,
  ni une génération durable, ni le droit d'appliquer une migration. (`d2cb7f063`)

- **12 nov. — Un accord reprend la ligne montrée.** La carte accorde la ligne complète dans
  son domaine ; sa reprise lance réellement la commande, tandis qu'une variante repasse
  par la porte. Les accords à usage unique sont consommés atomiquement. Le droit de lire
  un script extérieur reste déclaré séparément dans le bac à sable. (`b42a4ada9`)

- **12 nov. — Un plan compare les gabarits depuis leurs contenus fournis.**
  L’API Rust fusionne base/local/cible, nomme les conflits et valide le candidat
  par le chargeur déclaratif fermé, sans repli disque ni extraction de cache.
  Les transitions de données ou ressources non qualifiées refusent ; acquisition,
  adoption persistée, application et politique vivant/épinglé restent à livrer.
  (`8b3fab422`)

- **12 nov. — Un aperçu décrit les correspondances de chunks et de vecteurs connus.**
  L'API de bibliothèque conserve occurrences, cellules et provenance sur des faits
  fournis ; elle refuse les valeurs manquantes ou incompatibles, sans modèle ni
  écriture. Son rapport reste explicitement non qualifié vis-à-vis des faits du
  plan : aucun transfert ni `apply` n'est livré. (`20e4e2ffc`)

- **12 nov. — Le plan de migration contrôle son historique.** `migration inspect`
  et `migration plan` exposent les métadonnées et un plan préliminaire en lecture
  seule ; fichiers modifiés, versions orphelines et états incohérents sont refusés.
  L'admission précède les effets d'ouverture et les modèles ; `apply` reste
  indisponible et aucune ancre existante n'est transformée. (`e4239bdda`)

- **12 nov. — Des occurrences distinctes partagent leur index.** Une déclaration de dérivée
  par contenu réutilise versions, morceaux et vecteurs dans une cellule ; la recherche publique
  conserve les occurrences, leurs données et leurs positions. L'option est éprouvée par
  manifeste, sans activer les gabarits code ni migrer les bases existantes. (`831d08a35`)

- **11 nov. — Les manifestes dérivés s’ouvrent selon leurs dépendances.** Racines, relations et
  schémas nommés composent leurs vrais faits ; une dérivée garde l’identité définie par sa
  dérivation, et les manques, cycles ou définitions incompatibles sont nommés avant tout appel
  à la base. Les propriétés de relations compatibles restent additives. (`bffbe5033`)

- **11 nov. — Les imports relatifs suivent la valeur exportée.** En JavaScript/TypeScript,
  les dépendances issues de `require` et des imports par défaut suivent le symbole ou la valeur
  exportée, même sous un alias local ; une instance mène à sa zone d'export. Les modules absents
  ou externes gardent une liaison marquée ; une ambiguïté reste non résolue. Ce contrat de
  navigation ne promet pas la destination lexicale du LSP : cinq désaccords stricts restent
  dans le [bilan de validation](codeparsers/11-11-2026/04-bilan-navigation-et-provenance.md),
  sans score global de précision. (`d92c0f4b7`, codeparsers `a730a9b`)

- **11 nov. — L’atelier cherche dans le dépôt choisi et suit ses commandes.** Huit lectures
  prennent une référence explicite de backend ; les liens gardent cette cible après troncature
  et rechargement. `wait_output` suit le journal d’une commande accordée. Une invocation vérifie
  et exécute la même version des déclarations, y compris pendant un rechargement. (`09e86e689`)

- **11 nov. — Le chat suit l'hôte par le flux et garde ses actions après une panne.**
  Au repos, il n'interroge plus l'hôte ; la fin du tour pousse son état, avec le même usage
  dans la barre et le récapitulatif. Une erreur conserve les outils déjà exécutés et leurs
  résultats, permet de reprendre et se dit aussi pour un tour relancé hors du formulaire.
  (`8d5139f9e`)
- **11 nov. — Un morceau partagé rend toutes ses occurrences admises.** En recherche dense et creuse,
  chaque occurrence garde ses champs et ses positions ; ses meilleurs scores sont conservés, les
  égalités ont un ordre stable et les filtres continuent de s'appliquer. (`6c4b5ad0c`)

- **11 nov. — L’archiviste garde le nom des fiches, même long.** Il choisit une candidate par un identifiant court, puis range la section sous son nom exact ; un identifiant inconnu ou ambigu est refusé. Les noms des bilans restent lisibles par le modèle. (`cx-memoire-alias-candidats`)

- **9 nov. — La jauge du contexte se voit avancer.** Elle écrit son pourcentage, garde une barre
  visible même à moins de 1 % d'un modèle à un million de jetons, suit chaque appel au modèle
  pendant le tour, et ne se perd plus quand l'hôte redémarre. (`jauge-du-contexte`)
- **9 nov. — Une commande en attente se décide d'un clic, et l'atelier repart seul.** Quand la
  porte de l'atelier met une commande en attente, la même carte d'accord la montre sous l'appel ;
  accorder laisse passer cette commande exacte, jamais toute sa famille, et la décision relance le
  tour sans qu'on retape rien ; la carte d'un chemin hors de la racine aussi.
  (`atelier-commandes-sur-la-carte`, `atelier-chemin-relance`)
- **9 nov. — Un modèle qui refuse se voit attendre, et n'attend plus sans fin.** Chaque recul du
  client du modèle après un 429 ou un 503 se dit au journal et dans le statut du chat, avec son
  numéro, son code et son attente. Chaque appelant borne ses reculs : cinq minutes pour un tour de
  chat, deux pour un pas de l'équipier, dix pour le juge. Au-delà, l'appel échoue en nommant la
  cause et le temps attendu, au lieu de dormir vingt minutes en silence comme l'équipier le
  9 novembre.
- **9 nov. — L'atelier lit et écrit, et demande avant de sortir de chez lui.** Il lit 200 lignes
  à la fois ou un scope par son nom, remplace un texte unique ou des lignes qu'il vient de lire, et
  rend le diff de chaque écriture ; un worktree qu'il a ouvert se réindexe, sans question. Hors de
  sa racine, ou sur un dépôt que l'utilisatrice tient, une carte sous l'appel demande l'accord, et
  chaque décision s'inscrit dans un journal à part. Un atelier posé neuf prend pour racine le
  dossier d'où on le lance, ou `~/rw-code`. (`atelier-lire-ecrire`)
- **8 nov. — En JavaScript, une locale ne se confond plus avec celle du bloc voisin.** Deux `const ctx`
  dans deux blocs d'un même fichier ne s'échangent plus leurs usages : codeparsers c660893, 20 faux
  sur 30 deviennent justes au juge TypeScript.
- **8 nov. — L'équipier travaille aussi sans git.** Une tâche sur un dossier qu'aucun dépôt ne
  suit s'ouvre dans une copie de ce dossier, se relit par différence et se fusionne par recopie,
  refusée si le dossier a bougé entre-temps. Une racine hors dépôt n'est plus « lue telle quelle,
  jamais écrite » : la tâche écrit dans sa copie si `task_writes` le permet. `git_init` en fait un
  dépôt, seulement si on le déclare (tache-sans-git).
- **8 nov. — Depuis le chat, l'atelier ouvre un worktree ou une branche.** La conversation de
  l'atelier (`rag3weaver-backend atelier`) attache un worktree déjà sur le poste par un mot
  (`attach_worktree`) : il est inscrit, ouvert, son index part en fond, et il paraît sous « Dépôts »
  sans recharger la page. `open_branch` ouvre une branche dans un worktree frère et l'attache, la
  commande git lancée dite dans la réponse ; `list_worktrees` dit chemin, dépôt, branche et index.
  Rien n'est cloné ni poussé. (8258fe56b, `atelier-open-branch-2`)
- **8 nov. — Un quota ne fait plus tomber le chat : une minute, puis le double.** Le client du
  modèle suit la règle de Lucie après un refus : une minute, puis le double à chaque refus, au plus
  dix minutes, cinq reprises au plus. Un 503 attend comme un quota, et le 429 que Vertex rend au
  milieu du flux se rejoue tant que rien n'a été écrit dans le chat. L'erreur finale dit ses reprises
  et son temps en minutes. Le chat, le juge des bilans, l'archiviste et l'équipier suivent la même
  règle ; le client Anthropic garde la sienne. (`llm-reprise-2`)
- **8 nov. — Ce que j'ai cherché se retrouve, avec ce que j'ai ouvert.** La vue « mes recherches »
  de l'IDE rend les recherches faites dans un dépôt, les miennes et celles des agents sous leur nom,
  la plus récente d'abord. Pour chacune, elle montre ses premiers résultats et ceux que j'ai cliqués,
  avec de quoi la relancer : tout est lu dans le journal des runs, où le clic se range aussi. Au
  passage, le journal ne perd plus un ajout fait pendant l'écriture d'un run, et un clic ne rajeunit
  pas un vieux run (`932197e79`).
- **8 nov. — Le score d'une ligne dit d'où il vient.** Une ligne fusionnée nomme le signal qui l'a
  placée (`bestSignal`) et l'échelle du score de son morceau (`chunk.signal`). Le morceau montré est
  celui du signal qui pèse le plus dans son rang, et non plus celui du plus grand score brut : un
  bm25 à 1,6 au troisième rang ne l'emporte plus sur un dense à 0,4 au premier (`d540813d5`).
- **8 nov. — Le code du chat a les couleurs du dépôt.** L'hôte colore un bloc par les grammaires
  tree-sitter qui indexent déjà le code (Rust, C++, TypeScript, Python, et six autres), pour
  l'IDE et le terminal à la fois. highlight.js ne sert plus que de repli (`POST /highlight`,
  coloration-b).
- **8 nov. — Une base dont le journal recopie puis met à jour une relation se rouvre.** Une
  transaction qui supprime une arête, la recrée par un COPY de relations, puis la met à jour, ne se
  rejouait plus après une mort : « not found in the forward direction », et la base de code de
  Lucie ne s'ouvrait plus. Le rejeu retrouve maintenant la relation du COPY par son décalage
  définitif. Le journal réel de cette base se rejoue, avec 106 612 relations dans chaque sens et
  aucun écart.
- **8 nov. — Une base rouverte retrouve toujours son index vectoriel.** Toute ouverture charge les
  extensions notées à côté de la base, même quand le journal est vide : une mort juste après un
  point de reprise laissait l'index attendre son extension. Une extension qui ne se charge pas ne
  passe plus inaperçue : la base s'ouvre, et SHOW_INDEXES le dit dans `extension_error`, avec le
  chemin et la cause.
- **8 nov. — Une phrase collée retrouve son fichier, et le surlignage ne demande plus rien à l'hôte.**
  Avec lucivy 4.3.2, une ligne de plus de 100 caractères copiée d'un fichier le ramène premier par
  le signal exact, et une requête entière garde son flou jusqu'à 900 caractères (mesuré : 464 ms à
  933) ; le surlignage des morceaux du seul vecteur lit les positions que le service bge-m3 rend,
  sans tokenizer ni modèle dans l'hôte (139 ms pour trois textes) ; et un second processus qui veut
  écrire une base déjà tenue est refusé en nommant l'écrivain en place (`80ca3d561`, `617de00da`,
  `0a8ff53d9`, `217dd10a1`).
- **8 nov. — Les fichiers d'annulation du dataflow ne s'accumulent plus dans `/tmp`.** À la fin d'un
  graphe, tous les fichiers `*.undo.json` de plus de 64 Ko sont supprimés avec le reste, au lieu d'être
  gardés par défaut (seules les migrations `migration-…` conservent leur annulation pour un retour
  arrière ultérieur) : la fuite de mémoire vive en `/tmp` est étanchée (`b860ea2dd`).
- **8 nov. — Un gros COPY de blobs tient dans un petit tampon.**
  Un seul COPY de 112 blobs d'1 Mio échouait sous 256 Mio de tampon (« buffer pool is full ») ; un
  COPY se borne désormais en octets, vide ses lignes au disque par lots sans attendre un groupe
  plein, les ajoute à la tête née du même COPY par copie-échange (jamais sous les pieds d'un lecteur),
  et écrit son journal par tranches : le COPY passe, forcé comme journalisé, annulé sans laisser une
  page (`dadbf0041`).
- **8 nov. — Un backend dont la racine a disparu le signale nommément.** Quand le dossier racine d'un
  workspace n'existe plus sur le disque (worktree supprimé), `daemon_status` porte « racine absente : <chemin> »
  et chaque outil touchant les fichiers, `index` et `estimate` compris, rend « la racine du dépôt a
  disparu (<chemin>) : rouvrez le backend sur une racine qui existe » plutôt qu'une erreur brute
  d'entrée-sortie (`b0a9ab6e5`, `1b9983b59`, `1de9c908c`).
- **7 nov. — Un appel dans une garde de `match` compte comme usage.** En Rust, `motif if
  verifie(x) =>` : la garde est du code, et son appel apparaît dans `usages` et `impact`. Avant,
  codeparsers prenait la garde pour une partie du motif et l'appel n'existait pas pour l'index
  (`vue_portee`, appelée seulement là, avait 0 usage) ; sur code-lourd, 19 appels de plus et
  aucune locale prise pour un appel. codeparsers 672a0d5.
- **7 nov. — Le flux de l'IDE ne perd plus un événement quand deux relais se croisent.** Un id
  tiré hors du verrou laissait un relais envoyer N+1 avant N, et N se perdait en silence : un
  jeton manquant en direct dans le chat (1 à 3 tours sur 8 sous un modèle rapide), une ligne de
  boîte de l'équipier. L'id se tire désormais sous le verrou des abonnés ; un témoin à quatre fils
  (8 000 publications, 7 577 reçues avant) le garde. Et la barre range chaque backend par la
  nature que son gabarit déclare (`kind`), son dépôt et sa branche lus dans son espace de
  travail. 23757c7d6.
- **7 nov. — Le chat tient l'ordre d'arrivée.** Une réponse de l'agent se lit comme elle
  s'est faite : mots, appel d'outil, mots, appel, réponse finale, pendant le tour comme après un
  rechargement ; chaque outil déplié dit son heure, sa durée et son résultat entier ; la liste
  ne saute plus pendant le flux, ni quand on déplie un outil ; le chat prend tout l'écran.
  3d09b32c2.
- **7 nov. — La barre parle en dépôts.** « Dépôts », « Mémoire », « Projets » : chaque dépôt
  attaché nommé par son dossier et sa branche, les worktrees de ses tâches par la leur ; sous
  chaque conversation, la jauge du contexte (le dernier appel sur le contexte du modèle déclaré),
  ses compteurs et les fiches écrites pendant la conversation ; les runs à rôle (l'archiviste…)
  dans les tâches de fond. Le mot « backend » ne paraît plus à l'écran du chat. 3d09b32c2.
- **7 nov. — Le journal de l'hôte dit l'heure et l'auteur.** Chaque ligne de `daemon.log`,
  celles du moteur comprises, commence par l'heure ISO à la milliseconde, et une ligne née d'un
  appel nomme son client, sa session et son dépôt : une rafale s'attribue. Le relais ne fige
  jamais l'hôte (sortie non bloquante, patience mesurée, lignes perdues comptées). 3d09b32c2.
- **7 nov. — Deux écrivains à la fois ne mêlent plus leurs nœuds sous un COPY de relations.**
  Une transaction qui créait des nœuds puis les reliait par un COPY pouvait, si une autre validait
  des nœuds dans la même table entre-temps, relier en silence les nœuds de l'autre (1000 → 1001 lu
  2000 → 2001) ; le COPY verse désormais les nœuds de la transaction dans leur table avant de
  résoudre la moindre clé, comme un COPY de nœuds le faisait déjà (`c4f8610b9`).
- **7 nov. — La recherche surligne ce que le vecteur a trouvé, filtre juste et ne s'étouffe plus sur un prompt collé.**
  Un morceau trouvé par le seul dense porte ses passages surlignés par bge-m3, calculés à
  l'affichage et nulle part ailleurs (0 appel à l'indexation, prouvé par un service qui compte) ;
  un filtre ne laisse plus passer dans le signal creux les morceaux d'un autre document ; et une
  requête de 200 mots répond en 80 ms au lieu de 26 s, en disant ce qu'elle a coupé (`c5a40d376`,
  `aaf475d4b`, `77d0b0676`).

- **7 nov. — Une lecture survit à l'échec d'une de ses instructions.** Dans une transaction en
  lecture seule, une instruction qui échoue (un index absent, un graphe projeté absent) ne fait plus
  refuser les suivantes : les signaux d'une recherche sous instantané ne tombent plus ensemble.
  Une interruption arrête aussi toute la requête, CREATE réécrit compris ; `17a094d12`.
- **8 nov. — Le code se colore dans le terminal, avec les couleurs de l'IDE.** Un bloc de code
  de l'agent est colorié par l'hôte (`POST /highlight`, les grammaires tree-sitter de codeparsers)
  et peint dans la palette du thème « daemon », lu par le terminal (l'IDE ne le lit pas encore)
  (`templates/themes/daemon.json`). Le bloc se tient entier jusqu'à sa clôture, se replie à la
  largeur sans perdre un caractère, et reste lisible sans couleur quand la langue est inconnue.
  f2e6e2521, 3696a71cb.
- **7 nov. — Le Daemon Code Summoning dans le terminal.** Quand l'agent réfléchit, des runes
  tournent autour de ⛧ (« INVOCATION DU CODE… ») ; quand un outil lit, un symbole alchimique se
  transmute ; quand il écrit ou exécute, le Glitch tire « COMPILING CURSE » et ses frères. C'est
  la page de Lucie (ragforge, déc. 2025) refaite en cellules, l'animation choisie selon ce que
  l'outil a le droit de faire. Elle tient sous ssh : 60 à 158 octets par image, au plus 1,6 Ko/s,
  et se dégrade en 256 couleurs, en 16, sans couleur et en ASCII. La vue dit le dépôt, la branche
  et la tâche, et ne montre que son worktree. f66e72d31.
- **7 nov. — Le terminal de l'agent de code a sa vue.** `rag3weaver-tty`, sans option, ouvre une
  vue inline en ratatui et crossterm, la même sous Windows, macOS et Linux. Le texte de l'agent
  coule dans l'historique du terminal (défilement, copie, tmux), et seules trois lignes se
  redessinent en bas : la ligne en cours, l'état (réfléchit, agit, attend), la saisie. Entrée parle
  à l'équipier, Échap l'arrête. L'écran se redessine au plus 30 fois par seconde : 28 octets par
  token mesurés, bornés à 35, pour une connexion ssh. 1fc1e24bd.
- **7 nov. — L'équipier se suit depuis un terminal, même à travers ssh.** `rag3weaver-tty --plain`
  lance une demande à l'agent de code et écrit au fil de l'eau ses tokens, ses outils et sa boîte.
  Après une coupure, il reprend là où il en était, sans perte ni doublon (`Last-Event-ID`, prouvé
  par un flux coupé au milieu d'un run). Il tourne là où est l'hôte, ou sur une autre machine par
  un tunnel. C'est un crate à part, sans le moteur, et chaque outil dit maintenant ce qu'il a le
  droit de faire (`policy` dans `describe`). cfc5cc8e8.
- **7 nov. — Chercher sans avoir ouvert un dossier.** La recherche de l'IDE se tape dès
  l'accueil, sur chaque backend, sur tout l'index ; depuis l'arbre, le dossier courant la
  suit comme un filtre (le `scope` de `search_code`), effaçable d'un clic. Avant, la saisie
  n'apparaissait qu'une fois un dossier ouvert, et ce dossier ne filtrait rien. e2d5c9c88.
- **7 nov. — L'IDE entier est en React.** Après le cadre, le chat, le fichier et la
  recherche du 25 octobre : les tâches de l'agent et leur relecture avec l'équipier, les
  bilans de session par fiche, l'arbre, le symbole rendu section par section, le graphe, le
  DAG vivant, les réactions, les propositions, et l'édition dans la page (CodeMirror du
  magasin embarqué, Ctrl-S sur l'empreinte lue, le conflit montré par sa différence).
  L'ancienne page n'a plus de vue à elle. 43d7489ee, 08d10babf, 7897d2c96, a4ffea748.
- **7 nov. — Un lecteur ne voit plus jamais les lignes d'une écriture annulée.** Un bloc
  entièrement annulé perdait ses versions, ce qui voulait dire « tout validé » pour un lecteur
  qui le tenait encore : 10 + 2 048 relations d'un COPY annulé comptées, 4 fois sur 60. Les
  lignes annulées sont désormais marquées telles, au grain du bloc, nœuds et relations ; `42b85dc62`.
- **7 nov. — Un appel C++ ne voit que ce qu'il inclut.** La résolution suit les #include
  et la déclaration d'en-tête : 299 arêtes justes sur 300 au juge clangd, et une mention
  qui s'abstient se dit au lieu de passer pour devinée. 9fd37ab1c.
- **7 nov. — La mémoire parle à Claude Code.** Des hooks posés dans une seule session
  d'essai, par un script qui crée son worktree : à l'ouverture, qui tu es et ce que tu
  tenais ; à chaque tour, les fiches en rapport, ce que d'autres ont changé dans les
  fichiers que tu touches, le dernier verdict du juge ; après une compaction, le défi
  d'identité ; à 70 % du contexte réel, la suggestion de ranger. Jamais bloquant, muet
  sans hôte. d42787227.
- **7 nov. — Claude parle au trait Llm.** L'API Messages d'Anthropic derrière le même
  trait que Gemini et les modèles locaux, en HTTP brut, réflexion adaptative rejouée,
  refus nommé ; un seul point de branchement rend tout modèle au reste du runtime.
  L'équipier et le juge peuvent désormais tourner sur Claude comme sur Gemini.
  90df16ace.

## Octobre 2026

- **26 oct. — La recherche surligne ce qu'elle a trouvé.** Les positions rendues par
  lucivy arrivent jusqu'à la page, au caractère, dans l'extrait de chaque résultat ; et
  l'avertissement « rien de probant » se tait quand la phrase entière a été trouvée.
  11d350001.
- **26 oct. — Un graphe dans un graphe, une fois par élément.** Un manifeste cite un
  sous-graphe par fichier et l'appelle une fois ou par élément d'un flux, les résultats
  rendus dans l'ordre ; et un flux qui ne se lit pas en une seule valeur est refusé au
  lieu de se taire. 1bd1be619.
- **26 oct. — Créer l'index vectoriel ne fige plus les écritures.** Le CREATE pose
  l'index, attend seulement les écrivains déjà ouverts, puis bâtit pendant que les
  autres écrivent ; un index en construction ou incomplet se dit dans SHOW_INDEXES,
  et la recherche répond par les mots en attendant. Format 43. 97daa41ed.
- **25 oct. — L'IDE change de peau.** Le cadre et le chat passent en React, shadcn/ui
  et lucide : mode sombre, les backends en cartes avec leurs tâches, le chat de la
  tâche et de l'atelier sur la forme du deck builder avec les outils dépliés pendant
  le flux ; le bundle est bâti par le dépôt, embarqué et vérifié, jamais chargé
  d'ailleurs. Les autres vues suivent une à une. 81fdc2f40.
- **25 oct. — La recherche trouve ce qu'on a sous les yeux.** La requête entière part
  à lucivy telle quelle, en parallèle d'une recherche par mots, et l'exact passe
  devant la fusion : « Vue d'ensemble » rend sa page aux rangs 1, 2, 3 au lieu
  d'être absente des trente ; les questions en langue naturelle gardent leur
  3 sur 3. 159087582.
- **25 oct. — L'équipier.** Un agent qui accomplit une tâche seul, par pas, et parle à
  un superviseur par une boîte aux lettres ; un plan quand il le faut, le hors-plan en
  proposition, et le juge à chaque pas ; sur Gemini, la doc d'un dépôt de test en un
  pas, notée 10. 0ab04909a.
- **25 oct. — Un lecteur ouvert avant le point de reprise n'est plus attendu.** Il relit
  sa génération, gardée à côté jusqu'au point de reprise suivant ; un lecteur nouveau voit
  l'écriture. f8c0c8df0.
- **25 oct. — Le juge juge aussi les outils.** À chaque run, un second rapport
  par outil appelé : un défaut typé par genre (paramètre manquant ou mal nommé,
  retour bavard, hors sujet, incohérent…) et par gravité, avec sa preuve exacte et
  la forme corrigée, rangé dans la fiche de l'outil ; sur Gemini, sept défauts
  nommés sur un outil volontairement faux. bdf5929c7.
- **25 oct. — Un point de reprise ne fige plus les lecteurs.** Une lecture entre,
  lit le dernier validé et rend la main pendant qu'il s'écrit ; les blocs neufs se
  bâtissent hors des verrous et s'échangent sous eux. 66994dd43.
- **25 oct. — Le juge de session.** Un agent relit chaque tour du chat par son
  journal, rejoue ce qui lit, vérifie le résultat, se compare aux sessions
  précédentes de la même tâche et range sa fiche de bilan dans la mémoire ; sur
  Gemini, il nomme la description d'outil qui a trompé l'agent. 4b64a98e5.
- **25 oct. — Le chat code dans une tâche.** Sur un dépôt étranger ouvert dans
  l'IDE, le chat lit, cherche et édite le code sur la branche de la tâche, jamais
  ailleurs, et mène à sa relecture ; Gemini le fait en 13 s. ae127d634.
- **25 oct. — Un dossier qui dit de quoi il parle.** Chaque fiche porte la version
  de son format, et le dossier un `manifeste.yaml` : une base reconstruite après un
  changement de format relit les anciens fichiers par leurs migrations, ou dit
  lesquels elle ne sait pas lire. bea6c6a98.
- **25 oct. — La recherche de l'IDE montre ce que l'hôte sait.** Chaque résultat
  avec son score et ce que chaque signal en a dit, l'état de l'index en tête, les
  replis en pied, et une case pour faire relire. b0e5451d0.
- **25 oct. — Une recherche ne peut plus tuer un écrivain.** Le plein texte (lucivy
  4.3.1) ne panique plus sous validations concurrentes, et une recherche en échec
  ne fait plus perdre d'écriture ni geler une validation ; prouvé 80 fois sur 80
  sous charge et par le témoin réel à quatre lecteurs. 277f8b11d.
- **25 oct. — La mémoire dans des fichiers qu'on lit.** Chaque fiche est un fichier
  Markdown (son état, ses ancres, ses apports) ; si la base se perd, elle revient
  entière de son dossier. d72bcf5e0.
- **25 oct. — Le plan d'un fichier dans l'IDE.** La fiche d'un fichier montre ses
  scopes imbriqués, rendus par l'outil `outline` en JSON (un Plan déclaré), chacun
  vers sa ligne et son symbole, avec ce que vaut cette lecture. cbb41d312.
- **25 oct. — La recherche dit ce que chaque signal a trouvé.** Chaque résultat porte
  le score et le rang du plein texte et des vecteurs, et `search_code` rend sa méta
  en JSON. 0427c2c2d.
- **25 oct. — Les grammaires d'aujourd'hui.** tree-sitter 0.27 : les enums du pont
  cxx rattachés à leur module, des fermetures devenues fonctions nommées, les
  méthodes de snappy à leur classe ; un recul connu dans fmt, en ticket. dfb706b07.
- **25 oct. — Les requêtes préparées ne s'accumulent plus.** Une préparation porte
  sa requête et la rend en mourant : quatre lecteurs d'un vrai dépôt tiennent dans
  7,1 Go au lieu de 30. fb4d83b65.
- **25 oct. — Un service revenu est repris sans rien redémarrer.** Un backend
  ouvert sans son service d'embarquement le retente, et se rouvre de lui-même dès
  qu'il répond : les vecteurs reprennent, l'avertissement dit toujours le vrai.
  2a5a38cb0.
- **25 oct. — Aucune écriture du plein texte ne se perd en silence.** Chaque index
  finit par compter ses documents contre les lignes de la base et dit l'écart, au
  journal et dans l'état. 4d1a4a0ee.
- **25 oct. — La recherche tient sans ses vecteurs.** Pendant un rattrapage,
  `search_code` répond par les mots et dit pourquoi le dense manque, au lieu de
  tomber en entier. c46561dd0, 1f9c07c7b.
- **25 oct. — Ce que la mémoire sait d'un fichier.** `cited_by` rend les fiches qui
  citent un chemin, un dossier, un symbole ou un source, avec leurs ancres et les
  apports qui le citent, lues par un Plan déclaré, sans Cypher. 665a4a97c.
- **25 oct. — Trier par ce qui arrive aux liens.** Un Plan déclaré s'ordonne par le
  max ou le min d'un champ des lignes liées (les fiches par la date de leur dernier
  apport), en Cypher comme en SQL. beda3d8f1.
- **25 oct. — Les colonnes gardent leur numéro pour toujours.** Une colonne
  supprimée laisse un trou à sa place au lieu de faire renuméroter les autres au
  point de reprise : plus rien ne bouge sous les plans liés, première marche vers
  un point de reprise qui ne fige personne (V2-1a, format 42). 4ae060070.
- **25 oct. — Le raisonnement se règle et se compte.** Chaque modèle déclare son
  budget de raisonnement (`models.llm.reasoning`, absent par défaut), et un run
  compte à part les jetons de raisonnement et ceux servis par le cache. 527a0356c.
- **25 oct. — Le chat de l'atelier est dans l'IDE.** Un agent qui lit tes gabarits
  et propose une modification depuis le navigateur, rien d'écrit avant Appliquer,
  jugé sur le résultat avec Gemini (« fais compter le compteur par cinq » : l'aperçu
  compte par cinq). a13b22417, 657e568bd, a454d6a91.
- **25 oct. — Le premier outil sans une ligne de Cypher.** `outline`, le plan d'un
  fichier, est un Plan déclaré en JSON, rendu dans les deux dialectes (rag3db et
  PostgreSQL) par l'IR du Plan. 89b5d5dbd, sur c1e6f7dd1.
- **25 oct. — Une base de fiches se reconstruit depuis ses fichiers.** Les fiches
  mémoire vivent en fichiers lisibles (en-tête YAML, bloc machine invisible), et la
  base perdue se refait depuis le dossier, à parité champ à champ. 80ea996c2.
- **25 oct. — Tout run s'arrête d'un clic.** Un seul chemin d'arrêt pour tout ce qui
  tourne dans l'hôte, et le bouton Arrêter dans le DAG vivant. 95c985c4c.
- **25 oct. — Un oracle juge nos relations.** clangd compare 300 relations C++ à la
  vérité du compilateur : 73 % justes, 5 % fausses sous le juge strict (218 et 14 sur 300), et on sait lesquelles. Rust,
  TypeScript, Python, Go suivent. Branche codeparsers/oracle-clangd.
- **25 oct. — lucivy ne se fige plus.** Le plein texte ne se bloque plus sous lecteurs
  et réindexation, et un shard sans écrivain ne perd plus d'écriture ; pris par une
  branche en attendant la publication. 44c790c20.
- **25 oct. — Le moteur sait quel moteur a écrit la base.** La version du format de
  stockage est posée à la main, l'en-tête et le journal nomment le bâti, et un
  moteur trop vieux refuse en le disant. f84433579.
- **19 oct. — Les lectures n'attendent plus la montre.** Pendant qu'un index
  s'écrit, chaque lecture voit l'état d'avant en moins de 100 ms (max 97 ms contre
  1 282). L4, L5, L6 : a5a6e63e6, b2ee0f8ce, 27f80e5ed.
- **19 oct. — Une relation douteuse se compte et se dit, elle ne se devine pas.**
  Plus de résolution par le fichier, le dossier ou le nom seul : la mesure sur une
  vraie base a refusé trois fois l'inverse. 9b22ef8d7, c2ef8a941.
- **19 oct. — Le catalogue dit tout.** Chaque chemin d'écriture publie sur le bus,
  en JSON, ce qu'il a réellement écrit. 60348d75a.
- **18 oct. — Le DAG vivant dans le navigateur.** Les nœuds s'allument pendant
  qu'un outil tourne, les composants React se rechargent à chaud, et le même IDE
  s'ouvre sur un dépôt étranger. C1, C2, lots 3 et 4, c15915b73.
- **18 oct. — L'hôte dort et se réveille.** L'hôte vit derrière une socket
  systemd : il s'arrête après 30 minutes d'inactivité et revient à la première
  connexion, avec un jeton stable. 
- **12 oct. — Un magasin de libs.** Chaque lib du moteur a son dossier, les
  binaires la trouvent par RUNPATH, et `courant` se pose seulement après le
  verdict.
- **11 oct. — Plusieurs écrivains par défaut.** Le moteur accepte plusieurs
  écrivains, avec des verrous nommés ; le second attend au lieu d'échouer.
  0a630b764.
