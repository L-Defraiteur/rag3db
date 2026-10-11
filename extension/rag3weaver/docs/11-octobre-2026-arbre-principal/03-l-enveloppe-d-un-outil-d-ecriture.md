# L'enveloppe d'un outil d'écriture — atomique, ou dit pourquoi pas

*Arbre principal et mémoire longue, 11 octobre 2026. La page avant le code, demandée
par l'orchestration : la machinerie de la transaction par paquet, portée à un outil de
backend. Chaque section dit qui la tient. **Rien n'est codé.***

**D'où ça vient.** Le nœud de lien dit maintenant la vérité sur `atomic`
(`2d0687a71`). En une instruction, il est atomique par le moteur. En plusieurs
instructions de liens seuls, il est enveloppé de `begin` / `commit` / `rollback`, ce
qui est sûr puisqu'un lien n'a aucun effet hors de la base. Quand sa fermeture emporte
une création, il dit `false` avec sa raison. Ce dernier cas, et plus largement **tout
outil qui écrit des lignes**, est le sujet de cette page. Le contrat visé tient en une
phrase : *un outil qui écrit est atomique, ou il dit qu'il ne l'est pas, et ce qui est
resté.*

## 1. Les outils d'écriture des gabarits livrés, et ce que chacun écrit

*Mémoire longue. Recensé sur master le 11 octobre (gabarits `code`, `memory`,
`notebook`, `validated-result`, `proto/boutique`).*

| outil | nœud | ce qu'il écrit | atomique aujourd'hui | ce qu'il dit sur un échec |
|---|---|---|---|---|
| `put_*`, `save_*`, `submit_result` (tous les gabarits sauf `code`) | `EntityRecordNode` (`put`) | **une** ligne, par `ingest_entities` : la ligne, ses morceaux, son plein texte (fichiers), ses vecteurs si l'entité en déclare | **non** : plusieurs instructions, et des écritures hors de la base | « write/index failure (inspect state before retry) » et les avertissements — **sans dire ce qui est resté**, alors que le drain le sait (`rendu_pret`) |
| `create_ref_type` (`memory`) | `CreateRefTypeNode` | une ligne `RefType`, par `ingest_entities` | non, pour la même raison qu'un `put` | **rien** : il jette le rapport d'ingestion (`ingest_entities(…)?`), donc un échec compté dans `failed` passe pour un succès |
| `add_ref` (`memory`) | `AddRefNode` | une `Ref` (ligne, plein texte), **puis** un lien `CITE` | non : deux écritures séparées | fiche absente ou hors état : « écrite seule », dit. **Lien échoué après la `Ref` : une erreur qui ne dit pas que la `Ref` est restée.** Et comme `create_ref_type`, il **jette le rapport d'ingestion** de la `Ref` : un échec partiel passe pour un succès |
| `anchor`, `supersede_link` (`memory`) | `RelationBatchNode` | des liens | **oui**, ou dit pourquoi pas (`2d0687a71`) | « rolled back, nothing written », ou « writes may be partial » avec sa raison |
| `edit_file` (`code`) | `EditFileNode` | un fichier sur disque, puis sa réindexation | non : le fichier est hors de toute transaction | **honnête déjà** : une réindexation échouée rend `index_pending`, qui dit que le fichier est écrit et que l'index n'a pas suivi. C'est la forme à donner aux autres |
| `index` (`code`) | `IndexNode` | un **travail**, avec ses propres transactions par paquet | hors du sujet : un travail long, pas un appel | son journal |
| `run_command` (`code`) | `RunCommandNode` | rien en base | hors du sujet | son code de sortie |

**Ce que le recensement montre.** Dans les gabarits livrés, l'outil qui écrit le plus
souvent écrit **une** ligne. `EntityBatchNode` et les outils d'instantané n'apparaissent
dans aucun gabarit livré : seule l'expérience MTG les déclare. Le cas courant à rendre
honnête est donc « une ligne avec ses dépendances hors base », plus que « plusieurs
lignes ».

## 2. Le contrat, vu de l'outil et de l'agent

*Mémoire longue.*

1. **Tout outil d'écriture rend `atomic`**, et `atomic_reason` quand c'est `false`,
   comme le nœud de lien depuis `2d0687a71`. Un agent n'a pas à deviner, d'après le nom
   d'un outil, si un échec le laisse dans un état mixte.
