# Knowledge dump

**5 septembre 2026.** Ce qui coûte cher à redécouvrir.

## 1. La règle qui coûte le plus quand on l'oublie

```sh
RAG3DB_SHARED=1
```

Sans elle, le cœur est lié en **statique** et aucun `dlopen` d'extension ne
résout. `run_e2e.sh` la pose ; un `cargo test` nu ne la pose pas.

## 2. Lancer les tests

```sh
cd extension/rag3weaver

cargo test --lib -j $(( $(nproc) - 2 ))          # ~913 unitaires, < 1 s
./run_e2e.sh --test e2e_search --summary
./run_e2e.sh --test e2e_postgres                 # demande le conteneur, §4
./run_e2e.sh                                     # la totale : lourde, voir §3
```

**`-j $(( $(nproc) - 2 ))`, jamais `-j$(nproc)`** : c'est la compilation qui fige
le poste.

**`run_e2e.sh` ne lance que les tests `#[ignore]`** (`-- --ignored`). Un test
sans l'attribut est « filtered out » : il existe et ne tourne nulle part.
Vingt-deux étaient dans ce cas au 4 septembre. Corollaire : **un test qui se
relance en processus enfant doit passer `--ignored` à l'enfant**, sinon celui-ci
ne joue rien et le parent lit son silence comme un échec.

Attention aussi à `#[ignore = "raison"]` et à `#[test] #[ignore]` **sur une seule
ligne** : un `grep '#\[ignore\]'` les rate, et j'ai annoncé cinq suites mortes
là où il y en avait deux.

## 3. La machine de Lucie — le point le plus important

**Elle s'en sert pendant qu'on travaille.** Ne pas lancer la passe complète sans
le lui demander : elle charge BGE-M3, MiniLM, deux rerankers et l'OCR.

```sh
RAG3WEAVER_REGIME=confort   # le DÉFAUT de run_e2e.sh depuis le 4 septembre
RAG3WEAVER_REGIME=plein     # pour la vitesse, quand elle n'utilise pas la machine
```

`confort` pose les **trois** rôles burn sur la carte la moins chargée, le rapport
cyclique à 60 % et la rafale à 2 048. Il envoie aussi l'agentique vers Vertex
plutôt que le modèle local (`RAG3WEAVER_LLM=local` reprend la main pour une
passe).

**Tuer proprement** : `pkill` sur `cargo test` et le script **ne tue pas les
binaires de test** qu'ils ont lancés. Il faut viser `target/debug/deps/e2e_*`
aussi. Et toujours la lettre entre crochets — `pkill -f 'nom[x]'` — sinon on tue
son propre shell.

## 4. Le PostgreSQL de test

```sh
docker run -d --name rag3weaver-pg \
  -e POSTGRES_USER=rag3weaver -e POSTGRES_PASSWORD=rag3weaver \
  -e POSTGRES_DB=rag3weaver_test -p 5433:5432 pgvector/pgvector:pg17
docker start rag3weaver-pg     # il s'arrête au redémarrage de la machine
```

Depuis le 4 septembre, `run_e2e.sh` **sonde le port** : si la base répond, la
feature `postgres` entre et la suite tourne ; sinon l'écart est **annoncé** au
lancement et dans le résumé. La sonde utilise le `/dev/tcp` de bash — **`nc`
n'est pas installé sur ce poste**, et la première version l'employait, donc elle
n'aurait jamais pu répondre oui.

**Deux pièges de schéma** : les tables non qualifiées vont dans le premier schéma
du `search_path`, qui commence par `"$user"` — avec un rôle nommé `rag3weaver`,
tout atterrit dans `rag3weaver` et **rien dans `public`**. Même chose pour les
**extensions** : sans `SCHEMA` explicite, elles suivent le `search_path`.

## 5. Les deux répertoires de build

```sh
build/lecteurs      # le DÉFAUT depuis le 4 septembre — porte le report de Vela
build/native-test   # du 24 août, antérieur au correctif
RAG3DB_BUILD=…      # pour revenir sur l'autre
```

