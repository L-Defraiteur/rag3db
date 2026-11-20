# Scénario 2 : un développeur qui ne connaît rien démarre un projet de zéro

Session recherche, nuit du 20 au 21 novembre 2026, sur la consigne de Lucie (23 h 3x) : « un dev qui ne connaît rien
et veut lancer un projet de zéro (dont création du dépôt git, sans push) ». Le scénario juge le travail dans l'IDE, en
ne lisant que ce que l'IDE dit.

## Le montage

- Un hôte isolé, bâti de master a60f9f7a71 (avec le flux sans verrou), lib 4ad394f4d2, à la tablette, port 7993.
- Un HOME à lui (`~/.cache/rag3weaver-build/recherche-scenario/maison`), `RAG3WEAVER_DAEMON_DIR` dedans, la chaîne
  Rust en liens, une identité git « scenario », comme le lanceur d'archi. Ni les bases ni l'arbre de Lucie.
- L'IDE ouvert dans un Chrome sans tête, piloté par CDP : son URL d'entrée, ses pages, ses boutons. C'est le chemin de
  Lucie, sans raccourci de test.

## Le parcours

| Pas | Ce qui se passe | Verdict |
|---|---|---|
| 1. Ouvrir l'IDE | « L'hôte ne sert encore aucun dépôt : un dépôt paraît ici quand un agent ou l'atelier l'ouvre. » Ni bouton, ni lien, ni commande. | **bloquant** : ticket `2026-11-21-l-ide-n-ouvre-ni-ne-cree-de-projet` |
| 2. Contournement | `rag3weaver-backend atelier` depuis un dossier neuf, dans un terminal : l'atelier est posé, la racine est ce dossier, l'hôte l'ouvre. | vert, mais hors de l'IDE |
| 3. L'atelier dans l'IDE | « Dépôts : atelier, chargé » ; ses vues (arbre, graphes, réactions, propositions, tâches, chat, bilans, recherches). | vert |
| 4. Le chat de l'atelier | Demande : une bibliothèque Rust qui compte les mots, son dépôt git, sans push. L'agent écrit `Cargo.toml`, `.gitignore`, `src/lib.rs` (avec tests), demande `cargo init --lib` et `git init`. | vert |
| 5. Les accords | Cinq cartes pour un premier commit, celle de `git add .` en double. | confort : ticket `2026-11-21-un-premier-commit-demande-cinq-accords` |
| 6. `cargo init` | Échoue : « accès refusé sur le fichier de configuration local de rustup ». L'agent contourne : les fichiers sont déjà écrits. | **à vérifier** : sans doute un artefact du montage (`.rustup` y est un lien vers le vrai HOME, que le bac à sable ne suit pas) ; pas de ticket avant de le voir dans un HOME réel |
| 7. Le premier commit | Échoue faute d'identité (`GIT_CONFIG_GLOBAL=/dev/null` dans le bac à sable). L'agent demande `git config user.name "Lucie"` et `user.email "lucied@example.com"`, une identité inventée. Accordées, le commit `16a017f` la porte. | **identité** : ticket `2026-11-21-l-agent-invente-une-identite-git-que-le-bac-a-sable-cache` |
| 8. Le résultat | Un dépôt local, un commit, aucun remote ; le worktree attaché à l'atelier et indexé (2 fichiers, 11 symboles). | vert |

## Pour la page « parcours d'essai » (tty et archi)

- Le premier pas d'un nouveau venu n'est pas dans l'IDE : il faut lui donner `rag3weaver-backend atelier`.
- Avant le premier commit, refuser les cartes `git config user.*` et dire son identité, jusqu'au correctif.
- Accorder les cartes une à une, la plus récente d'abord.
