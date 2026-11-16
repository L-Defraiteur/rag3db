# Everything declarative : passation pour Codex (16 novembre 2026)

Pour une session Codex neuve, qui n'a que le dépôt, les docs et les tickets. ED (everything declarative) tient
dans rag3weaver : l'écrivain des générations (la synchronisation du code sous `_couche`, `_depuis`, `_jusqu_a`), la
visibilité des lectures, la recherche lexicale côté rag3weaver, le découpage, les modèles déclarés (`models.*`) et
quelques outils de l'agent. Le fond de la conception est dans les pages du 14 novembre :
`docs/everything-declarative/14-11-2026/03-…` (2a′, les générations), `05-…` (la vue publiée), `06-…` (l'écrivain et le
cache par rang), et le rapport `00-rapport-de-session.md` du même dossier. Le dernier knowledge dump est
`docs/everything-declarative/13-11-2026/03-knowledge-dump.md`.

**État final d'ED (16 novembre 2026, 10 h 20, arrêt demandé par Lucie)** : tout est poussé, plus rien ne tourne. Sur master :
l'écrivain, les garde-fous de la synchronisation, les outils de l'agent, le découpeur, l'ordre stable des ex æquo
(f9ebafc9e). **Une branche prête, pas sur master** : `stable-tie-identity` (entrée 2.A ci-dessous). Un ticket neuf
confirmé par c7 : le jeton Vertex (entrée 2.B). Les entrées 2.1 et suivantes ne sont pas commencées.

## 1. L'état

**Sur master** (16 novembre au matin) :
- **L'écrivain des générations** : une ligne `Scope`/`File` qui change n'est plus réécrite sur place. Elle est fermée
  (`_jusqu_a`), et la ligne neuve naît au rang en attente (`_depuis`). Il recopie les arêtes entrantes, mais jamais
  `CONSUMED_BY`, qui est le miroir d'un `CONSUMES` sortant (témoin 18). Il écrit par paquet (le grain du paquet). « Tout
  visible » se lit sur la base à l'ouverture (témoin 21, commit 209bc299b). Code : `src/code_sync.rs`, les témoins dans
  `src/code.rs`, `mod tests_de_l_ecrivain`.
- **Les garde-fous de la synchronisation sous les générations** : `maxMissingRatio`, la source vide, `files.removed` et
  `scopes.removed`, le plan seul, un fichier écarté à la lecture qui quitte l'index, `files_unchanged`. Suite :
  `tests/e2e_code_sync.rs`.
- **Les outils de l'agent** : `git rev-parse`, `ls-files` et `describe` en lecture seule (`src/commande.rs`) ; `grep_files`
  qui refuse au-delà de 1 000 lignes (`refuse_above`, `src/code_tools.rs`) ; les descriptions de `search_code` et de
  `grep_files` en anglais (gabarits `templates/backends/{code,atelier}/`). `POST /ide/teammate/name` renomme l'agent d'une
  tâche.
- **Le découpeur** borne un morceau en caractères aussi : une ligne plus longue que `max_size` se coupe
  (`src/chunker.rs`, `chunk_lines`).
- **L'ordre stable des ex æquo** de la recherche lexicale (`src/fts_handle.rs`, `search_hits_with_ties` ;
  `src/search.rs`, `search_bm25_chunked`).

**En branche, rien d'ED** : tout est poussé. Les sondes (`sonde-*`, `mesure-2a-prime-3`) sont hors push et ne se fusionnent
pas.

**Pour ne pas casser** :
- **Les suites exigées par chemins** : `tools/poste/required_suites.txt`. Un lot qui touche `src/code.rs`,
  `src/code_sync.rs`, `src/catalog.rs`, `src/catalog/*`, `src/generations.rs`, `src/marque_d_ecrivain.rs` ou
  `tests/e2e_code_sync.rs` joue `e2e_code_sync`. La passe épinglée l'ajoute d'elle-même, et la garde du push refuse une
  passe qui ne l'a pas jouée verte. Le 16 novembre, l'écrivain a cassé cette suite sans qu'aucune passe ne la joue.
- **La lib épinglée** : `rag3db-build-373b5477b…` (le magasin de libs), sous `RAG3WEAVER_MOTEUR_ANCIEN=1`, tant que le C++
  de master est plus neuf que cette lib. La passe le dit en tête de son résumé.
