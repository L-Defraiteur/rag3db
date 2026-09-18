# Les produits, et comment ils s'empaquettent

**18 septembre 2026.** Conclusions d'une discussion avec Lucie, notées pour ne
pas les re-réfléchir. Rien ici n'est un chantier ouvert : c'est ce qu'on sait
vouloir, et ce que ça implique le jour où on le livre.

## Les quatre produits

1. **L'agent, façon Claude Code — la boucle étrange.** Un agent de code qui
   construit ses propres backends et bases dans son contexte : il déclare des
   entités, des relations et des dérivées (par gabarits du catalogue), ingère
   son dépôt, cherche, et refait tout ça pour chaque projet. Sur la machine
   de l'utilisateur : rag3weaver avec le moteur et ses extensions, lucivy, le
   démon d'embarquement (une carte ou le processeur, partagé par toutes les
   sessions), et le serveur MCP qui expose les outils.
2. **Le moteur de workflows cloud, façon Elasticsearch.** Un serveur chez
   Luciform, multi-locataire (les cellules `_Org` / `_Project` existent), une
   API, des graphes de dataflow comme unité de travail, des embarquements
   servis par nos cartes ou par Vertex.
3. **Le plan gratuit local du 2.** Le même serveur chez l'utilisateur : le
   binaire du 1 avec l'API à la place du MCP, ou les deux.
4. **La bibliothèque.** rag3weaver embarqué dans le programme de quelqu'un :
   crate Rust, roue Python, paquet Node. Le modèle de Kuzu.

Le 1 tire tout le reste : un binaire complet qui indexe un dépôt en trente
secondes et répond à un agent par MCP, c'est le 3 tel quel et le 2 derrière
une API.

## L'empaquetage

- **Tout dans le binaire.** Les extensions du moteur (`vector`, `geo`, le
  plein texte natif) **liées statiquement** dans `librag3db` — le moteur
  sait le faire pour la cible WASM, à généraliser en natif — ou, à défaut,
  embarquées dans le paquet et écrites dans un cache au premier chargement.
  Zéro téléchargement, zéro version à faire correspondre.
- **Les registres sont l'installeur.** PyPI : une roue par plateforme
  (`maturin`, `manylinux x64` et `aarch64`, `macosx arm64`, `win_amd64`),
  bibliothèque, moteur, extensions et démon dedans. npm : le patron
  d'esbuild, un paquet léger et un paquet par plateforme en
  `optionalDependencies`, liaisons par `napi-rs`. crates.io : la source,
  `build.rs` compile le moteur (dix minutes la première fois, comme le crate
  Kuzu), et `cargo binstall` pour le binaire de commande. Homebrew/winget
  plus tard pour le seul binaire.
- **La carte se détecte au premier lancement**, pas à l'installation
  (l'heuristique du premier index : granite-107m si ~50 000 documents ou
  carte < 4 Go ou absente, 278m sinon). Aucune dépendance système : burn
  passe par wgpu, Vulkan ou Metal sont déjà là ; ni CUDA ni ROCm à demander.
- **Les poids des modèles ne sont pas dans le paquet** (1,1 Go pour 278m).
  Consentement explicite, puis téléchargement depuis Hugging Face dans le
  cache partagé `~/.cache/huggingface`, une fois par machine. Les modèles
  sont libres (granite Apache 2.0, BGE-M3 MIT), pas de jeton à demander. Le
  refus laisse indexer en plein texte seul. `HF_ENDPOINT` respecté : un
  miroir sur luciformresearch.com le jour où il faut figer une version ou
  servir un réseau fermé.
- **Limites à connaître** : PyPI plafonne une roue à 100 Mo par défaut (on
  peut demander plus ; le moteur, les extensions et le démon tiennent
  dessous) ; quatre plateformes à compiler et tester à chaque version, une
  intégration continue à monter une fois.

## Ce qui en découle tout de suite

- **Pas de dépôt d'extensions.** La constante `extension.rag3db.com`,
  inventée par le renommage, disparaît : `INSTALL` sans dépôt nommé refuse
  clairement et nomme `RAG3DB_EXTENSION_REPO` (cœur C++, 18 septembre). La
  variable reste pour le jour improbable où des extensions optionnelles
  existeraient.
- **Ce que luciformresearch.com aura à servir** : le service du produit 2,
  la documentation, et au mieux un miroir des modèles. Pas les binaires du
  local — les registres s'en chargent.

## Ce qui n'est pas ouvert, et ne bloque rien

La liaison statique des extensions en natif, la chaîne de construction par
plateforme, l'installeur, le consentement au téléchargement du modèle (sa
place naturelle : l'entrée produit « indexer ce dépôt »). Tout ça attend le
jour où on met le produit entre les mains de quelqu'un ; d'ici là, l'ordre
de Lucie tient : replier les KB en entités dérivées, l'heuristique du premier
index, puis l'entrée produit.
