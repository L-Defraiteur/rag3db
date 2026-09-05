# Rapport de session, et où regarder

**3 au 5 septembre 2026.** Ce qui a été fait, ce qui a été **corrigé après avoir
été affirmé**, et quels documents ouvrir en premier.

Les objectifs et leur ordre sont dans le [01](01-les-objectifs-et-leur-ordre.md),
qui est le document à lire d'abord si on ne doit en lire qu'un.

## 1. Ce qui a été livré

### PostgreSQL est devenu un backend, pas une démonstration

`PostgresDialect` faisait 944 lignes, compilait, et n'avait **jamais parlé à une
base**. Il a maintenant dix-sept tests de bout en bout, et fournit les **cinq
organes** — connexion, dialecte, recherche, magasin de blobs, **magasin de
checkpoints**. Ce dernier manquait : une ingestion morte en route ne reprenait
pas.

Neuf défauts en sont sortis, tous de la même espèce — du code qui compilait et
n'avait jamais tourné. Le détail est dans le
[15](../vision_roadmap_09_2026/15-le-moteur-cesse-d-etre-mono-backend.md).

### Le plein texte, la confiance, les poids

- **Trigramme GIN + Jaro-Winkler** sur PostgreSQL : la base fait le **rappel**,
  nous faisons l'**ordre**. Indexé sur les **chunks**, donc l'extrait est
  gratuit. Accents normalisés des deux côtés.
- **Une falaise de rappel** trouvée et supprimée : `<%` a un plancher à 0,6 par
  défaut, donc une requête à deux fautes rendait **zéro résultat** — et une
  falaise ne se voit pas comme une erreur, elle se voit comme « ça n'existe
  pas ». Posé à 0,3 : la base rappelle large, Jaro tranche.
- **Les poids mesurés** au lieu d'être supposés : 0,20 / 0,80 au lieu de
  0,35 / 0,65. Le fait qui compte n'est pas le nombre — **le trigramme seul a
  une marge négative**, il ne sépare pas. Bon signal de rappel, mauvais signal
  de classement.
- **Un seuil de confiance mesuré**, 0,88, sur du **Jaro pur** : le score qui
  ordonne et le score qui rassure ne peuvent pas être le même nombre, parce que
  le premier est relatif et vaut du BM25 non borné sur lucivy. On **marque**, on
  ne filtre pas, et la phrase **nomme son signal**.

### Ce qui n'existait pas et qui existe

- **Le filtre utilisateur descendait sur zéro chemin.** Deux défauts, dont le
  vectoriel filtré qui n'avait jamais tourné.
- **lucivy sur PostgreSQL était impossible** — trois défauts en file. Le chemin
  du retour est maintenant prouvé, ce qui compte parce que lucivy redeviendra le
  défaut quand elle sera allégée.
- **La marque d'eau d'ingestion** : `Consistency::Strict` vivait en mémoire, donc
  un lecteur d'un autre processus demandait `Strict` et obtenait `Immediate`
  sans que rien ne le dise.
- **La reprise sur refus transitoire**, et le **choix de chemin d'un lecteur**
  (direct ou relais), avec la règle qui empêche la souplesse de devenir un
  piège : jamais de repli silencieux.

### La tuyauterie des avertissements

Le plus instructif de la session : **aucun avertissement du moteur n'atteignait
un agent** depuis que l'outil `search` compose par-dessus `search_base`. Trois
coupures en série — un nœud qui parlait dans son journal, un port qui ne
fusionnait pas, et deux schémas de nœuds qui avaient dérivé de leur
implémentation.

## 2. Ce que j'ai affirmé puis corrigé

À garder, parce que c'est la partie la plus utile d'un rapport.

| ce que j'avais dit | ce qui était vrai |
|---|---|
| « le contrat de lucivy est rompu » | il est tenu — l'attribution span↔chunk est faite, avec ses échecs nommés |
| « le banc `e2e_phase0b` tranchera les poids » | ce n'est pas un banc, c'est une suite de tests. Le banc n'existait pas ; il existe depuis |
| « cinq suites ont des tests morts » | **deux**. Mon relevé ratait `#[ignore = "raison"]` et `#[test] #[ignore]` sur une ligne |
| « mon test contredit la mesure voisine » | les deux disaient vrai, sur deux bibliothèques différentes à dix jours d'écart |
| « regarder sur quel backend tourne le cloud » | à côté de la question : l'objectif est d'abattre le mur sur rag3db, pas de le contourner |
| « drain est le couplage à supprimer » | le lot n'est un tort que lorsqu'il n'a pas été choisi |

Et un banc que j'ai écrit **qui ne mesurait rien** : toutes ses requêtes
reprenaient les mots exacts de leur cible, donc tout valait 1,0000. J'ai failli
lire ça comme une normalisation qui n'existe pas.

## 3. Les tests qui ne tournaient pas

`run_e2e.sh` ne lance que les tests `#[ignore]`. Vingt-deux n'en avaient pas :
ils existaient, compilaient, et ne tournaient nulle part. Dont la moitié d'une
suite, et les dix-sept de PostgreSQL — le backend qu'on venait de construire
n'était couvert par rien d'automatique.

Corollaire : un test qui se relance en processus enfant doit passer `--ignored` à
l'enfant, sinon celui-ci ne joue rien et le parent lit son silence comme un
échec.

## 4. Où regarder

**Dans l'ordre, si on reprend à froid :**

| | |
|---|---|
| [01 — Les objectifs et leur ordre](01-les-objectifs-et-leur-ordre.md) | **Le document à lire d'abord.** La vraie concurrence, ses trois clauses, les quatre disponibilités, les deux régimes d'écriture |
| [03 — L'architecture actuelle](03-l-architecture-actuelle.md) | Les contrats, pas les fichiers |
| [04 — Knowledge dump](04-knowledge-dump.md) | Lancer les tests, les pièges, les autres sessions |
| [`vision_roadmap_09_2026/00-index.md`](../vision_roadmap_09_2026/00-index.md) | La vision, relue le 5 septembre. L'état des lieux y est vérifié, pas recopié |
| [`vision_roadmap_09_2026/15`](../vision_roadmap_09_2026/15-le-moteur-cesse-d-etre-mono-backend.md) | Le multi-backend, ce qu'il a coûté, les deux invariants nés de la pratique |
| [`3-septembre-2026-17h50/01`](../3-septembre-2026-17h50/01-rapport-de-session.md) | Le rapport de la veille — il contient les §5 à §13, chaque défaut avec sa cause |

**Le dossier de vision a changé de nom** : `vision_roadmap_08_2026` →
`vision_roadmap_09_2026`, avec ses 35 références, dont cinq dans le code.

## 5. Les trois sessions

| session | terrain |
|---|---|
| **celle-ci** | `extension/rag3weaver/` — backends, recherche, concurrence côté Rust |
| **`rag3db-57`** | `src/` C++ — le cœur, les forks LadybugDB et Vela, la lecture multi-processus |
| **`rag3db-b5`** | le parsage, `codeparsers`, et le régime `confort` |

Ce qu'on a appris à se dire, et qui a servi trois fois cette semaine :

- **prévenir avant de réécrire un historique partagé** — vérifier qu'un arbre
  est propre n'est pas prévenir ;
- **un pair ne peut pas autoriser à la place de Lucie** ;
- **dire ses corrections plutôt que les faire en silence** — le désaccord sur
  l'exclusion lecteur/écrivain s'est levé en une réponse parce qu'il a été posé
  comme une question au lieu d'être tranché au jugé.