- **La garde des règles de l'analyse** (`extension/rag3weaver/scripts/check_regles_de_l_analyse.py`, jouée par
  check_features) : un lot qui change `src/code.rs` monte `REGLES_DE_L_ANALYSE`, ou dit « sans effet sur l'analyse »
  dans un message de commit, la casse n'y fait rien. Un commit vide suffit s'il n'a changé que des tests.
- **Une lecture brute voit l'histoire** : un `MATCH` écrit à la main sur `Scope` ou `File` lit aussi les lignes fermées.
  Il se borne par `Catalog::condition_visible(alias)`, aux deux bouts pour une arête. Les lecteurs par plan et les
  recherches le font déjà.
- **Le juge de banc** (la parité de `search_code`, `tests/sonde_parite_search_code.rs` sur la branche
  `banc-parite-search-code`) est la référence des classements. Il juge contre un **index neuf** du même arbre, pas contre un
  repassage.

## 2. Le travail pour Codex, dans l'ordre

Chaque entrée renvoie à son ticket, sous `docs/everything-declarative/tickets/` sauf mention. **La commande de la passe**
est la même partout. Elle se joue à luciepc, dans **ton** worktree `~/git_workspaces/rag3db-cx-<nom>`, avec **ton** target :

```
export XDG_RUNTIME_DIR=/run/user/$(id -u)
systemd-run --user --unit=cx-<nom> --collect -E POSTE_MEM_MAX=32G -E RAG3WEAVER_MOTEUR_ANCIEN=1 \
  -E SUITE_DU_TEMOIN=<suite témoin> -E SUITES=<suites, ou -> -E CIBLE=target-cx-<nom> \
  bash -c "$HOME/git_workspaces/rag3db-cx-<nom>/extension/rag3weaver/scripts/poste lourd \
    bash $HOME/.cache/rag3weaver-build/passe_epinglee.sh rag3db-cx-<nom> cx/<journaux> '' \
    > $HOME/.cache/rag3weaver-build/cx/<journaux>.out 2>&1"
```

Le résumé est `~/.cache/rag3weaver-build/cx/<journaux>/resume.log`, et sa dernière ligne dit `VERT` ou `ROUGE …`. Le push
suit `docs/procedures/push-d-un-lot.md` : `poste push <nom> -- bash ~/.cache/rag3weaver-build/push_garde.sh
rag3db-cx-<nom> cx/<journaux>`, dans un `if grep -qx VERT <résumé>`, puis les trois contrôles. Jamais sans le « à toi » de
l'orchestration (c7).

Pour un témoin seul pendant le travail : `cargo test --lib --features
rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm <filtre>`, par
`scripts/cargo_magasin.sh` après `. scripts/lib_du_magasin.sh && exporter_la_lib` (avec `RAG3DB_LIB` posé). Toujours sous
`poste lourd`, avec `CARGO_BUILD_JOBS=8` au plus.

### 2.A En cours : l'ordre des ex æquo d'une base réécrite à une base neuve — prêt, à pousser

**Reprise du 16 novembre, après audit demandé par Lucie** : candidat préparé sur
`cx-ed-stable-tie-identity-r1`, produit `9fcc98ad3f347d821f3f1b99c936a73a9e080d41`,
worktree portable `/home/lucied/git_workspaces/rag3db-cx-ed-stable-tie-identity-r1`.
Les quatre patchs sont repris à l'identique sur master927941a67, sans conflit.
Voir [le candidat et sa recette](02-candidat-stable-tie-identity.md).
La qualification sur lib59ac attend FINI G et l'épingle des passes actualisée ;
aucune passe nouvelle lancée ni ancien GO repris. Cette annotation prime sur
la commande de reprise historique ci-dessous. Entrée2.B Vertex ensuite.

**État : prêt, passe VERTE, pas sur master.** Branche `stable-tie-identity`, tête **1834bf111**, sur master fe5d81f81,
4 commits gmail.
- **Ce que le lot fait** : les ex æquo se départagent par une identité stable, `search::stable_order` : une occurrence
  par son chemin, sa ligne de début et son nom, une version par son uuid de contenu. Avant, l'uuid d'occurrence, qui
  porte `_depuis` sous les générations, changeait d'une base réécrite à une base neuve.
- **Où** : `src/search.rs` (`stable_order`, les tris de `search_bm25_chunked`, de la remontée vers les occurrences et des
  résolutions), `src/dataflow/generic_search_nodes.rs` (le tri de la fusion).
