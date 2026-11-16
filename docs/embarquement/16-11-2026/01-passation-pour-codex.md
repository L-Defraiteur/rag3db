# Embarquement : passation pour Codex (16 novembre 2026)

Pour une session Codex neuve, qui n'a que les docs et les tickets. Chaque entrée de la partie 2 doit pouvoir se
commencer sans question ; si l'une ne le peut pas, c'est la page qui est en faute.

Le contexte long est ailleurs :
- **la page de règles de l'embarquement**, à jour au 16 novembre, avec le tableau de tous les réglages :
  `docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md` (sections « Les réglages de
  l'embarquement, en un seul endroit », « Le repli vers un service quand la carte est pleine ») ;
- **le dernier knowledge dump** : `docs/embarquement/13-11-2026/02-knowledge-dump.md` (antérieur aux lots du 15 et
  du 16, résumés ci-dessous) ;
- **la procédure de push** : `docs/procedures/push-d-un-lot.md`.

## 1. État

### Sur master (par lot, du plus ancien au plus récent)

| Hash | Ce que le lot a apporté |
|---|---|
| 73aa05511 | Le repli quand la carte est pleine (`fallback: {"service": …}`, `required_mib`, `margin_mib`, `min_throughput`, `window_secs`, `fallback_hold_secs`), la provenance des vecteurs au journal d'index ; le plafond d'une requête embarquée (`models.embed.query_max_chars`, 6 000 par défaut) |
| f85977d09e | Le service d'embarquement : une panique de calcul rend un 503 nommé, puis le service s'arrête avec le code 70 en le disant ; le plafond d'un texte (`RAG3WEAVER_EMBED_MAX_CHARS`, 12 000) |
| 6157600059 | Le repli par activation nommée (`fallback: {"activation": "<nom>"}`, `address`, `fallback_hold_max_factor`), au placement près, sa carte mesurée |
| 828ff28153 | Un texte au-dessus du plafond est **coupé et compté**, jamais refusé avec son lot (client et service) |
| ca3120fb70 | Le journal d'index écrit les textes coupés et les jetons tronqués ; chaque index a son propre journal |

### En branche, à garder sans fusionner

- `repli-cpu-a3-2` (worktree `~/git_workspaces/rag3db-a3` à luciepc) : un essai de repli CPU des tests, **dont le
  vert était dû à la chance** (voir le ticket `2026-11-15-un-repli-cpu-des-tests-sans-vulkan.md`). Ne pas fusionner.
- Le worktree `~/git_workspaces/rag3db-repli-nuit` à luciepc : l'hôte bâti d'avance pour la mesure de la carte
  pleine (partie 2, entrée 11). Il se retire par `garde_worktree.py` après cette mesure.

### Ce qui tourne à luciepc

- **7878** (`rag3weaver-embeddings@granite-278m`) et **7879** (`rag3weaver-embeddings@bge-m3`, avec le relecteur
  bge-reranker-v2-m3 et l'OCR ppocrv6-tiny), sur card2, binaire `~/.local/bin/rag3weaver-embeddings` de sha256
  `a70f1e46…`. Ce binaire déclare `max_chars_per_text: 12000`, mais **pas encore `cut_texts`** : il précède la coupe
  côté service. Face à un client d'avant 828ff28153, il refuse encore en 413 un lot qui contient un texte trop
  long. Les hôtes bâtis sur master coupent avant l'envoi, ce qui suffit. Le binaire neuf `6196fbe7…` (de master)
  attend la pose de la bascule, faite par la session G (optimiseur) avec ses hôtes.
- **Le journal propre de chaque service** : `~/.cache/rag3weaver-unites-embarquement/journal-<modèle>.log` (le
  journal de l'unité systemd est vide, le drop-in `journal.conf` y renvoie la sortie). La dernière ligne d'un arrêt
  dit « ✗ arrêt : <cause> (code N) ».

### Pour ne rien casser

- **card0 est l'écran de Lucie** : jamais un modèle, un essai ni un exemple dessus. **card2** porte les services : un
  modèle de plus n'y part qu'avec 13 Gio libres, ou le 7879 arrêté (par G, annoncé). Un essai GPU va sur l'iGPU
  (`igpu:0`, card1, sans écran).
- **Les unités `rag3weaver-embeddings@`** ne se touchent pas : ni arrêt, ni `.env`, ni pose. C'est la session G,
  à son mot et avec annonce, ou Lucie. **Jamais un essai sur 7878 ou 7879 en place** : un service d'essai se lance à
  part, sur un port libre.
- **Le plafond de 12 000 caractères** : un texte plus long est coupé avant l'envoi, et le journal d'index écrit « N
  texte(s) coupé(s) à 12000 caractères avant l'envoi au service « … » ». Ce n'est pas une erreur : c'est la ligne
  d'un fichier de données géant (ticket du découpeur, chez ED).
- **Le régime GPU des passes** et les variables des services (`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879`
  à luciepc) sont posés par `tools/poste/passe_epinglee.sh` : ne pas les réinventer.

## 2. Travail pour Codex, dans l'ordre

Chaque ticket est sous `docs/embarquement/tickets/`, sauf mention. Ordre : du plus sûr au plus dépendant. Les
commandes de la partie 4 valent pour toutes les entrées.

### 1. La tolérance de parité entre services — `2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`

- **À faire** : un script client `extension/rag3weaver/scripts/embed_parity.py`, qui embarque un jeu fixe
  (`extension/rag3weaver/tests/fixtures/embed_parity/texts.json`, une centaine de textes de 50 à 1 400 caractères)
  seul puis en lots de 8, 32 et 128 de compositions différentes, et rend l'écart absolu maximal, le cosinus minimal
  et la part identique au bit. Puis la mesure sur 7878 et 7879, et la règle de preuve d'une pose écrite dans la page
  de règles (bit près dans un même lot entre deux binaires ; sous la tolérance mesurée, avec une marge × 2, entre
  lots). Le détail est au ticket.
- **Preuve** : `extension/rag3weaver/tests/e2e_embed_parity.rs`, `#[ignore]`, test
  `embed_parity_on_the_fixture_is_bit_exact`, sur la fixture `tests/fixtures/residence/fournisseur.py` (écart 0,0
  attendu). Rouge d'abord : le script absent. Suite : `./run_e2e.sh --test e2e_embed_parity`.
- **Taille** : 3 à 4 h.
- **Bloquant** : rien. La mesure réelle charge 7878 et 7879 quelques secondes : l'annoncer au banc.

### 2. Le bit selon le lot — `2026-11-16-un-meme-texte-ne-rend-pas-les-memes-bits-selon-son-lot.md`

- **À faire** : rien d'autre que le fermer quand l'entrée 1 est sur master, en le disant (observation tenue, pas de
  correctif) : `git mv` vers `docs/tickets/closed/`, l'État réécrit (« fermé … »), dans un commit de docs.
- **Taille** : 15 min. **Bloquant** : l'entrée 1.

### 3. Un réglage déterministe de cubecl — `2026-11-16-un-reglage-deterministe-de-cubecl-a-chercher.md`

- **À faire** : lire burn et cubecl (les versions du `Cargo.lock`, sous `~/.cargo/git/checkouts/`) pour un réglage
  qui fixe l'autotune ou rend les réductions déterministes ; le mesurer avec le script de l'entrée 1 sur un service
  d'essai **sur `igpu:0`**, port libre. Gratuit = écart 0 entre lots de même remplissage **et** débit qui ne baisse
  pas de plus de 2 %. Si gratuit : le poser par défaut dans `extension/rag3weaver/src/burn_device.rs`. Sinon : fermer
  en notant les chiffres.
- **Preuve** : les chiffres dans le ticket ; si un réglage est posé, un test de lib sur `igpu:0` (sans iGPU, le test
  dit qu'il est sauté, et c'est **rouge**).
- **Taille** : 2 à 3 h, plus 2 h si posé. **Bloquant** : l'entrée 1.

### 4. Le refus « est mort avant de répondre » nomme le programme — `2026-11-14-le-refus-est-mort-avant-de-repondre-nomme-le-type-de-service.md`

- **À faire** : dans `extension/rag3weaver/src/serveur.rs`, le refus d'un démarrage mort nomme le **programme**
  lancé (son nom de fichier), à côté du type de service. `DaemonEmbedder::serveur` (`src/daemon/embeddings.rs`)
  passe aujourd'hui `SERVICE` comme nom. Le nom du journal (`<journal>/rag3weaver-embeddings.log`) **ne change
  pas** : `cause_d_arret` (`src/model_source/activation.rs`) le lit.
- **Preuve** : un test de lib dans `serveur.rs` ou `activation.rs`, `a_dead_start_names_the_program`, sur la fixture
  `fournisseur.py` lancée avec une option qui la fait sortir au démarrage (`--arret-nomme <marqueur>`, le marqueur
  posé d'avance) : le refus contient `fournisseur.py`. Rouge d'abord.
- **Taille** : 1 à 2 h. **Bloquant** : rien.

### 5. Le témoin d'un creux d'un autre modèle sans dual — `2026-11-15-aucun-temoin-qu-un-creux-d-un-autre-modele-que-le-dense-n-attache-pas-de-dual.md`

- **À faire** : un témoin seulement (le code est juste). Dans `extension/rag3weaver/src/backend.rs` (tests), un
  backend dense granite + `models.sparse` bge-m3 activé sur la fixture, même dimension : pas de dual attaché
  (le catalogue n'a pas de `dual_embedder`, et le service `has_dual` qu'il enregistre dans `catalog.rs` est faux),
  et la requête dense passe par le dense. Puis un creux du même modèle que le dense : le dual est
  attaché. Le détail est au ticket.
- **Preuve** : le témoin doit rougir si l'on retire la condition `source.model == self.embed.model` dans
  `open_with_reason` : le vérifier une fois, à la main, avant de remettre la condition (c'est son rouge).
- **Taille** : 1 à 2 h. **Bloquant** : rien.

### 6. Le plafond exact en jetons — `2026-11-15-un-plafond-par-texte-en-jetons-exact.md`

- **À faire** : une méthode `count_tokens(&self, text) -> Option<usize>` au trait `Embedder`
  (`src/embedder.rs`), relayée par `Arc`, implémentée par les quatre embarqueurs burn (`burn_bge_m3_embedder.rs`,
  `burn_granite_embedder.rs`, `burn_minilm_embedder.rs`, `burn_multilingual_minilm_embedder.rs`). **Compter avant
  la troncature** : leur tokenizer est réglé avec `TruncationParams`, donc un `encode(…).len()` rend au plus le
  plafond du modèle. Encoder sans troncature (une copie avec `with_truncation(None)`) ou lire
  `Encoding::get_overflowing()`. Puis un `max_tokens_per_text` dit dans l'identité du service, et la même coupe que
  pour les caractères (`cut_to_cap` dans `src/daemon/embeddings.rs`), comptée.
- **Preuve** : un test de lib par embarqueur qui charge un tokenizer (les modèles sont dans le cache de luciepc ;
  sans modèle, rouge nommé) : un texte de 3 000 jetons compte 3 000, pas 512. Rouge d'abord.
- **Taille** : une demi-journée à une journée. **Bloquant** : rien.

### 7. La cause d'arrêt d'un service posé à la main — `2026-11-15-la-cause-d-arret-d-un-service-pose-a-la-main-n-est-pas-dite-par-l-hote.md`

- **À faire** : une conception courte d'abord, soumise à l'orchestration (une page `docs/embarquement/<jour>/`) :
  un champ optionnel `journal` dans une déclaration `models.<capacité>` (le chemin du journal du service posé),
  lu comme `cause_d_arret` le fait pour une activation (`src/model_source/activation.rs`), pour que l'état d'index
  dise « service d'embarquement arrêté : <cause> » au lieu de « pas joint ». Puis le code.
- **Preuve** : un test avec la fixture lancée à part (hors activation, comme `poser_un_service` dans les tests de
  `activation.rs`), arrêtée avec `--arret-nomme`, et la déclaration qui nomme son journal : l'état le dit.
- **Taille** : une demi-journée. **Bloquant** : la conception relue.

### 8. Le débit par service — `2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md`

- **À faire** : la part d'embarquement seulement (la table par provenance et sa lecture). Une page de conception
  d'abord ; le détail, les fichiers et ce qui ne se touche pas sont au ticket.
- **Preuve** : `the_rate_is_noted_per_provenance`, `known_embedding_rate_reads_the_current_provenance` ; suites
  exigées `e2e_code_sync` et `e2e_lectures`.
- **Taille** : une journée. **Bloquant** : la conception relue ; la part du banc et celle de l'arbre avancent à part.

### 9. Les anciens morceaux géants — `2026-11-16-les-anciens-morceaux-geants-sont-a-reecrire.md`

- **À faire** : `Catalog::oversized_chunks(factor)` et `rewrite_oversized_chunks(factor)` sur le modèle de la
  réécriture des morceaux sans texte, appelés par `run_index` avant les vecteurs, dits au journal, suspendus par
  `RAG3WEAVER_REWRITE_OVERSIZED=0`. Le détail est au ticket.
- **Preuve** : `oversized_chunks_are_rewritten_before_the_vectors` ; suites `e2e_code_sync`, `e2e_lectures`.
- **Taille** : une demi-journée. **Bloquant** : le découpeur d'ED sur master d'abord
  (`docs/everything-declarative/tickets/2026-11-16-le-decoupeur-laisse-passer-un-morceau-de-53-000-caracteres.md`).

### 10. Le repli CPU des tests — `2026-11-15-un-repli-cpu-des-tests-sans-vulkan.md`

- **À faire** : un vrai repli CPU des embarqueurs de test par le backend ndarray de burn, pour que
  `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=cpu` charge granite-107m sans GPU. Aujourd'hui `BurnDevice::Cpu`
  (`src/burn_device.rs`) passe par wgpu, et llvmpipe (installé) refuse la réservation de 384 Mo de cubecl.
  Ne pas fusionner la branche `repli-cpu-a3-2`, dont le vert était dû à la chance : **lire la sortie de l'enfant
  relancé**, pas la ligne « ok » du parent.
- **Preuve** : le test `A3` de `tests/e2e_aide_des_tests_repli.rs` (sur la branche), vert sur un poste **sans GPU
  visible** (`WGPU_ADAPTER_NAME` vide), plusieurs fois de suite.
- **Taille** : une journée, peut-être plus (le backend ndarray de burn doit couvrir les opérations des modèles).
- **Bloquant** : rien ; priorité basse.

### 11. La mesure du repli sur une carte réellement pleine — `2026-11-15-mesurer-le-repli-sur-une-carte-reellement-pleine.md`

- **À faire** : la jouer, une fois, quand une passe occupe card2 **en mémoire** (un modèle résident en plus, un gros
  travail GPU). Les vecteurs du 7878 ne la remplissent pas assez : constaté le 15 au soir. Tout est prêt à luciepc
  dans `~/.cache/rag3weaver-build/groupe-repli-jeu/` :
  - `guetter.sh` lit card2 toutes les 15 s, pendant 3 h au plus, et lance la mesure une fois ;
  - `nuit.sh` garde avant tout : card2 sous 13 548 Mio libres, le fichier `va-du-banc` de moins de deux heures, le
    7879 sur un binaire qui dit `max_chars_per_text`. Il bâtit l'hôte sur master, puis lance `mesure.sh` ;
  - le résultat sort en `resultat-<date>.md`, à recopier dans le ticket et dans la page de règles.
  Lancer par `setsid nohup bash ~/.cache/rag3weaver-build/groupe-repli-jeu/guetter.sh > …/guet.out 2>&1 &`.
- **Preuve** : le fragment de résultat (verdict, repli dit, morceaux par le 7879, index loin des 904 s du 14).
- **Taille** : 1 h, plus l'attente de la fenêtre. **Bloquant** : la passe qui charge card2 et le « va » de son
  propriétaire (le banc, l'arbre), donnés par l'orchestration.

### 12. granite-278m arrêté huit fois — `2026-11-08-granite-278m-a-luciepc-s-est-arrete-huit-fois-cause-illisible.md`

- **À faire** : à la prochaine sortie du 7878 ou du 7879, lire `journal-<modèle>.log` et nommer la cause. Le service
  dit désormais sa cause en dernière ligne. Sans nouvelle sortie avant le 8 décembre 2026, fermer le ticket en le
  disant.
- **Taille** : 30 min. **Bloquant** : une sortie, ou la date.

### 13. La résidence des modèles — `2026-11-12-gerer-activation-et-residence-des-modeles.md`

- **Ce qui reste** (page de règles, section « Manquant ») : la politique de préchauffage anticipé, et la
  déclaration des services de card2 par une `Activation`.
- **Bloquant** : **le mot de Lucie**, car les trois unités résidentes sont à elle. Ne rien commencer sans lui.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **La relance du 7878 avec un plafond relevé** (`RAG3WEAVER_EMBED_MAX_CHARS=60000` dans
  `~/.config/rag3weaver/embeddings/granite-278m.env`), proposée le 16 au matin pour l'index B′ du banc, a été
  **refusée par les permissions de la session d'embarquement**. Personne ne l'a faite à sa place. Elle n'est plus
  nécessaire, puisque la coupe côté client (828ff28153) a rendu B′ vert sans elle. Elle reste à la décision de Lucie.
- **La pose du binaire neuf** des services (`6196fbe7…`) se fait avec la bascule, par la session G, au mot de
  l'orchestration.
- **Déclarer les services de card2 par une activation** (entrée 13) : son mot.

## 4. Règles du périmètre, pour Codex

- **Anglais pour tout code neuf** : identifiants, options, clés de manifeste et de JSON, variables d'environnement,
  noms de tests, scripts. Le **français** reste pour la prose : docs, commits, tickets, messages. L'existant ne se
  renomme pas ; un alias en anglais quand on le touche.
- **Un worktree et un target à soi** à luciepc, branche `cx-<sujet>` :
  ```bash
  git -C ~/git_workspaces/rag3db fetch -q origin
  git -C ~/git_workspaces/rag3db worktree add -b cx-<sujet> ~/git_workspaces/cx-<sujet> origin/master
  git -C ~/git_workspaces/cx-<sujet> submodule update --init extension/rag3weaver/codeparsers
  ```
  Jamais rien dans `~/git_workspaces/rag3db` (l'arbre de Lucie). Le shell de luciepc est fish : passer les commandes
  par `ssh luciepc bash -s <<'EOF' … EOF`.
- **Tests de lib, sous le verrou du poste** (jamais `-j$(nproc)`) :
  ```bash
  cd ~/git_workspaces/cx-<sujet>/extension/rag3weaver
  POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd env RAG3WEAVER_TARGET_REPRENDRE=1 \
    CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-cx-<sujet> RAG3DB_LIB=courant CARGO_BUILD_JOBS=8 \
    scripts/cargo_magasin.sh test -j8 --lib --features rag3db-native,code,daemon,openai-llm -- <filtre>
  ```
  Sans `openai-llm`, des témoins de tâche rougissent sans rapport avec le lot : toujours le jeu des passes.
- **Rouge d'abord** : le témoin est vu rouge pour la bonne raison (lire le panic), commité et poussé, puis le
  correctif. **Une suite non jouée, ou un test qui se dit sauté, est rouge.**
- **La passe et le push** suivent `docs/procedures/push-d-un-lot.md` : la passe par `tools/poste/passe_epinglee.sh`
  (copie au cache `~/.cache/rag3weaver-build/passe_epinglee.sh`), avec `CIBLE=target-cx-<sujet>`,
  `RAG3WEAVER_MOTEUR_ANCIEN=1`, et `SUITES` selon les chemins touchés :
  - `catalog*`, `code.rs`, `code_sync.rs`, `generations.rs`, `marque_d_ecrivain.rs` → `e2e_code_sync`, que la garde
    vérifie ;
  - le journal d'index (`index_nodes.rs`, `worktrees.rs`) → `e2e_lectures` ;
  - l'activation et la fixture → `e2e_embarqueur_a_la_demande`.
  Le push se fait par `poste push <session> -- bash ~/.cache/rag3weaver-build/push_garde.sh <worktree> <dossier>`,
  **seulement si** la première ligne de `resume.log` nomme la tête **et** que la dernière est `VERT`. Une passe en
  file garde l'ancien résumé : ne jamais croire un VERT qui ne nomme pas la tête. **Jamais sans le « à toi » de
  l'orchestration.**
- **Après le push** : les trois contrôles (status vide, aucun rebase, FETCH_HEAD = HEAD), puis le retrait du worktree
  par `extension/rag3weaver/scripts/garde_worktree.py`, avec une copie fidèle de `.git/worktrees/<nom>` en archive.
  **Jamais de force push.** Une branche reposée se pousse sous un nom neuf (`-r1`, `-r2`). Une branche distante ne
  s'efface pas.
- **L'identité git** est l'adresse gmail de Lucie, sans aucune mention d'un assistant dans les commits.
- **Un défaut connu non corrigé va dans un ticket**, contrôlé tout de suite par
  `python3 extension/rag3weaver/scripts/check_tickets.py` (la Famille : `moteur` ou `produit` seulement).
- **Jamais un essai sur 7878 ou 7879 en place**, jamais card0, jamais d'arrêt ni de `.env` d'une unité
  `rag3weaver-embeddings@`.
