# La borne de mémoire du journal d'un COPY — une page avant le code

5 octobre 2026, session cœur C++. Étape 4 du chargement en masse journalisé (page 04), lot 2 ;
le lot 1 (les statistiques d'un `COPY` dans sa transaction) est codé à part. Conception
acceptée par l'orchestration le 5 octobre, avec quatre conditions reprises au §5.

**[lu]** : vu dans le code, à `eb2d78e46` (cartographie par un agent, non rejouée).
**[déduit]** : tiré de cette lecture.

## 1. Ce qui manque

Avec `CALL force_checkpoint_on_copy=false`, un `COPY` écrit ses lignes au journal de sa
transaction au lieu de forcer un point de reprise. Avant d'en faire le défaut, il faut que ce
journal soit borné.

- Le journal d'une transaction (`LocalWAL`) est une suite de pages de 4 Kio prises par `malloc`,
  comptées dans le tampon comme non évictables, sans borne ni réglage. Au bout : « Unable to
  allocate memory! The buffer pool is full » **[lu]**.
- Le format est verbeux : chaque valeur réécrit son type ; un vecteur `FLOAT[N]` y est N valeurs
  **[lu ; l'amplification est déduite, à mesurer]**.
- Le rejeu n'est pas borné non plus : il garde toute une transaction dans son stockage local
  jusqu'à son `COMMIT` **[déduit]**. Rejouer un `COPY` demande la mémoire de le faire.
- Le fichier du journal n'a ni identifiant de transaction ni longueur d'enregistrement ; le
  rejeu suppose les transactions contiguës **[lu]**.

## 2. Trois voies

- **(A) Le repli.** Au-delà d'un seuil, le `COPY` cesse de journaliser et sa transaction
  devient une transaction à point de reprise forcé.
- **(B) Le débordement.** Le journal local déborde dans un fichier par transaction, recopié
  dans le journal au commit. Format inchangé ; mais la recopie se fait sous les deux verrous
  du commit (tous les autres attendent), le disque écrit deux fois, et le rejeu reste sans
  borne.
- **(C) L'écriture directe.** Le journal local rejoint le fichier du journal avant le `COMMIT`.
  Une seule écriture ; mais il faut des identifiants de transaction, un rejeu en deux passes,
  un enregistrement d'abandon, un point de reprise qui ne tronque pas un journal portant une
  transaction ouverte : le format et le rejeu changent.

**Retenue : (A).** (B) et (C) restent ouvertes pour plus tard.

## 3. Pourquoi (A)

Depuis `37608cf4b`, une transaction forcée n'écrit **rien** au journal : elle est durable par
le seul point de reprise que sa validation impose, entière ou pas du tout — propriété déjà
témoignée (`ForcedTransactionTest`). Le repli s'appuie dessus :

- au franchissement, le `COPY` appelle `setForceCheckpoint`, le journal local est vidé
  sur-le-champ (la mémoire est rendue), et plus rien n'y est écrit jusqu'à la fin de la
  transaction ;
- un petit `COPY` — les paquets de rag3weaver — reste journalisé, sans point de reprise ;
- un très gros `COPY` retrouve le comportement d'aujourd'hui, là où un point de reprise coûte
  proportionnellement le moins ;
- la borne vaut aussi au rejeu : aucun `COPY` journalisé ne dépasse le seuil ;
- ni le format ni le rejeu ne sont touchés.

PostgreSQL fait de même en esprit : sous `wal_level=minimal`, un `COPY` dans une table créée
par la transaction n'écrit pas de journal, et la table est synchronisée à la validation.

Ce que (A) ne fait pas :
- une transaction forcée attend le départ des autres à sa validation (inchangé) ;
- les insertions ordinaires restent sans borne : elles sont journalisées **au commit**, en plus
  de leur stockage local **[lu]** — hors de ce lot ;
- un `COPY` qui écarte des lignes (`IGNORE_ERRORS`) reste forcé, comme aujourd'hui.
- sous plusieurs écrivains (mode éteint hors du banc), deux transactions repliées en même
  temps s'attendent l'une l'autre à leur validation, jusqu'au délai, qui en annule une. Deux
  `COPY` forcés le font déjà ; le repli le rend possible pour deux transactions qui ne le
  prévoyaient pas en commençant (remarque du banc à la relecture).

## 4. La forme

- **Le réglage** : `CALL copy_journal_threshold=<octets>`. Défaut relatif au tampon — un
  huitième, plafonné à 256 Mio, **à confirmer par la mesure** : exigence de l'orchestration,
  sous le tampon de 4 Gio du produit un paquet de 2 048 fichiers reste journalisé avec une
  marge d'au moins ×2. La mesure (un paquet réel à 2 048 et à 512 fichiers, et ce qui y pèse :
  lignes, chaînes, vecteurs) précède le choix du défaut ; si le défaut proposé ne donne pas la
  marge, un autre est proposé plutôt qu'un repli silencieux.
- **Ce qui est compté** : la taille du journal local de la transaction (`LocalWAL::getSize`),
  tout compris — un second `COPY` s'ajoute au premier, et aux écritures ordinaires déjà là.
- **Où** : dans les deux écritures du `COPY`, `NodeTable::logInsertedRowsToWAL` (nœuds, en fin
  de `COPY`) et `logRelsToWAL` du partitionneur (relations, par paquet, sous son verrou
  d'ordre). Le test se fait par lot de lignes : le dépassement est d'au plus un lot.
- **Le franchissement se voit** : un compteur par base des `COPY` repliés sur le point de
  reprise, lisible par une fonction (`CALL` à nommer au code), que la mesure de rag3weaver
  imprimera.
- **Après le repli** : `Transaction::shouldLogToWAL` reste vrai (les opérateurs s'en servent),
  mais la validation d'une transaction forcée n'écrit rien ; pour ne pas remplir de nouveau la
  mémoire, les écritures au journal local d'une transaction forcée sont sautées à la source
  (un test dans `LocalWAL`, à voir au code).

## 5. Les témoins

Chaque témoin « mort » tue le processus base ouverte ; dans le cas journalisé il exige un
`.wal` non vide (sinon rien n'est rejoué et le témoin ne prouve rien).

1. Un `COPY` sous le seuil : journal non vide, aucun point de reprise ; mort ; le rejeu rend
   toutes les lignes.
2. Un `COPY` au-dessus (seuil abaissé par le réglage) : la mémoire du journal local est rendue,
   le point de reprise est fait, le fichier du journal est vide, le compteur vaut 1 ; mort après
   la validation : tout est là.
3. Le franchissement en cours de transaction — ce que la transaction avait déjà au journal
   local avant le seuil : un `CREATE`, un `ALTER`, un premier petit `COPY`, puis le gros.
   Validée ; annulée ; morte avant la validation (rien ne revient, pas même à moitié) ; morte
   après (tout est là, par le point de reprise seul).
4. Deux `COPY` dans une transaction, le second seul au-dessus du seuil.
5. Un `COPY` de relations qui franchit le seuil au milieu de ses paquets.
6. Le seuil compté sur la transaction : deux `COPY` chacun sous le seuil, ensemble au-dessus.

Relecture croisée par le banc (reprise et commit) avant le push.

## 5 bis. Ce que les mesures ont changé (5 octobre, après la page)

- **La forme compacte des tableaux est passée devant** (`1c232f318`) : un `FLOAT[768]` pesait
  9,2 Ko au journal, il en pèse 3,1. Le seuil se fixera sur cette forme.
- **Le mécanisme est codé** : `Transaction::fallBackToForcedCheckpoint`, `LocalWAL::discard`,
  le compteur lisible par `CALL current_setting('copy_journal_fallbacks') RETURN *` (il compte
  les replis, pas les validations). Les témoins du §5 sont `CopyJournalThresholdTest` et
  `CopyJournalThresholdCrossedTest` ; le banc ajoute la mort pendant le point de reprise d'une
  transaction repliée, à ses sept points.
- **Le défaut du seuil n'est pas fixé.** Quatre passes des embarquements sous `COPY` journalisé :
  491 Mo de journal pour 62 Mo de texte en mode fichiers, sans gain de temps ; 1 396 Mo et
  +30 à 40 s en mode blobs, où les blobs du plein texte sont écrits deux fois. Leur guetteur
  mesure le fichier entre deux points de reprise, pas une transaction : la sonde
  `RAG3DB_PROFILE_JOURNAL` (`c1c2f9dfc`) rendra le poids par transaction et par table, et le
  défaut se fixera dessus, avec une marge de ×2 sous le tampon de 4 Gio.
- **Le basculement ne se justifie pas par la vitesse** (orchestration) mais par la stèle : plus
  de point de reprise forcé par `COPY`, plus d'attente du départ des autres à la validation,
  plusieurs `COPY` dans une transaction durable. Son exigence de performance : ne ralentir aucun
  mode — fichiers à ±3 %, blobs ramenés par le repli à leur temps d'avant.

## 5 ter. Une réservation par page du journal (10 octobre, seconde session cœur C++)

- **Lu** : le journal local écrit dans un `InMemFileWriter` qui prend ses pages une à une, 4 Kio
  chacune (`in_mem_file_writer.cpp:22`) ; hors `TEMP_PAGE_SIZE` c'est `mallocBuffer`, donc un
  `BufferManager::reserve` par page (`memory_manager.cpp:54-57`, `:70-72`). Le `COPY` de
  2 420 766 relations de `RelCopyBMExceptionRecoverySameConnection` journalise ~181,6 Mo, soit
  ~44 300 réservations.
- **Mesuré** (doublure instrumentée, essais interrompus par un refus) : 20 000 à 45 000
  réservations de 4 Kio par essai, 0,7 à 1,8 ms passées dans `reserve`, soit ~40 ns chacune,
  pour un `COPY` de 0,7 à 1,4 s. Le `malloc` de chaque page n'est pas compté. Le coût en temps
  est négligeable ; la conséquence est ailleurs : toute doublure qui refuse « une réservation
  sur N » voit le nombre de réservations d'un `COPY` journalisé multiplié par ~6 (7 000 hors
  journal, 45 000 avec).
- Le seuil de cette page ne s'applique qu'au tampon configuré : sous le tampon par défaut d'un
  test (0,8 × la mémoire, ~97 Gio ici) il vaut son plafond, 256 Mio, et le repli ne joue pas.

## 6. Ensuite

La liste complète jouée « défaut basculé » (`force_checkpoint_on_copy=false`), puis le
basculement : les deux témoins du banc qui attendent le point de reprise d'un `COPY` sont à
réécrire avec lui. Puis la forme compacte des vecteurs au journal, qui recule d'autant le
seuil utile.