- **Témoin** : `backend::tests::ties_keep_the_same_order_from_a_rewritten_base_to_a_fresh_one`. Rouge sur ea4704bf6
  (deux ordres différents des six ex æquo), vert sur a4d1c59db.
- **Passe VERTE sur 1834bf111** : e2e_code 36, e2e_lectures 27, check_features 8/8, lib 2595/0/72, navigateur 45.
- **Ce qui reste** : attendre le « à toi » de c7. Puis reposer sur la tête de master du moment, sous un nom neuf
  (`git cherry-pick` des 4 commits de `fe5d81f81..1834bf111`, puis la ligne de nouveautés à rajouter si
  `docs/nouveautes.md` entre en conflit). Rejeu court si master a apporté du code, puis le push par la garde.

### 2.B Un modèle Vertex déclaré garde son jeton d'une heure — 4 h, rien ne bloque, gravité haute

Ticket : `docs/banc/tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md`, confirmé pour ED par c7.
- **Le problème** : `connect_llm` (`src/model_source.rs`), en protocole vertex, prend le jeton une fois et le pose en
  `Auth::Bearer` fixe dans `OpenAiLlm::vertex` (`src/openai_llm.rs`). Au bout d'une heure, chaque appel rend 401
  `ACCESS_TOKEN_EXPIRED`, et une passe longue perd ses décisions sans s'arrêter : 45 de perdues dans la passe F300 de banc.
- **À faire** : le client garde la `TokenSource` (`src/gcp_auth.rs`) et appelle `token()` à chaque appel. Sur un 401, il
  fait `invalidate()` puis **une seule** retentative ; un second 401 remonte en erreur nommée. **Ne touche pas** le
  chemin des clés d'API (protocole openai avec clé fixe).
- **Témoin, rouge d'abord** (nom anglais, dans les tests d'`openai_llm.rs`) : un faux serveur (`tests/common/fake_sse`, ou
  un `tiny_http` local) qui accepte le jeton n°1, puis rend 401 jusqu'à ce qu'un jeton n°2 arrive. La `TokenSource`
  factice donne n°1, puis n°2 après `invalidate()`. Le second appel réussit avec n°2 : rouge aujourd'hui, puisque le
  jeton fixe reste n°1.
- **Passe** : `SUITE_DU_TEMOIN=e2e_demon_hote SUITES=-`, avec la feature `openai-llm` (déjà dans la passe).

### 2.1 Un llama-server déclaré est classé cloud — 2 h, rien ne bloque

Ticket : `2026-11-15-un-llama-server-declare-est-classe-cloud-par-budget-de-reprise.md`.
- **Le problème** : `ModelSource::budget_de_reprise` (`src/model_source.rs`) reconnaît un modèle local par `provider`
  (`local`, `service`) ou `protocol: "llama_server"`. Or un llama-server se déclare `provider: "compatible"`,
  `protocol: "openai"`, `address: "http://127.0.0.1:8080/v1"`, et il est alors classé cloud (budget de 100 000 jetons).
- **À faire** : reconnaître aussi le local par l'adresse de boucle (`127.0.0.1`, `localhost`, `::1`, avec ou sans port).
  Retirer `llama_server` des protocoles locaux reconnus, puisque `connect_llm` le refuse pour un modèle de langage.
- **Témoin, rouge d'abord** (nom anglais, dans les tests de `model_source.rs`) : une déclaration `compatible`/`openai` sur
  `http://127.0.0.1:8080/v1`, avec une fenêtre de 32 000, a pour budget la moitié de sa fenêtre et non 100 000.
- **Passe** : `SUITE_DU_TEMOIN=e2e_demon_hote SUITES=-`.

### 2.2 `provider` et `protocol` à l'envers de l'usage — 4 h, rien ne bloque

Ticket : `2026-11-15-provider-et-protocol-les-mots-d-un-modele-sont-a-l-envers-de-l-usage.md`. **Décidé** : la manière
d'accéder devient `access` (`local`, `service`, `compatible`) et le fournisseur `provider` (`openai`, `vertex`, `anthropic`,
`llama_server`). Les anciens mots restent lus par alias **jusqu'au 29 novembre 2026**, puis un refus nommé donne le
nouveau.
- **À faire** : dans `src/model_source.rs` (`ModelSource`, `Provider`), serde lit les deux formes. Un manifeste qui mélange
  les deux est refusé par un message qui cite les deux mots. Les manifestes et docs du dépôt sont réécrits dans la forme
  neuve : `templates/backends/*/backend.json`, `README.md` de rag3weaver, exemples de `docs/`. **Ne touche pas** les
  manifestes de Lucie hors du dépôt : c7 s'en charge.
