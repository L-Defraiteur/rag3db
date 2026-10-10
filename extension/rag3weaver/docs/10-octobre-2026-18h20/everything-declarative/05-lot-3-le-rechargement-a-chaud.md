# Lot 3 — le rechargement à chaud

11 octobre 2026, session « everything declarative » (chantier I). Un backend
déclaré se recharge en service : on modifie un fichier, on recharge, la
nouvelle version répond ; si elle est invalide, l'ancienne reste.

## 1. Ce que c'est

- `Backend` tient ses déclarations dans un `RwLock<Arc<PreparedBackend>>`.
  **Un appel prend la version courante à son départ** (`backend.prepared()`)
  et la passe de bout en bout : outil, crochets avant et après, validation,
  exécution du graphe. Un rechargement pendant l'appel ne change rien à
  l'appel en cours ; le suivant prend la nouvelle version.
- `Backend::reload()` relit le manifeste et tout ce qu'il nomme (graphes des
  outils et des crochets, scripts, schémas JSON, réactions) avec
  `PreparedBackend::load`, donc **revérifié exactement comme au
  chargement** ; puis remplace la version d'un seul geste et rend
  `Reloaded { version }` (0 à l'ouverture, +1 par rechargement accepté ;
  `backend.declarations_version()`).
- **Refusé, l'ancienne version reste en service**, avec une erreur qui dit
  quoi corriger (« rechargement refusé, l'ancienne version reste en
  service : … ») : un fichier invalide ou absent, un type de nœud inconnu,
  et tout ce qui a été **fixé à l'ouverture** de la base — `database`,
  `embeddings`, `models`, `vector_extension`, `workspace`, `fts_positions`,
  `buffer_pool`, `relations`, `entities` (comparées après fusion de leur
  schéma JSON, sans leur description). Un changement de schéma est une
  migration (vision §12), pas un rechargement : le refus le nomme.
- `PreparedBackend` garde le chemin canonique de son manifeste.
- Le binaire `rag3weaver-backend` ne change que d'une ligne
  (`backend.prepared().describe()`).

## 2. Les témoins (`src/backend_reload_tests.rs`)

Un backend « toy » écrit dans un dossier temporaire (un outil `echo` dont le
graphe appelle un script rhai), monté sans base (une connexion qui ne rend
rien). Rouges avec un bouchon (7), puis 7 sur 7.

- un script modifié change la sortie sans redémarrer — et rien ne change
  avant le rechargement ;
- un graphe modifié change la sortie ;
- un graphe invalide (`NoSuchNode`) est refusé en le nommant ; l'ancienne
  version répond toujours, le numéro ne bouge pas ;
- un script supprimé est refusé ; l'ancienne version répond ;
- **un appel parti avant le rechargement finit sur l'ancienne version**
  (la version qu'il a prise au départ, rejouée après le rechargement, rend
  l'ancienne sortie ; un appel nouveau rend la nouvelle) ;
- un changement de `database` est refusé comme migration ;
- deux rechargements comptent deux versions.

## 3. Ce qui n'est pas dans ce lot

- **La montre sur les fichiers.** Elle passera par le réacteur, sur le
  montage `Reactor::watch_bound` de la session mémoire (arrêt attendu,
  portée du run déjà réglés) : un fichier du dossier du backend change, la
  réaction appelle `reload()`, et le rechargement émet sur le bus
  « rechargé : tel fichier, telle version » ou « refusé : pourquoi » — ce que
  la page vivante (lot 6) écoutera. Aujourd'hui `reload()` s'appelle
  explicitement.
- **Les gabarits de rendu** ne sont pas déclarés par un backend : ils sont
  lus à chaque rendu dans `templates/render` (déjà « à chaud », sans
  revérification). Les vues du lot 4 seront des déclarations du backend, et
  se rechargeront avec lui.
- **Les nœuds scriptés du dossier `nodes/`** entreront dans `PreparedBackend`
  au lot 4 ; ils se rechargeront alors avec le reste, sans fuite (lot 2).
