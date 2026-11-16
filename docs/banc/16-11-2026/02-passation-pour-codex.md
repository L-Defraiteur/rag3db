# Passation du banc pour Codex (16 novembre 2026)

Cette page s'adresse à une session Codex neuve, qui n'a que les docs et les tickets. Elle dit où en
est le banc, ce qu'il y a à faire dans l'ordre, ce qui attend Lucie, et les règles à appliquer.
Chaque entrée de la partie 2 renvoie à un ticket, où se trouve le détail.

Le banc est la session qui **mesure** : parités, juges et bancs de débit. Il ne corrige pas le
produit. Il prouve ce que les autres sessions changent, et il ouvre un ticket quand il trouve un
défaut.

## 1. État

**Sur master** (07fc981588 au moment d'écrire) :
- `docs/banc/16-11-2026/01-la-parite-de-la-migration.md` : la parité de la bascule. Une copie migrée
  A, l'ancienne base B et un index neuf B′ sont jugés contre les bonnes réponses, puis A contre B′
  par signal, et l'histoire au plein texte est prouvée par l'élagage (§ 10). **C'est la page à lire
  en premier.**
- `docs/banc/14-11-2026/02-le-juge-de-parite-de-search-code.md` : le juge, ses régimes et ses
  leçons. La première : « une parité prouve d'abord que les deux côtés ont bâti et joué ».
- `docs/banc/15-11-2026/02-jusqu-ou-l-archiviste-voit.md` : la mesure longue de l'archiviste,
  arrêtée au § 9.
- `docs/banc/15-11-2026/01-l-archiviste-contre-la-compaction.md`,
  `docs/banc/14-11-2026/01-laya-pour-l-archiviste.md` (Laya n'est pas branchable) et
  `docs/banc/14-11-2026/03-l-equipier-sous-gemini.md`.
- Le dernier knowledge dump du banc, `docs/banc/11-11-2026/02-knowledge-dump.md`, est ancien. Les
  pages ci-dessus le complètent.

**En branche** : `banc-parite-search-code` (tête 3d422fad11), le juge de parité. Elle n'est pas sur
master ; voir l'entrée 2.

**Rien ne tourne** du banc à luciepc à 9 h 30. Aucune place de `poste` n'est tenue par le banc.

**Ce qu'il faut savoir pour ne rien casser** :
- **Les copies de la parité** sont à luciepc, sous `~/.cache/rag3weaver-build/banc-parite/` :
  - `migration-lot-A-20261116` (A, la copie migrée) ;
  - `migration-ref-B-20261116` (B, l'instantané réparé) ;
  - `migration-bprime-20261116` (B′, l'index neuf) ;
  - `preuve-master-{neuf,lot,elague}-20261116`.

  Elles servent encore : **ne pas les effacer**. Toute sonde travaille sur une copie reflink de
  celles-ci (`cp -a --reflink=always`). On n'ouvre **jamais** une base de service ni l'original
  d'une autre session.
- **Le juge relit par master**, avec `RAG3WEAVER_PARITE_NO_MIGRATION=1` (une base se lit dans sa
  forme) et `RAG3WEAVER_PARITE_REQUIRE_SIGNALS=vector,bm25`. Un relevé qui ne voit pas un signal
  exigé refuse, au lieu de rendre un faux vert lexical.
- **Une parité de vecteurs se juge par tolérance**, pas au bit près : cosinus au-dessus de 0,99999,
  écart absolu sous 1e-4. Un même texte embarqué dans des lots différents rend d'autres bits
  (§ 4 de la page de parité).
- **Une source de banc a ses sous-modules** : `git submodule update --init`, puis aucune ligne « - »
  dans `git submodule status`. Le premier B′ en manquait.
- **Le seuil de 6 s** (`search.index_wait_s`, 14a78135f1) : une recherche attend l'index en
  synchrone si le reste estimé est court, sinon elle répond « index pas prêt ». Le banc
  d'étalonnage (entrée 6) doit donner à l'hôte le débit par service qui convertit ce seuil en lots.
- **luciepc** : `ssh -T -o BatchMode=yes lucied@luciepc "bash -c '…'"`. Le shell distant est fish,
  d'où le `bash -c`. Une passe longue se lance par `setsid nohup … & disown`, journal dans un
  fichier, puis on relit ce journal dans la minute. Après un `rsync` ou un `scp` d'un script,
  `chmod +x`.
- **Les services d'embarquement de luciepc** : 7878 (granite-278m, dim 768), 7879 (bge-m3) et 7880
  (granite-107m), sur card2. On y est client seulement : jamais un service ajouté sur card2, jamais
  card0, jamais une relance de service (c'est à Lucie).

## 2. Le travail pour Codex, dans l'ordre

**La liste est vivante** (c7, 16 novembre, 11 h) : Codex ne travaille que si Lucie ouvre une session avec lui. D'ici là, les entrées sont au banc, dans l'ordre 6, 1, 2, 3, 4, 7, 8. Chacune est annotée « en cours par banc, branche, tête » quand le banc la prend.

Chaque entrée a son ticket sous `docs/banc/tickets/` ; le ticket a le détail. Les tailles sont en
heures de travail.

### Entrée 1. Juger F300 et N rejouée de la mesure longue

**Commencé par banc, arrêté le 16 novembre à 11 h** :
- **la cause des 45 décisions vides de F300 est trouvée** : du tour 199 au tour 244, chaque Decide rend
  `HTTP 401 ACCESS_TOKEN_EXPIRED`. Le jeton Vertex d'un `models.llm` est pris une fois et dure une heure ;
- ticket `docs/banc/tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md`, pour ED
  (confirmé par c7) ;
- **restant** : juger F300 sur ses 199 premiers tours, en le disant, et rejuger F 25 %, F 25 % variante et S 50
  contre la N rejouée. La N rejouée a ses 244 décisions ; vérifier qu'aucun de ses tours ne porte un 401 sous
  `reponse.errors`, avant de la prendre pour référence ;
- **à corriger aussi** : le script de la mesure longue (`x_archive_long.py`) marque `"ok": true` un tour dont
  `reponse.errors` est plein. Il doit arrêter la passe.

- **Ticket** : `docs/banc/tickets/2026-11-16-juger-f300-et-n-rejouee-de-la-mesure-longue-de-l-archiviste.md`
- **À faire** :
  - expliquer les 45 décisions vides de F300 ;
  - rejuger F 25 %, F 25 % variante, S 50 et F300 contre la N rejouée (`ref-N-0653-20261116-0305`) ;
  - écrire le § 10 de `docs/banc/15-11-2026/02-jusqu-ou-l-archiviste-voit.md`.
- **Ce qui ne se touche pas** : les passes elles-mêmes, qu'on ne rejoue pas, et le code de
  l'archiviste.
- **Preuve** : 244 décisions par juge ; le N contre N rend P1 = 1.
- **Taille** : 3 à 4 h, surtout des appels Vertex en série.
- **Bloque** : rien.

### Entrée 2. Ranger le juge de parité sur master

- **Ticket** : `docs/banc/tickets/2026-11-16-ranger-le-juge-de-parite-de-search-code-sur-master.md`
- **À faire** : une branche `cx-banc-juge-de-parite` depuis master, qui reprend quatre éléments de
  `banc-parite-search-code` : la sonde, le comparateur, le jeu et l'entrée `[[test]]`. Elle les fait
  compiler sur master, avec un témoin du comparateur.
- **Ce qui ne se touche pas** : `src/catalog.rs`, `src/code.rs` et les deux autres sondes de cette
  branche ; les noms existants, qui restent en français.
- **Preuve** :
  - le témoin du comparateur, vu rouge d'abord ;
  - `cargo test --test sonde_parite_search_code --features rag3db-native,code --no-run` sous
    `poste lourd` ;
  - un relevé réel de B′ rejoué.
- **Taille** : 3 à 4 h.
- **Bloque** : le « à toi » de l'orchestration pour le push.

### Entrée 3. La tolérance de score dans le comparateur

- **Ticket** : `docs/banc/tickets/2026-11-16-la-tolerance-de-score-dans-le-comparateur-de-parite.md`
- **À faire** :
  - `--score-tolerance` dans `parite_search_code.py` ;
  - un script neuf `scripts/vector_parity.py`, repris de `~/.cache/rag3weaver-build/vec-cmp.py`
    (luciepc), avec son témoin ;
  - le donner à embarquement pour sa preuve de parité des services.
- **Preuve** : témoins rouges d'abord. La paire `preuve-bm25-master/neuf.json` contre `elague.json`
  doit être VERT à 1e-5 et ROUGE sans option.
- **Taille** : 2 à 3 h.
- **Bloque** : l'entrée 2.

### Entrée 4. La pile de résolution de codeparsers sous le manifeste de Lucie

- **Ticket** : `docs/banc/tickets/2026-11-16-mesurer-la-pile-de-resolution-de-codeparsers-sous-le-manifeste-de-lucie.md`
- **À faire** : deux index neufs de 284cc00c0b, avant (f10336d3f2) et après (tête de master), avec
  `workspace.index_exclude` de Lucie. Puis les comptes, et la liste des Scope sans `DEFINED_IN`.
- **Ce qui ne se touche pas** : le manifeste de Lucie lui-même (on le copie) et les bases de
  service.
- **Preuve** : 0 Scope pour `app.js` dans chaque base, avant tout compte. Les chiffres vont à
  codeparsers, et dans une page du banc.
- **Taille** : 3 h, dont 2 d'index.
- **Bloque** : rien.

### Entrée 5. Rejouer la parité de la migration au pointeur lucivy 4.3.9

**Fait le 16 novembre (banc), ticket fermé** : voir la page de parité, § 11 et § 12. Il reste à rejouer la parité
sur la copie finale de la bascule, quand c7 la refera : même méthode, contre un index neuf du même état de l'arbre.

- **Ticket** : `docs/tickets/closed/2026-11-16-rejouer-la-parite-de-la-migration-au-pointeur-lucivy-4-3-9.md`
- **À faire** : quand `lucivy-core` est en 4.3.9 dans `extension/rag3weaver/Cargo.lock` de master :
  - le bm25 seul A contre B′ (`b-prime-signals.sh`) ;
  - tous les signaux A contre B (`parity-migration.sh`) ;
  - le § 11 de la page de parité.
- **Preuve** : le bm25 seul passe à 30/30 au même classement, sinon c'est un défaut à nommer cas
  par cas. C'est la dernière preuve que la bascule attend.
- **Taille** : 1 h.
- **Bloque** : le pointeur lucivy 4.3.9 (session lucivy). À regarder chaque matin :
  `git show origin/master:extension/rag3weaver/Cargo.lock | grep -A1 'name = "lucivy-core"'`.

### Entrée 6. Le banc d'étalonnage du débit d'embarquement

**En cours par banc, arrêté le 16 novembre à 11 h (mot de Lucie)** :
- **Fait** :
  - la page de conception, `docs/banc/16-11-2026/03-le-banc-d-etalonnage.md`, avec les réponses de c7 et
    d'embarquement (ad29a938c3). Accord pour les pas 1 à 4 ; le pas 5 attend la lecture de la clé
    `embedding_rate:{modèle}:{adresse}@{poste}` par embarquement ;
  - **le pas 1, le jeu figé** : `~/.cache/rag3db-banc-notes/etalonnage/set.json` à luciepc, sha256
    `458f83f0e8877cd3c585eb6b3a54e0ef4b42c0e15cd4e1a72439854326acaa3d`, produit par
    `~/.cache/rag3weaver-build/etalonnage-jeu.sh`. Il contient :
    - 4 096 textes réels de `ScopeContent_Chunk`, par ordre de `_text_hash`, tirés d'une copie de B′ : médiane
      382 caractères, p90 1 751, max 25 927 ;
    - 1 024 morceaux pleins, de 1 400 à 1 500 caractères ;
    - **le nombre médian de morceaux par fichier : 6** (p90 32, sur 6 288 fichiers), pour la colonne « fichiers
      moyens du dépôt en 6 s ».

    Le décodage des textes est vérifié : 4 092 longueurs sur 4 096 égalent `size(_text)`, et les 4 écarts sont
    de quelques caractères non ASCII. `_text_hash` n'est pas le sha256 du seul texte.
- **Restant** : les pas 2 à 5 du § 8 de la page. Aucun code n'est écrit, aucune branche n'existe.
- **Pour le pas 2, ce que le code de master offre** :
  - un faux service à débit connu se monte avec
    `rag3weaver::daemon::embeddings::EmbedDaemon::new(Arc::new(CallbackEmbedder::new(dim, f)))`. La fonction `f`
    dort en proportion des caractères, et le démon se sert par `.servir_lie(TcpListener::bind("127.0.0.1:0"))`
    dans un fil ;
  - le client est celui de l'hôte : `DaemonEmbedder::joindre(adresse)` (feature `daemon`), dont `embed` envoie le
    lot tel quel ;
  - les lots « comme l'hôte » se forment par `crate::embedder::stable_batches(lens, lot_budget(conseil, défaut))`,
    où `conseil` vaut `Embedder::budget_conseille()` ;
  - l'identité se lit par `identite()` (`modele`, `dim`, `lot_conseille`, `max_chars_per_text`).

  Le test proposé est `tests/bench_embedding_rate.rs`, en anglais. Il porte un témoin non ignoré (le faux service
  retrouvé à 5 % près, rouge d'abord, la mesure posée à vide) et une mesure ignorée, réglée par
  `RAG3WEAVER_BENCH_SET`, `RAG3WEAVER_BENCH_SERVICES`, `RAG3WEAVER_BENCH_HOST` et `RAG3WEAVER_BENCH_OUT`.
  La mesure se fait sous `poste mesure`, annoncé à c7.

- **Ticket** : `docs/banc/tickets/2026-11-16-le-banc-d-etalonnage-du-debit-d-embarquement.md`
- **À faire** : **d'abord la page de conception**, soumise à c7 et à embarquement (le jeu de lots de
  1 500 caractères, la mesure par service et par poste, la forme de l'écriture). Puis la mesure,
  après leur accord.
- **Ce qui ne se touche pas** : le catalogue (`Rate`, `note_measured_embedding_rate`) avant
  l'accord d'embarquement ; les services eux-mêmes.
- **Preuve** : `/sante` de chaque service relu avant tout chiffre, et le débit du banc comparé à
  celui des lignes `[montre]` d'un index réel.
- **Taille** : la page 2 h, la mesure 4 h.
- **Bloque** : l'accord sur la page.

### Entrée 7. La mémoire d'un index avec ses vecteurs

- **Ticket** : `docs/banc/tickets/2026-10-19-la-memoire-d-un-index-avec-ses-vecteurs-a-mesurer.md`
  (le protocole y est écrit).
- **À faire** : les passes A (sans `malloc_trim`) et B (avec), chacune seule, sur une base neuve,
  avec les services d'embarquement. Puis la mémoire du processus de l'hôte relevée avant, pendant et
  après l'index.
- **Preuve** :
  - celle du ticket fermé qu'il prolonge ;
  - le témoin rouge, la passe A, montrant la mémoire gardée ;
  - la passe B qui la rend.
- **Taille** : 4 h.
- **Bloque** : une fenêtre où luciepc est peu chargé. Annoncer la mesure par `poste mesure`, qui a
  la priorité, et prévenir l'orchestration.

### Entrée 8. Un témoin de temps du banc qui rougit sous charge

- **Ticket** : `docs/banc/tickets/2026-11-08-un-temoin-de-temps-du-banc-rougit-sous-charge.md`
- **À faire** : rejouer
  `LockBench.AutoCheckpointIsNotPostponedForeverUnderWritersWithoutAGap` seul, à charge basse,
  cinq fois. Puis lui donner une borne qui ne dépend pas du temps du poste : une horloge injectée,
  ou une borne mesurée au début du cas. Enfin l'ôter de `probabilistic.txt`.
- **Ce qui ne se touche pas** : la borne elle-même, qu'on ne relâche pas sans preuve.
- **Preuve** : la nouvelle forme vue rouge sous une charge simulée, verte ensuite. Le banc C++
  entier rejoué une fois.
- **Taille** : 4 à 6 h, du code C++ de test.
- **Bloque** : c'est un test du cœur. Annoncer le lot à l'orchestration avant de commencer.

### Les tickets du banc qui ne sont pas pour Codex

- `2026-11-12-qualifier-gemini-decision-1-apres-son-arrivee-sur-vertex.md` : attend Google Vertex.
- `2026-11-14-un-symbol-que-plus-rien-ne-relie-reste-apres-le-retrait-de-son-fichier.md` : à
  everything declarative.
- `2026-10-12-*` et `2026-10-18-*` (HNSW, fils d'un bâti d'index) : au cœur C++ ou à hotfixs.
- `2026-10-10-*` et `2026-10-11-*` (l'élagage du graphe) : confort, sans porteur.

**État final du banc, le 16 novembre à 11 h** (arrêt au mot de Lucie) :
- rien ne tourne à luciepc ;
- aucune place de `poste` n'est tenue ;
- la branche de code `banc-parite-search-code` est propre et poussée (3d422fad11) ;
- toutes les pages et tous les tickets sont sur master ;
- les entrées 5 (fait), 6 et 1 (en cours) sont annotées ci-dessus ;
- les entrées 2, 3, 4, 7 et 8 ne sont pas commencées.

## 3. Ce qui attend Lucie

Codex n'y touche pas.
- **Le plafond du 7878** (`RAG3WEAVER_EMBED_MAX_CHARS`) : la coupe côté client (828ff28153) rend le
  relèvement inutile. Le relancer reste son choix.
- **Les fichiers texte à lignes énormes** (`demo.test`, `copy_edge_case.test`, `q24.benchmark`…) :
  l'exclusion `minified_line_max_bytes` ne les couvre pas, parce qu'ils sont `texte_brut`. Faut-il
  les indexer ? C'est la « porte du généré », proposée par codeparsers.
- **La variante de l'archiviste** (F, S, à quel chevauchement) : après le § 10 de l'entrée 1.
- **Le ménage de `banc-parite/`** à luciepc : 424 Go apparents, surtout des copies reflink. Rien ne
  s'efface avant la parité de l'entrée 5 et son mot.

## 4. Les règles du périmètre

- **Langue** : tout code neuf est en anglais (identifiants, options, clés JSON, scripts, noms de
  tests et de sorties). La prose, les commits et les tickets sont en français. Les noms existants ne
  se renomment pas.
- **Un worktree et un target à soi** : branches `cx-…`, worktree sous `~/git_workspaces/` ou
  `~/.cache/rag3weaver-build/`, target sous `~/.cache/rag3weaver-build/target-cx-…` (32 Go au plus,
  effacé avec le lot). Jamais l'arbre de Lucie (`~/git_workspaces/rag3db`) : on n'y commite pas, on
  n'y change pas de branche.
- **Passe et push** par `docs/procedures/push-d-un-lot.md`, et jamais sans le « à toi » de
  l'orchestration (c7), même pour un lot court. Les docs seules suivent la même procédure, sous
  `poste push`, avec un commit par chemins (`git commit -- <chemins>`).
- **Rouge d'abord** : un témoin se voit rouge avant le correctif, puis vert. Une suite non jouée est
  rouge ; un test sauté n'est pas vert. Une couleur ne s'annonce pas sans avoir été jouée.
- **Une mesure prouve d'abord l'artefact mesuré** : le binaire, la lib, la base, le manifeste, les
  signaux actifs et le service répondant. Ensuite seulement les chiffres. Et « une parité prouve
  d'abord que les deux côtés ont bâti et joué ».
- **Identité** : `user.email = luciedefraiteur@gmail.com`, vérifiée avant chaque commit. Aucune
  ligne d'attribution à un assistant. Jamais l'adresse pro.
- **Git** : aucun force push, même `--force-with-lease`. Aucune branche distante effacée. Une branche
  rebasée se pousse sous un nom neuf. Chaque commit se pousse dans la minute. « Poussé » se dit
  après trois contrôles : `git status` propre, aucun rebase en cours, FETCH_HEAD égal à HEAD.
- **Lourd** : tout bâti ou toute passe lourde passe par
  `POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd timeout <N> …`, jamais `-j$(nproc)`.
  Laisser deux cœurs libres (`-j 8` à luciepc).
- **Secrets** : `.vault` ne se lit jamais en clair ; les identifiants se passent par leur chemin.
  Vertex en série, avec une attente doublée à partir de 60 s.
- **Processus** : on ne tue que ses propres pids, relus avant le `kill`.
- **Un défaut connu et non corrigé a un ticket** sous `docs/banc/tickets/`, pas une ligne dans un
  message.