- **Témoin, rouge d'abord** : l'ancienne forme (`provider: compatible, protocol: openai`) et la neuve (`access: compatible,
  provider: openai`) lisent le même `ModelSource` ; la forme mêlée est refusée en nommant les deux mots.
- **Passe** : `SUITE_DU_TEMOIN=e2e_demon_hote SUITES=e2e_lectures`. Une ligne de nouveautés.

### 2.3 Les erreurs de libération des réclamations et de lecture de dette — 3 h, rien ne bloque

Ticket : `2026-11-11-les-erreurs-de-liberation-des-reclamations-et-de-lecture-de-dette-sont-masquees.md`.
- **Le problème** : `Catalog::rendre_mes_reclamations` ignore une erreur SQL (`if let Ok`) et rend zéro. `run_index`
  (`src/dataflow/index_nodes.rs`) change une erreur de verrou ou de `index_progress` en dette zéro.
- **À faire** : la libération rend un résultat qui distingue le nombre libéré d'une erreur (table et cause). La fin du
  journal d'index garde cette erreur sans effacer la cause d'arrêt d'origine. Une dette illisible se dit inconnue, jamais
  zéro. Pas de reprise silencieuse.
- **Témoins, rouges d'abord** : une connexion qui injecte une erreur sur la requête de libération, puis sur
  `index_progress`. Le journal final garde l'erreur et ne dit pas « dette 0 ». Le chemin nominal reste vert.
- **Passe** : `SUITE_DU_TEMOIN=e2e_lectures SUITES=e2e_code`. `catalog.rs` déclenche `e2e_code_sync` d'office.

### 2.4 Un accord appliqué malgré une erreur de journal — 3 h, rien ne bloque

Ticket : `2026-11-12-un-accord-peut-etre-applique-malgre-une-erreur-de-journal.md`.
- **Le problème** : dans `src/daemon_host/atelier_fichiers.rs`, `decider_accord` publie l'accord en mémoire puis écrit le
  reçu dans `decisions.jsonl`. Si l'écriture échoue, l'appel rend une erreur alors que l'accord est appliqué. Le témoin qui
  le montre existe déjà : `erreur_du_journal_apres_commit_ne_signifie_pas_absence_d_effet` (module
  `publication_accord_tests`).
