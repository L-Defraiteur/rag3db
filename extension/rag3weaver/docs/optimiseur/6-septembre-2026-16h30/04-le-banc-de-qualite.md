# Le banc de qualité : ce que chaque modèle retrouve dans notre code

**6 septembre 2026, 22 h 40, session « optimiseur ».** Le chantier A du
[03](03-chantiers-ouverts.md). La vitesse était mesurée
([01 §4](01-l-etat-du-moteur.md)) ; voici la qualité, sur *nos* requêtes et
*notre* code, pour que Lucie tranche le modèle d'indexation sur les deux
chiffres à la fois.

## 1. Le banc

`tests/e2e_banc_qualite.rs`, un test `#[ignore]` (`banc_qualite`) :

- **Le corpus** : 67 scopes **réels** de rag3weaver, lus dans `src/` au moment
  du test (embedder, burn_device, démon, scope, search, jaro, uuid, OCR, code,
  gcp_auth, postures, regime, fts, filter) et découpés comme l'indexation le
  fait : la doc au-dessus, la signature, le corps, coupé à 1 500 caractères
  (la limite façon ragforge). Les cibles des questions y sont, avec autant de
  voisins du même module pour que retrouver la bonne ne soit pas gratuit.
- **Les questions** : 45, 25 en français et 20 en anglais, écrites comme un
  développeur les taperait, sans le nom de la fonction (« se connecter au
  démon, ou le démarrer s'il n'est pas là », « fuzzy string similarity that
  tolerates typos »). Une cible chacune, parfois deux voisines acceptées
  (`fuse_signals` / `fuse_results`).
- **Deux variantes** : le code tel quel — nos commentaires sont en français,
  ce qui avantage le français — et le code **sans aucune ligne de
  commentaire**, ce qu'on trouverait dans un dépôt commenté en anglais ou nu
  (le noyau Linux, une bibliothèque tierce).
- **La mesure** : MRR, rappel à 1 et à 5 (cosinus, tout le corpus classé), la
  taille d'index pour 20 000 chunks (dim × 4 octets), et la liste des questions
  ratées à 5 avec ce que le modèle a vu à la place.

```text
RAG3WEAVER_SANS_DEMON=1 RAG3WEAVER_BURN_DEVICE=gpu:1 \
  cargo test --features burn-embedder,daemon --test e2e_banc_qualite \
  -- --ignored --exact banc_qualite --nocapture
RAG3WEAVER_BANC_MODELES=granite-107m,bge-m3   # pour n'en mesurer que certains
```

`RAG3WEAVER_SANS_DEMON=1` est là pour ne pas toucher au démon en place : il
peut appartenir à l'autre session et servir un autre modèle, et un test qui
le trouve périmé le remplacerait.

## 2. Le tableau

Carte TV, Vulkan, Flex32, autotune, fusion. « sans // » : le corpus sans
commentaires.

| modèle | dim | MRR | R@1 | R@5 | MRR sans // | R@1 | R@5 | index 20 k | temps |
|---|---|---|---|---|---|---|---|---|---|
| granite-107m | 384 | 0,779 | 0,62 | **1,00** | 0,738 | 0,60 | 0,91 | 30 Mo | 0,9 s |
| **granite-278m** | 768 | **0,844** | **0,71** | **1,00** | **0,812** | **0,73** | 0,93 | 61 Mo | 2,2 s |
| BGE-M3 | 1024 | 0,793 | 0,62 | **1,00** | 0,797 | 0,67 | **0,96** | 81 Mo | 5,7 s |
| MiniLM anglais | 384 | 0,610 | 0,49 | 0,71 | 0,618 | 0,49 | 0,73 | 30 Mo | 0,7 s |
| MiniLM multilingue | 384 | 0,573 | 0,38 | 0,82 | 0,553 | 0,38 | 0,80 | 30 Mo | 0,2 s |

Le temps est celui des quatre passes (deux corpus, deux jeux de questions),
modèle chaud ; la vitesse comparée est dans le 01.