2. **Un échec dit ce qui est resté**, pas « inspect state ». Le drain sait déjà ce qui
   est prêt (`FlushResult.rendu_pret` : la donnée, le plein texte, le dense, le creux).
   Un échec qui le rend dit à l'agent si la ligne existe, sans sa recherche. Et un échec
   qui n'a rien laissé le dit aussi, puisque c'est la bonne nouvelle.
3. **`add_ref` dit que sa `Ref` est restée** quand le lien échoue après elle, comme il
   le dit déjà pour une fiche absente. Et **aucun nœud ne jette le rapport d'une
   ingestion** : `AddRefNode` et `CreateRefTypeNode` le font aujourd'hui (ticket
   `2026-10-11-add-ref-et-create-ref-type-jettent-le-rapport-d-ingestion.md`). C'est un
   défaut ordinaire, qui ne demande pas cette page pour se corriger.
4. **La forme existe déjà quelque part** : `edit_file` rend `index_pending` quand la
   réindexation n'a pas suivi le fichier écrit. Ce qui est resté, dit dans la réponse
   même : c'est le modèle.
5. **Vu d'un client MCP**, rien ne change dans le protocole : le refus part en
   `isError` avec le texte du nœud, et le succès porte `atomic` dans son JSON. Ce qui
   change, c'est ce que dit le texte.

Ce contrat ne demande **aucune transaction** : il demande de dire la vérité. C'est la
première marche, et elle tient seule.

## 3. Ce que la transaction par paquet garantit, ce qui vit hors de la base, et le prix de l'empoisonnement

*Arbre principal.*

**Ce que fait le paquet** (`code_sync.rs`, `commencer` et `terminer`) :

- `commencer` ouvre `BEGIN` sur la connexion du catalogue et pose `set_in_transaction(true)`.
  Le catalogue n'émet plus de DDL tant que la transaction est ouverte : le schéma est posé
  avant, par `prepare_schema_for_ingest`. Les points de reprise du dataflow sont coupés
  dans la transaction, puisqu'elle-même est le point de reprise.
- `terminer` fait `COMMIT` si le paquet a réussi. Sur un échec, il fait `ROLLBACK`, **puis
  empoisonne le catalogue** (`Catalog::poison`). Un `COMMIT` refusé vaut le même chemin.
- **La garantie** : un paquet est entier ou absent. Une mort pendant la validation laisse
  le paquet entier ou absent, jamais à moitié (`d74c0877e`, moteur `37608cf4b`), et une
  reprise refait les paquets non validés. Depuis la marche 0 (`75de6618f`), l'arrêt propre
  se fait à cette frontière.

**Pourquoi l'empoisonnement.** Le moteur défait la base, mais pas les caches du catalogue,
qui gardent des écritures défaites : les identifiants de nœuds, les modèles enregistrés, les
index plein texte ouverts et la file des liens. Seule une réouverture les remet d'accord avec
la base. L'empoisonnement refuse donc tout jusqu'à elle, plutôt que de laisser un cache
mentir. Le même état sert au point de reprise échoué (« la base doit être rouverte »), avec
`close_without_checkpoint` : la dernière connexion d'une base au tampon plein la ferme sans
point de reprise, qu'elle ne pourrait pas faire.

**Ce qui vit hors de la base, et comment le paquet le rattrape :**

