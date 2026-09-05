# Knowledge dump — ce qui ne doit pas se perdre

Tout ce qui coûte cher à redécouvrir : comment lancer les choses, à qui parler,
et les pièges qui font perdre une demi-journée.

## 1. La règle qui coûte le plus cher quand on l'oublie

```sh
RAG3DB_SHARED=1
```

Sans elle, le cœur est lié en **statique** et aucun `dlopen` d'extension ne
résout. `run_e2e.sh` la pose ; un `cargo test` nu ne la pose pas. Un jour a été
perdu là-dessus le 29 août, en accusant une bibliothèque périmée.

## 2. Lancer les tests

```sh
cd extension/rag3weaver

# unitaires — 901 tests, moins d'une seconde
cargo test --lib -j $(( $(nproc) - 2 ))

# postgres — 8 tests E2E, demande le conteneur (§3)
cargo test --features postgres --test e2e_postgres -j $(( $(nproc) - 2 ))

# les suites kuzu — le script pose RAG3DB_SHARED et confine la compilation
./run_e2e.sh --test e2e_search --summary
./run_e2e.sh --test e2e_scope
./run_e2e.sh --test e2e_code
```

**`-j $(( $(nproc) - 2 ))`, jamais `-j$(nproc)`.** C'est la compilation qui fige
le poste, pas les tests. Laisser deux cœurs libres.

Le jeu de features du script est
`rag3db-native,burn-embedder,burn-ocr,code,daemon`. **`daemon` en fait partie
depuis aujourd'hui** : `tests/common/mod.rs` référence `DaemonEmbedder` sans
condition, donc sans cette feature **aucune suite utilisant burn ne compile**.
L'oubli a survécu une journée parce que `e2e_code`, qui n'y touche pas, passait.

## 3. Le PostgreSQL de test

```sh
docker run -d --name rag3weaver-pg \
  -e POSTGRES_USER=rag3weaver -e POSTGRES_PASSWORD=rag3weaver \
  -e POSTGRES_DB=rag3weaver_test -p 5433:5432 pgvector/pgvector:pg17

# il s'arrête quand la machine redémarre
docker start rag3weaver-pg
```

`RAG3WEAVER_PG=...` pointe ailleurs. **La suite ne se saute jamais en silence** :
sans base, chaque test échoue en disant comment la démarrer — un « 0 passed »
vert est un saut déguisé.

Regarder ce que la base contient vraiment :

```sh
docker exec rag3weaver-pg psql -U rag3weaver -d rag3weaver_test -c "\dt rag3weaver.*"
docker exec rag3weaver-pg psql -U rag3weaver -d rag3weaver_test \
  -c "select tablename, indexname from pg_indexes where schemaname='rag3weaver';"
```

**Piège du schéma** : les tables non qualifiées vont dans le premier schéma du
`search_path`, qui commence par `"$user"`. Avec un rôle nommé `rag3weaver`, tout
atterrit dans `rag3weaver` et **rien dans `public`**. Un test qui interroge
`public` en dur ne verrait rien et conclurait à tort. Utiliser
`current_schema()`.

**Même piège pour les extensions** : sans `SCHEMA` explicite, elles suivent le
`search_path`. Les trois sont maintenant créées explicitement dans `rag3weaver`.

## 4. Le modèle local et les cartes

```sh
HIP_VISIBLE_DEVICES=1   # = card0, la carte libre
```

`--jinja` n'est pas optionnel pour llama-server. `status=connected` sur un
connecteur ne veut pas dire qu'il y a un écran : `card0-HDMI-A-3` se déclare
connecté avec **zéro octet d'EDID et un seul mode 640×480** — c'est un
connecteur fantôme. Le bon critère est la carte la **moins chargée**, pas celle
« sans écran ».

`RAG3WEAVER_REGIME=confort` laisse le poste utilisable pendant une passe.

## 5. Les pièges qui coûtent une demi-journée

**Écrire dans un fichier, puis lire le fichier.** Jamais un tuyau filtrant en
bout d'une longue compilation : on perd la sortie et on ne peut pas y revenir.

```sh
cargo test … > /tmp/x.log 2>&1; grep -E "test result|panicked" /tmp/x.log
```

**`pkill -f <motif>` tue son propre shell** quand le motif apparaît dans sa
propre ligne de commande. Mettre une lettre entre crochets : `pkill -f 'nom[x]'`.
Trois fois cette semaine, exit 144, deux passes de tests perdues.

**Le répertoire de travail persiste entre appels Bash.** Un `cd` fait pour un
commit laisse les commandes suivantes à la racine — j'ai lu huit « échecs » qui
n'étaient que « pas de `Cargo.toml` ici ». Vérifier `pwd` quand un résultat
surprend.