## 3. Ce qu'il dit

- **granite-278m est le meilleur des cinq, devant BGE-M3**, sur les deux
  corpus, à 2,5 fois la vitesse et aux trois quarts de l'index. Ce n'est pas
  une surprise : IBM l'a entraîné pour la recherche de code, BGE-M3 pour tout.
- **granite-107m tient à 6 points de MRR de 278m avec les commentaires, à
  7 sans**, et à 9 points de rappel à 1 (0,62 contre 0,71). Il retrouve tout
  dans les cinq premiers quand le code est commenté ; sans commentaires il
  en perd quatre sur 45. Deux fois plus rapide, un index deux fois plus petit.
- **Les deux MiniLM ne sont pas au niveau** : un quart à un cinquième des
  questions hors des cinq premiers, sur les deux corpus. L'anglais seul ne
  s'effondre pas sur le français grâce aux identifiants ; le multilingue
  (fenêtre 128 jetons, chunks tronqués) fait pire.
- **Ce que les trois bons ratent est le même** : `jaro_winkler` (rien dans le
  code ne dit « faute de frappe », les trois vont vers `precision_par_defaut`
  ou `validate_id`) et le `token` de gcp_auth (ils préfèrent `from_env`, qui
  construit le même client : une cible discutable). Le reste est trouvé.
- **Les commentaires en français aident le français** de 4 points de MRR sur
  granite-107m, de 3 sur 278m, de rien sur BGE-M3. Un dépôt anglais efface
  cet avantage, pas le classement.

## 4. Ce que je recommande

**Tranché par Lucie le 7 septembre 2026** (relayé par la session
architecture) : granite-278m par défaut, granite-107m en régime rapide ; le
choix par heuristique de taille de dépôt viendra avec l'index à plusieurs
embarquements (session rag3db-40). Le démon suit (`MODELE_PAR_DEFAUT`), le
garde-fou `check_embedding_model` refuse un index existant construit en
107m, c'est voulu, et son message nomme les deux modèles.

**granite-278m par défaut** : meilleur que BGE-M3 sur notre code, 41 000 à
49 000 jetons par seconde (BGE-M3 : 22 000), 61 Mo d'index pour 20 000
chunks. Sur le cœur C++ de rag3db (20 132 chunks, 23 s de modèle avec 107m),
ça fait ~45 s de modèle, l'ingestion sous 75 s au lieu de 51 — toujours sous
trois minutes pour 5 000 fichiers.

**granite-107m en régime rapide** (`RAG3WEAVER_EMBED_MODEL=granite-107m`) :
pour un premier index d'un très gros dépôt (le noyau, 90 000 fichiers) où
l'on accepte 9 points de rappel à 1 contre un modèle deux fois plus vite et
un index deux fois plus petit. Le reranker rattrape une partie de l'écart
puisque le rappel à 5 est le même.

**BGE-M3 reste le second étage** (creux appris, 8 192 jetons) et le modèle
des documents non-code ; il ne gagne rien sur le code.

Le banc est petit (45 questions, 67 scopes) : les écarts de 2 à 3 points
sont du bruit, ceux de 6 à 9 ne le sont pas, et l'ordre des cinq est net.
Pour l'affiner : des requêtes réelles de la suite `e2e_search`, et un corpus
pris dans un dépôt anglais (le noyau), sans nos identifiants en français.

## 5. Ce que le banc a trouvé en passant

La ligne `[rag3weaver] burn : … · précision …` disait **« f32 »** pour tout
modèle chargé après le premier dans un processus : burn ne configure une
carte qu'une fois (`AlreadyInitialized` au second `configure`), et notre
message en déduisait la précision par défaut. La carte restait en Flex32, les
poids aussi (le chargeur suit `float_dtype_voulu()`, pas la carte) : le
calcul était juste, la ligne mentait. Corrigé dans `burn_device::resolve` :
on relit `device.settings().float_dtype` et on dit « Flex32 (déjà posée sur
cette carte) », ou la vraie précision si elle diffère de celle demandée.
