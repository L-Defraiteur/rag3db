# Le paquet npm de rag3weaver — chantier G

9 octobre 2026, orchestration, à la demande de Lucie. Rien n'est codé. À
prendre par une session dans **son propre worktree**, après que Lucie l'a
placé dans l'ordre des chantiers (`01-plan-de-reprise.md`).

## 1. Ce que Lucie veut

« Un paquet natif npm pour utiliser rag3weaver, pour faciliter l'installation
partout — comme lucivy, qui a des CI/CD pour construire et publier chaque
binding. » Et, le 9 octobre :

- **GPU en option** ; **les embarquements en service par défaut**. Le cas
  échéant, on branche à la main un service d'embarquement — pour son père,
  l'accès Vertex de Lucie, *si les crédits startup couvrent aussi les
  embarquements* (à vérifier dans sa console, pas supposé).
- **La CI ne fait que publier**, vu la lourdeur du bâti (le fork de Kuzu en
  C++ dans chaque cible).
- **Des Docker de bâti par plateforme**, dont les configurations sont
  **commitées** pour qu'on les retrouve.

## 2. Ce qui existe, à réutiliser

- **lucivy** (`L-Defraiteur/lucivy`) : `bindings/nodejs` en napi avec ses
  `optionalDependencies` par plateforme ; `.github/workflows/release.yml`
  avec une matrice, une porte `PUBLISH_ENABLED` et la publication de
  confiance (OIDC) vers npm, PyPI et crates.io ; `publish_if_missing` qui ne
  republie jamais une version déjà là. C'est le squelette à copier.
- **rag3db** : `tools/nodejs_api` (binding Node du moteur seul, napi en C++,
  dernier bâti le 24 août — pas celui qu'on veut, mais la preuve que le
  moteur se lie à Node) ; le moteur se bâtit en **statique**
  (`add_library(rag3db STATIC …)`, `src/CMakeLists.txt`) ; ~26 Mo pour la
  bibliothèque partagée aujourd'hui.
- **rag3weaver** : le binaire `rag3weaver-backend` sert tout en HTTP avec son
  flux d'événements ; les modèles se déclarent en service ou en local
  (`model_source.rs`, `RAG3WEAVER_SERVICE_EMBED`, `RAG3WEAVER_EMBED_SERVICE`) ;
  `gcp_auth.rs` sait obtenir un jeton Vertex sans crate de plus — mais
  **aucun embarqueur Vertex n'existe** : l'embarquement en service parle à
  notre service (`rag3weaver-embeddings`), pas à Vertex.

## 3. Les deux formes, dans cet ordre

1. **Le paquet qui embarque le binaire.** `rag3weaver-backend` bâti par
   plateforme, chacun dans un sous-paquet `optionalDependencies`
   (`rag3weaver-linux-x64-gnu`, `rag3weaver-windows-x64`, `rag3weaver-darwin-arm64`, … — les mêmes noms que lucivy ; ou sous `@luciformresearch/`, qui porte déjà `codeparsers` et les anciens `ragforge`),
   et un paquet `rag3weaver` en JS qui lance le binaire et lui parle en HTTP
   et par son flux d'événements. Une seule API : celle qui existe. Marche
   depuis Node, Electron, Tauri, un script.
2. **Le binding napi en processus**, ensuite, quand un usage l'exige : une
   surface choisie (le catalogue, la recherche, le backend déclaré), pas tout
   le crate ; c'est un engagement d'API à tenir.

## 4. Ce qu'il y a dans le binaire publié