- **À faire (le choix d'ED pour le contrat de reçu)** : écrire le reçu **avant** de publier l'accord. Si le reçu ne
  s'écrit pas, rien n'est publié et l'erreur le dit. Un reçu écrit dont la publication échoue ensuite se marque annulé par
  une seconde ligne du journal. Ne jamais rejouer un accord au redémarrage.
- **Témoin** : le témoin existant change d'attente (l'accord n'est **pas** appliqué quand le journal échoue). Rouge d'abord
  avec la nouvelle attente.
- **Passe** : `SUITE_DU_TEMOIN=e2e_demon_hote SUITES=-`.

### 2.5 L'estimation d'une réindexation — 5 h, rien ne bloque

Ticket : `2026-11-15-l-estimation-d-une-reindexation-compte-toute-la-source.md`.
- **Le problème** : `estimate_of` (`src/dataflow/index_nodes.rs`, `src/estimate.rs`) estime une réindexation d'une base
  indexée comme un premier index.
- **À faire** : partir de la dette réelle (`dense_missing`) et des seuls fichiers dont l'empreinte (contenu et version de
  l'analyse, colonne `File.analyse`) diffère. Les fichiers inchangés ne coûtent rien. **Ne pas** traiter dans ce lot la
  partie « au rythme observé », qui demande un étalonnage par poste.
- **Témoin, rouge d'abord** : sur une base déjà indexée, sans dette, l'estimation d'une réindexation sans changement
  annonce zéro vecteur et ne demande pas de confirmation (`confirm`).
- **Passe** : `SUITE_DU_TEMOIN=e2e_lectures SUITES=e2e_code`.

### 2.6 L'écrivain décide le genre d'une méthode définie ailleurs — 4 h, rien ne bloque

Ticket : `2026-11-16-l-ecrivain-decide-le-genre-d-une-methode-definie-dans-un-autre-fichier.md`. Tout y est : la voie
choisie (l'écrivain lit les classes visibles du paquet), la limite dite, le témoin
`a_method_defined_in_another_file_than_its_class_is_a_method`. Passe : `SUITE_DU_TEMOIN=e2e_code SUITES=-`
(`e2e_code_sync` d'office). Ferme aussi le ticket de codeparsers en renvoyant ici.

### 2.7 Recommencer une écriture refusée par un ALTER validé — 6 h, bloqué par une lib

Ticket : `2026-11-15-recommencer-une-ecriture-refusee-par-un-alter-valide.md`.
- **Le refus** : il se reconnaît par `contains("Schema changed, retry:")`, la constante C++ étant
  `Transaction::SCHEMA_CHANGED_RETRY`.
- **À faire, les deux grains** : la transaction par paquet du chemin d'instantané se recommence entière. L'instruction seule
  de l'écrivain des générations se recommence seule. Les reprises sont bornées (par exemple 3) et comptées au rapport
  (`SourceSyncReport`).
- **Bloqué** : le témoin ne peut rougir que sur une lib qui porte le refus, le lot du cœur 101ff50a6. Tant que la lib
  épinglée est 373b5477b, on ne peut ni le voir rouge ni le voir vert. Demande à c7 la lib à épingler avant de commencer.
- **Témoin** : un `ADD COLUMN` validé pendant un paquet. Le paquet va au bout, aucune ligne n'est perdue ni doublée, et le
  rapport dit une reprise.

### 2.8 Les témoins instables — 1 à 3 h chacun, rien ne bloque

Un lot par ticket. La règle est la même pour tous : un témoin attend sa condition **par un repère** (un journal, un appel
en vol, une tâche indexée), jamais par une durée, et dit les secondes quand il abandonne. Rouge d'abord sous charge (la lib
entière, ou l'étape navigateur entière).
- `2026-11-08-le-temoin-de-la-porte-de-l-equipier-lit-le-second-retour-avant-qu-il-arrive.md` (1 h) : attendre
  `vus.len() >= 2` dans `src/daemon_host/teammate_tests.rs`.
- `2026-11-07-le-temoin-du-chat-dans-la-page-rougit-a-la-premiere-execution-apres-un-bati.md` (3 h) : le témoin attend
  l'ouverture du flux et appelle `onopen()` une fois.
- `2026-11-07-des-tests-de-la-lib-rougissent-sous-charge.md` (3 h) : `src/commande.rs` et
  `src/daemon_host/taches/tests.rs`. La borne de perte du journal se mesure sur ce que la commande a écrit après la
  dernière troncature.
- `2026-11-13-le-temoin-de-la-variante-sans-accord-rougit-sous-l-etape-navigateur-entiere.md` (2 h) : rejouer l'étape
  entière en gardant le journal de l'hôte du témoin, puis départager une attente trop courte d'un verdict qui n'arrive
  jamais.
- `2026-10-25-le-temoin-des-vues-de-l-ide-rougit-sous-le-lot.md` (2 h).
- `2026-10-18-trois-temoins-rougissent-sous-charge.md` (3 h) : pour le second, établir d'abord si une lecture peut voir un
  lot à moitié.
- `2026-10-12-une-lecture-refusee-pendant-une-reaction-une-fois-sur-neuf.md` (3 h) : faire dire à `lock_for_read` qui
  tient le verrou au refus, puis corriger le tenant.

Passe de chacun : `SUITE_DU_TEMOIN=<la suite du témoin> SUITES=-`. Pour un témoin navigateur, la passe joue déjà l'étape
navigateur entière.

### 2.9 Des morceaux refusés par le service ne sont jamais repris — 4 h, rien ne bloque

Ticket : `2026-11-08-des-morceaux-refuses-par-le-service-ne-sont-jamais-repris.md`.
- **Le problème** : un morceau que le service d'embarquement a refusé (erreur 500) pendant un index reste compté en dette
  par `count_marqueur_manquant` (`src/catalog.rs`), mais n'est plus jamais réclamé. `Catalog::preparer_l_embarquement`
  l'exclut par `suivi.silent_uuids()`, l'état « silencieux » que `EmbedWatch` (`src/dataflow/record_nodes.rs` :
  `silent`, `errors`, `empty`) a noté et qui survit à la passe.
- **À faire** : un morceau refusé redevient réclamable au prochain index. L'état noté par `EmbedWatch` ne doit exclure que
  pendant la passe qui l'a vu. La dette dite est alors celle que le rattrapage peut solder. Le texte vide (`empty`) reste
  exclu : il n'a rien à embarquer.
- **Témoin, rouge d'abord** : le témoin `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it` existe dans une
  forme intermédiaire, sur la branche `temoin-rattrapage-service` (cb996541d). Repars de lui :
  - un premier index sur une petite source, avec un service déclaré qui refuse chaque texte (un `tiny_http` local qui rend
    500, comme dans ce témoin) ;
  - puis une réouverture avec un service sain et `embeddings.catch_up_on_open` ;
  - « vecteurs 0 sur N » aujourd'hui (rouge), N sur N après le correctif.
- **Passe** : `SUITE_DU_TEMOIN=e2e_lectures SUITES=e2e_code` (`catalog.rs` déclenche `e2e_code_sync`).

### 2.10 Le redécoupage forcé des anciens morceaux géants — 4 h, après la bascule

Ticket : `2026-11-16-redecouper-les-versions-dont-un-morceau-depasse-trois-fois-la-taille.md`. Tout y est. **Après la
bascule** des bases de Lucie : c'est sur elles qu'il sert. Passe : `SUITE_DU_TEMOIN=e2e_code SUITES=e2e_lectures`.

### 2.11 Le dialecte postgres sans la forme à générations — 2 jours, rien ne bloque, gros

Ticket : `2026-11-14-le-dialecte-postgres-n-a-pas-la-forme-a-generations.md`. `PostgresDialect` (`src/dialect.rs`) n'a ni
`condition_visible_sur`, ni `create_generation_table`, ni `latest_generation_rank`, ni `sait_les_generations`. Une base
postgres neuve reste donc à la forme d'avant.
- **À faire** : les implémenter en SQL sur le modèle du dialecte rag3db (`Rag3dbDialect`, même fichier), puis faire passer
  sur postgres les témoins de l'écrivain qui ne lisent pas du Cypher brut.
- **Commence par une page de dessin** sous `docs/everything-declarative/<date>/`, que c7 relit avant le code.
- **Passe** : `SUITE_DU_TEMOIN=e2e_postgres SUITES=e2e_code`.

### 2.12 Le scope de namespace devant la classe — 3 h de mesure, puis une page

Ticket : `2026-11-16-un-scope-de-namespace-passe-devant-une-classe-du-meme-nom.md`. Mesure puis page. **Pas de code avant
le mot de c7.**

### 2.13 Après la bascule des bases de Lucie : vérifier, puis fermer

Ces tickets se ferment sur une mesure, pas sur du code. Chacun demande le hash de la bascule à c7, puis une mesure de banc
sur master :
- `2026-11-15-l-ecrivain-de-2a-prime-2-rougit-a-la-parite-lexicale.md` : la cause (A), la version de l'analyse dans
  l'empreinte, est sur master. Le reste de l'écart est la longueur moyenne du ticket BM25. À fermer en renvoyant au ticket
  BM25, une fois la parité rejouée sur master.
- `2026-11-15-le-repassage-d-une-base-par-l-ecrivain-coute-dix-fois-le-parent.md` : le grain du paquet est sur master
  (b45003f0c). Demande à banc la mesure du repassage sur master. Banc l'avait mesuré à 193 s contre 243 s pour le parent,
  sur d57b28c80. Ferme si elle tient.
- `2026-11-15-une-reindexation-par-le-chemin-d-avant-laisse-des-scopecontent-orphelins.md` : la bascule fait passer les
  bases aux générations, et le chemin d'avant n'écrit plus de versions. Il reste une purge des orphelins existants, sur une
  copie d'abord, avec un témoin. 4 h.
- `2026-11-14-une-recherche-a-versions-garde-20-gio-de-memoire-residente.md` et
  `2026-11-14-une-recherche-sans-filtre-paie-le-masque-des-versions-entier.md` : à mesurer sur l'hôte réel après la
  bascule (RSS et durée de dix `search_code`, filtrés et non filtrés). Ferme ce que la mesure ferme, et change le reste en
  ticket chiffré.
- `2026-11-14-la-visibilite-dans-le-predicat-du-masque-coute-et-mene-a-l-oom.md` : depuis le 16 novembre, `tout_visible`
  est faux sur toute base qui a une histoire. Le prédicat porte donc la visibilité en service. À mesurer de même, et à
  traiter en priorité si la mesure montre la mémoire qui monte.

### Ce qui n'est pas pour Codex

Ces tickets sont chez d'autres, ou attendent une décision. N'y touche pas sans le mot de c7 :
- **attendent une décision** : `2026-11-15-l-uuid-par-le-contenu-sans-annexe.md` (arbre principal et c7) ;
  `2026-11-16-l-histoire-gardee-au-plein-texte-deplace-le-classement-bm25-de-la-vue-courante.md` (le correctif est
  lucivy 4.3.9 et son pointeur) ;
- **chez arbre principal** : `2026-11-13-la-couche-de-travail-lisible-des-autres-sessions-n-existe-pas-encore.md`,
  `2026-11-13-l-etat-des-couches-relance-git-a-chaque-lecture-de-l-etat.md`,
  `2026-11-13-l-identifiant-d-un-depot-depend-de-la-branche-extraite.md` ;
- **chez le cœur** : `2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md`,
  `2026-11-14-un-predicat-sur-les-noeuds-et-l-arete-d-un-saut-fait-balayer-toute-la-table.md`,
  `2026-11-15-mesurer-la-plus-longue-transaction-d-ecriture-sur-code-lourd.md` (une mesure pour le cœur, que Codex peut
  faire si le cœur la lui demande) ;
- **chez archi** : `2026-11-08-l-equipier-sur-gemini-ce-que-les-essais-reels-ont-montre.md`,
  `2026-11-15-le-socle-d-agent-de-code-les-primitives-de-fichier.md` ;
- **avec embarquement** : `2026-11-12-des-manifestes-bge-visent-le-port-granite.md`.

## 3. Ce qui attend Lucie

Codex n'y touche pas.
- **La bascule de ses bases** vers les générations : elle attend le pointeur lucivy 4.3.9 (les statistiques BM25 au rang
  visible) et son mot. Il faudra lui dire deux choses :
  - ses recherches denses rendront d'autres résultats (le contenu borné aux octets, le rendu d'une version) ;
  - ses bases garderont leurs anciens morceaux géants jusqu'à la réécriture de leur contenu.
- **Ses manifestes** (le gabarit code-rag3db, les copies des tâches) : c7 y porte les nouvelles descriptions de
  `search_code` et de `grep_files`, avec une sauvegarde. Après le 29 novembre, ils devront aussi dire `access`/`provider`
  (entrée 2.2).

## 4. Les règles du périmètre

- **L'anglais pour tout code neuf** : identifiants, noms de tests, `/commandes`, options, clés de manifeste et de JSON,
  noms d'outils, de nœuds et de routes. Aussi les descriptions d'outils, les invites, les textes des gabarits produit et les
  erreurs rendues par l'API. **Le français pour la prose** : docs, commits, tickets, commentaires. Pas de renommage de
  l'existant : un alias anglais quand on touche la chose, l'ancien gardé un temps.
- **Un worktree et un target à soi**, à luciepc : `~/git_workspaces/rag3db-cx-<nom>` et
  `~/.cache/rag3weaver-build/target-cx-<nom>`. Branches `cx-…`. Jamais l'arbre de Lucie (`~/git_workspaces/rag3db`).
- **Rouge d'abord** : un témoin se voit échouer à l'exécution, pas seulement à la compilation, avant le correctif. Puis
  vert.
- **Une suite non jouée est rouge** ; un test sauté n'est pas vert.
- **Passe et push** par `docs/procedures/push-d-un-lot.md`, jamais sans le « à toi » de c7. Si la garde dit ARRÊT parce
  que master a apporté du code, repose et rejoue court. Pas de push forcé, pas de suppression de branche distante : une
  branche reposée se pousse sous un nom neuf.
- **Le lourd sous `poste lourd`**, `POSTE_MEM_MAX=32G`, au plus 8 tâches de compilation.
- **Une base de Lucie ne s'ouvre jamais en place** : une copie reflink seulement (`cp -a --reflink=always`).
- **L'identité git est l'adresse gmail de Lucie**, par `-c user.email=…`, jamais une adresse professionnelle. **Aucune
  mention d'assistant** dans les commits.
- **Chaque commit se pousse sur sa branche dans la minute.**
- **Un défaut connu non corrigé** devient un ticket sous `docs/everything-declarative/tickets/`, et `check_tickets.py`
  reste vert (`python3 extension/rag3weaver/scripts/check_tickets.py`).