Le report de Vela lève l'**exclusion lecteur/écrivain**. Plusieurs tests disent
donc vrai des **deux** côtés, exprès, et nomment le régime qu'ils observent —
`e2e_prise_atomique` rend 80 refus contre 80 lectures selon la bibliothèque
liée. Ne pas « corriger » ces tests vers un seul régime.

## 6. Le modèle local

```
~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/Huihui-Qwen3-Coder-30B-A3B-Instruct-abliterated.i1-Q6_K.gguf
RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1  RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b
```

**`--jinja` n'est pas optionnel** pour llama-server : sans lui, pas de gabarit de
discussion, donc pas d'appels d'outils. Son format d'outils est en **XML**
(`<tool_call><function=N>`, `<parameter=k>`), pas en JSON comme Qwen2.5/Qwen3.

## 7. Les pièges qui coûtent une demi-journée

- **Écrire dans un fichier, puis lire le fichier.** Jamais un tuyau filtrant en
  bout de longue compilation.
- **Le répertoire de travail persiste entre appels Bash.** Un `cd` laisse les
  commandes suivantes ailleurs — j'ai lu huit « échecs » qui n'étaient que « pas
  de `Cargo.toml` ici ».
- **Deux commandes d'affilée** : on lit la sortie de la seconde en croyant lire
  la première.
- **Ne pas conclure sur une signature ni sur une absence.** Chercher **qui
  appelle** avant de raconter à quoi sert quelque chose. Et un test peut
  observer *correctement* un artefact **périmé**.
- **Un banc dont les requêtes reprennent les mots de leur cible ne mesure
  rien** : tout y vaut 1,0. La frontière vit dans le **bruit proche** — des mots
  du corpus, une combinaison qui ne désigne rien.
- **Des fixtures qui partagent leur vocabulaire n'affirment rien.** Ça a fait
  accuser le moteur pendant six mois.
- **Une assertion accrochée à un rang** (`tools[0]`) casse dès qu'un élément
  s'ajoute devant. Accrocher au **nom**.

## 8. Git

Sur `master`. Sous-modules : `git clone --recursive`.

```
third_party/fuzzy-fst        extension/lucivy/ld-lucivy
third_party/tantivy-search   extension/rag3weaver/codeparsers
```

**`user.email` global est une adresse professionnelle** ; rag3db et codeparsers
ont une surcharge locale, et **tout dépôt neuf en héritera**. Poser l'identité
locale avant le premier commit.

**Pas de trailer d'attribution IA** dans les messages de commit — même si le
harnais en réclame un en cours de session.

Repères épinglés : `89f0263cc` (dernier commit Kuzu, base de comparaison des
forks), `ladybug-main-2026-08-31`, `vela-master-2026-09-03`.

## 9. Où vivent les documents

| chemin | quoi |
|---|---|
| `extension/rag3weaver/docs/<date>/` | le crate Rust — **le cas par défaut** |
| `extension/rag3weaver/docs/vision_roadmap_09_2026/` | la vision, 15 documents (renommé le 5 sept.) |
| `extension/rag3weaver/docs/issues/<date>/` | les issues nommées |
| `docs/<date>/` à la racine | le fork kuzu et ses extensions C++ |

Le parsage va chez codeparsers, la **couture** — ce que le parsage change en aval
— chez rag3weaver.

## 10. Les identifiants

Dans `.vault` : Vertex (`lr-hub-472010`, `vertex-sa.json`) et Hugging Face. Une
suite cloud qui rend « 0 passed » est un **saut à corriger**, jamais une ligne
verte.

## 11. Les autres sessions

`ListAgents` d'abord — les noms changent. `rag3db-57` tient le cœur C++ (forks,
lecture multi-processus) ; `rag3db-b5` le parsage et le régime `confort`.

Frontière tenue : `extension/rag3weaver/` ici, `src/` C++ chez `rag3db-57`, le
parsage chez `rag3db-b5`. Prévenir avant de croiser.