- **Features** : `rag3db-native`, `code`, `daemon` (le client du service
  d'embarquement), `openai-llm` ; **pas** `burn-embedder` par défaut.
- **GPU en option** : un second jeu de binaires avec `burn-embedder`
  (Vulkan), publié sous un autre nom de sous-paquet (`…-gpu`), ou choisi à
  l'installation. À trancher au premier bâti, selon ce que pèse wgpu.
- **Les poids** ne sont jamais dans le paquet : téléchargés au premier usage,
  comme aujourd'hui.
- **Le bac à sable** des commandes est Linux seulement (`landlock`) : sous
  Windows et macOS, le paquet le dit à l'ouverture, il ne fait pas semblant.
- **Le moteur en statique** dans le binaire : pas de `.so` / `.dll` à côté.

## 5. Les embarquements en service

- Par défaut, le paquet pointe vers un service d'embarquement déclaré
  (`RAG3WEAVER_EMBED_SERVICE=hôte:port`, plusieurs adresses possibles) : le
  nôtre, ou un `rag3weaver-embeddings` lancé par la personne sur une machine
  qui a un GPU.
- **Vertex** : il faudrait un embarqueur HTTP pour l'API d'embarquement de
  Vertex (modèle `gemini-embedding` ou équivalent), derrière le même trait
  `Embedder`, avec `gcp_auth.rs` pour le jeton. Deux faits à ne pas oublier :
  un index est lié à **un** modèle (un index fait avec Vertex ne se lit pas
  avec granite, et l'inverse) ; et les crédits startup de Lucie couvrent ou
  non les embarquements — **à vérifier avant d'y compter**.
- Le choix se fait à l'installation, par l'assistant du CLI prévu dans la
  vision générale (§3) : « quel modèle, comment le brancher ».

## 5 bis. La première ouverture : un assistant, contournable (Lucie, 10 octobre)

Lucie : « peut-être un wizard CLI au début qui te demande tout ce que tu
veux, ou bien contournable par paramètres ; un truc à la première ouverture
qui te pointe un fichier JSON où tu règles une config, ou qui te propose de
répondre étape par étape : choisir le service d'embarquement, créer
automatiquement votre service, ou un existant… ; ça te pointe le `.env` où
mettre la clé du LLM si tu en veux un. »

La forme retenue par l'orchestration, à concevoir en page après la démo du
paquet :

- une commande `rag3weaver init` (et la même chose au premier lancement sans
  manifeste) qui pose les questions **dans l'ordre où on en a besoin** :
  où est le code à indexer ; les embarquements — *servis par nous*, *un
  service existant* (adresse), *local sur votre carte* (le binaire GPU), ou
  *aucun pour l'instant* (plein texte seul, vecteurs en dette, dit tel quel) ;
  le modèle de langage — *aucun*, *une API compatible OpenAI*, *Anthropic*,
  *un serveur local* — et où va la clé ;
- **chaque réponse existe aussi en paramètre** (`--embed service=…`,
  `--llm anthropic`, `--non-interactive`), pour les scripts et la CI ;
- l'assistant **n'écrit que des déclarations** que le produit lit déjà : le
  manifeste de backend (`models.<capacité>`, le `workspace`) et un `.env`
  pour les secrets, jamais une clé dans le manifeste ; il dit où il les a
  écrits et se relance sans casser ce qui existe ;
- « créer automatiquement votre service » = lancer `rag3weaver-embeddings`
  sur la carte locale si le paquet GPU est là, ou dire ce qu'il faut
  installer sinon.

C'est l'« assistant dans le CLI » de la vision générale (§3 : « fait choisir
le modèle et la façon de le brancher »), rendu concret.

## 6. Le bâti : des Docker par plateforme, la CI ne fait que publier

Ce que Docker peut et ne peut pas faire, dit franchement :

| Cible | Bâti dans un Docker ? | Comment |
|---|---|---|
| Linux x86_64 (glibc) | oui | une image avec cmake, clang, Rust, Node ; c'est la plus simple |
| Linux aarch64 | oui | la même image sur un hôte ARM, ou `cross` (QEMU : lent mais marche) |
| Linux musl (Alpine) | à voir | le fork de Kuzu en musl n'a jamais été essayé |
| **Windows x64 (MSVC)** | **pas vraiment** | MSVC ne tourne pas dans un Docker Linux. Deux voies : `clang-cl` + `xwin` (les en-têtes et bibliothèques Windows dans un Docker Linux — ça marche pour Rust, c'est à prouver pour le fork C++), ou un **vrai Windows** : un runner GitHub `windows-latest`, ou le PC de son père comme poste de bâti |
| **macOS (arm64, x64)** | **non** | pas de toolchain Apple en Docker (osxcross existe, licence Apple floue) ; un runner GitHub `macos-latest`, ou un Mac à portée |

Donc la règle « la CI ne fait que publier » tient pour Linux ; pour Windows
et macOS, soit la CI bâtit quand même (runners GitHub, avec un cache du bâti
du moteur : `sccache` ou l'artefact du moteur gardé entre deux versions),
soit un poste réel bâtit et pousse l'artefact que la CI publie. À trancher au
premier essai, pas avant.

Les Docker vivent dans le dépôt, commités : `tools/build-images/<cible>/`
(un `Dockerfile` par cible, un `build.sh` qui rend le binaire dans
`dist/<cible>/`), et le workflow de publication prend ce qu'il trouve dans
`dist/` ou dans les artefacts d'une version. Les images elles-mêmes ne sont
pas commitées (publiées sur GHCR si on veut les partager entre machines).

## 7. Le premier geste

1. Un bâti **Linux x86_64 dans un Docker** du binaire `rag3weaver-backend`
   avec le moteur en statique et les features du §4 : c'est la mesure de ce
   que pèse le binaire et de ce que prend le bâti.
2. Le même sous **Windows**, sur un runner GitHub : la liste réelle des
   accrocs du moteur hors Linux (c'est aussi le préalable du logiciel pour le
   père de Lucie et du produit CAO, `../../visions/`).
3. Le paquet JS qui lance le binaire et lui parle ; un test qui installe le
   paquet dans un dossier vide et indexe trois fichiers.
4. Le `release.yml` copié de lucivy, publication de confiance, porte fermée
   tant que Lucie n'a pas dit « publie ». Le nom `rag3weaver` est **libre** sur npm
   (vérifié le 9 octobre) ; l'organisation `@luciformresearch` existe et
   porte déjà `codeparsers`. Lucie choisit le nom nu ou l'organisation.

Ce chantier ne touche aucun fichier des chantiers A à F : il vit dans
`tools/build-images/`, `.github/workflows/`, et un nouveau
`extension/rag3weaver/bindings/nodejs/`.