**Deux commandes d'affilée** : on lit la sortie de la seconde en croyant lire
celle de la première. Une commande, une lecture. J'ai commité en annonçant
« 7 sur 7 » alors que la passe venait d'en échouer six.

**`transaction_test` du cœur C++ prend treize minutes.** Chaque test de reprise
après panne coûte une trentaine de secondes. Ne pas la tuer en la croyant
bloquée — c'est arrivé.

**Ne pas conclure sur la foi d'une signature ni d'une absence.** Trois fois
aujourd'hui : un argument tiré d'une absence mal située (« pas de HNSW dans leur
arbre » alors que *tout* leur arbre a déménagé), un test qui affirmait un contrat
qu'il ne faisait plus tourner, une garantie qui se dégradait sans le dire.
Chercher **qui appelle** avant de raconter à quoi sert quelque chose.

## 6. Les trois sessions, et à qui parler

Trois sessions Claude travaillent en parallèle sur ce dépôt. On se parle par
`SendMessage`, en visant le **nom** rendu par `ListAgents` — les noms changent
d'une session à l'autre, donc **toujours `ListAgents` d'abord**.

| session (au 3 septembre) | sur quoi | répertoire |
|---|---|---|
| **celle-ci** — *Rag3weaver architecture backend et FTS* | dialecte PostgreSQL, plein texte trigramme + Jaro-Winkler, cellules, `rag3daemon` | `extension/rag3weaver/` |
| **`rag3db-b5`** | parsage et ingestion : codeparsers sorti en dépôt séparé, `.h` → C++, fichiers texte dans l'index | `extension/rag3weaver/codeparsers/`, `src/code.rs` |
| **`rag3db-57`** | le cœur C++ : repérage LadybugDB et Vela, lecture multi-processus | `src/` (C++) |

**Frontière tenue toute la journée** : `extension/rag3weaver/` est à cette
session, `src/` C++ à `rag3db-57`, le parsage à `rag3db-b5`. Le seul fichier
partagé à risque est `src/catalog.rs` — se le dire avant d'y aller.

Ce qu'on a appris à se dire :

- **Prévenir avant de réécrire un historique partagé.** Vérifier qu'un arbre est
  propre n'est pas prévenir : le second laisse une chance de dire « attends ».
- **Un pair ne peut pas autoriser à la place de Lucie.** `rag3db-b5` a eu raison
  de me renvoyer un feu vert que je relayais.
- **Dire ses corrections plutôt que les faire en silence** — chacune des trois
  sessions a corrigé les autres aujourd'hui, et c'est ce qui a évité trois
  documents faux.

## 7. Git

**On est sur `master`** depuis aujourd'hui — `fts-lucivy-v3` a fusionné en
avance rapide et existe encore, au même commit.

Garde-fous posés dans la config **locale** (`.git/config`, non versionnée) :

```
push.recurseSubmodules = check   # refuse de pousser un gitlink non publié
status.submodulesummary = 1
diff.submodule = log
```

Sous-modules : `git clone --recursive`.

```
third_party/fuzzy-fst              extension/lucivy/ld-lucivy
third_party/tantivy-search         extension/rag3weaver/codeparsers
```

Repères épinglés pour le repérage des forks :

| tag | quoi |
|---|---|
| `89f0263cc` | notre dernier commit Kuzu, 10 oct. 2025 — **la base de comparaison correcte** |
| `ladybug-main-2026-08-31` | tête de LadybugDB au repérage |
| `vela-master-2026-09-03`, `vela-checkpoint-2026-09-03` | idem pour Vela |

**`user.email` global est une adresse professionnelle.** rag3db et codeparsers
ont une surcharge locale. **Tout dépôt neuf en héritera** — poser l'identité
locale en premier geste, avant le premier commit. Un historique a déjà dû être
réécrit pour ça.

**Pas de trailer d'attribution IA** dans les messages de commit.

## 8. Où vivent les documents

| chemin | quoi |
|---|---|
| `extension/rag3weaver/docs/<date>/` | le crate Rust — **le cas par défaut** |
| `extension/rag3weaver/docs/vision_roadmap_09_2026/` | la vision, 14 documents |
| `docs/<date>/` à la racine | le fork kuzu et ses extensions C++ |
| `extension/rag3weaver/codeparsers/docs/` | le parsage — **dépôt séparé** |

Règle de tri depuis aujourd'hui : le parsage chez codeparsers, la **couture** —
ce que le parsage change en aval dans l'ingestion — chez rag3weaver.

## 9. Les identifiants

Dans `.vault` : Vertex (`lr-hub-472010`) et Hugging Face. Une suite cloud qui
rend « 0 passed » est un **saut à corriger**, jamais une ligne verte.