| quoi | pendant le paquet | si la transaction est défaite ou le processus meurt |
|---|---|---|
| **plein texte en fichiers** (le défaut d'une base neuve) | lucivy écrit ses segments ; avant le `COMMIT`, le dossier est rendu durable, puis sa **génération est marquée en base, dans la même transaction que les lignes** (`flush_blob_store` → `mark_next_generation`, `fts_generation:<clé>`) | la génération marquée reste l'ancienne ; à l'ouverture, le plein texte relit celle que la base connaît. Les segments en trop sont des orphelins, sans réponse fausse |
| **plein texte en blobs** (les bases d'avant la bascule) | les blobs vont **dans** la transaction (`UNWIND … MERGE`, lots de 32 Mo) | défaits avec elle. Un mécanisme de report existe sans être le défaut : `defer_blob_push` jusqu'à la fin de la synchronisation, sous la marque durable `fts_pending` que `rebuild_pending_fts` rattrape. Le défaut casse sous un petit tampon (ticket du COPY de blobs) |
| **vecteurs** | **jamais dans le paquet** : l'indexation exige le plein texte seul, et les vecteurs sont une **dette** payée après, par passes de 512 (`embarquer_le_retard`), dans leurs propres écritures | la dette se compte et se rattrape à la passe suivante ; l'état d'index dit « vecteurs en cours » |
| **liens en file** (mode de masse) | en mémoire, posés à la fin ou quand la file dépasse sa borne ; la marque `relations_pending` est en base | une reprise finit en masse quand la marque est posée, et la file oubliée est refaite depuis les fichiers (marche 0) |

**Réponse à la question de la §4 : oui, le paquet diffère déjà le hors-base au `COMMIT`**, et
c'est la forme à porter. Ce n'est pas la poussée des blobs, qui est un report jusqu'à la fin
de toute la synchronisation, mais le **plein texte en fichiers**. Le hors-base est rendu
durable avant le `COMMIT`, et **sa marque de génération voyage dans la transaction de la
base**. Si la transaction tombe, la marque tombe avec elle, et l'ancien état reste le seul que
la base connaisse. Pour un `put` d'une ligne, c'est exactement le cas : la ligne et ses
morceaux en base, les vecteurs en base (colonnes, et l'index vectoriel du moteur dans la même
transaction), et le plein texte en fichiers avec sa génération marquée dans la transaction.

**Le prix de l'empoisonnement, pour un outil :**

- **Aujourd'hui**, un processus MCP par session : le serveur sort avec le code 75 et le
  client le relance. Le prix est une relance pour **une** session.
- **Sous la forme C** (marche 1, `deb423288`), le backend est lâché seul : ses réactions et
  ses indexations s'arrêtent, l'appel en cours rend une erreur transitoire, et l'appel suivant
  le rouvre. Les autres backends ne voient rien. Mais **toutes les sessions de ce backend**
  perdent leurs appels en vol, et un réacteur s'arrête au milieu d'une réaction. Un outil
  courant qui empoisonne sur un échec banal (une validation de schéma, un conflit de révision)
  coûterait une réouverture à tout le monde.
- **Donc un outil ne doit empoisonner que sur ce que le moteur impose** : un point de reprise
  échoué ou un `COMMIT` refusé. Un échec qu'il sait défaire, un `ROLLBACK` d'une transaction
  dont il connaît les caches touchés, doit **invalider ces caches-là**, et non le catalogue
  entier. C'est le travail que l'empoisonnement évite au paquet, qui touche tous les caches ;
  un outil d'une ligne en touche peu (l'identifiant de sa ligne, et sa génération de plein
  texte).

**La question dure, et la première réponse de l'arbre principal** (marche 1 de C) : sous
l'hôte, un backend dont la base doit être rouverte est lâché seul. Son appel en cours
rend une erreur transitoire nommée, jamais rejouée, et l'appel suivant le rouvre. Les
autres backends ne voient rien, mais **les autres sessions du même backend perdent leurs
appels en vol, et leurs caches avec**. Un outil qui empoisonne son catalogue coûte donc
une réouverture à tous les clients de ce backend.

## 4. Les formes possibles, et leur coût

*À deux, à écrire.* De la moins chère à la plus chère :

- **dire** (§2) : `atomic: false` partout où ce n'est pas sûr, et ce qui est resté après
  un échec ;
- **envelopper ce qui est sûr** : comme pour les liens, une transaction seulement quand
  tout ce que l'outil écrit vit dans la base ;
- **différer ce qui vit hors de la base au `commit`** : le plein texte et les vecteurs
  écrits après la validation de la transaction de la base, rattrapés par la dette si le
  processus meurt entre les deux ;
- **la machinerie complète du paquet**, empoisonnement compris, avec son prix au §3.

## 5. Les témoins

*Mémoire longue pour la part outil ; arbre principal pour la part transaction.*

- **un `put` qui échoue après la ligne et avant ses vecteurs** dit que la ligne existe et
  que sa recherche dense manque (faute injectée, sur le modèle de celle du drain des
  liens : deux variables, jamais l'une sans l'autre) ;
- **un `add_ref` dont le lien échoue** dit que la `Ref` est restée ;
- **chaque outil d'écriture des gabarits livrés rend `atomic`** : un filet sur le modèle de
  `src/gabarits.rs`, qui rougit quand un gabarit ajoute un outil qui ne le dit pas.

## 6. Ce qui reste à Lucie

*À écrire en dernier.*
