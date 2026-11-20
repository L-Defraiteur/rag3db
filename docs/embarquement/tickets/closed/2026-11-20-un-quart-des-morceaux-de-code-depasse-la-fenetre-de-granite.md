# Un quart des morceaux de code dépasse la fenêtre de granite

- **État** : fermé le 20 novembre 2026 par le lot des morceaux d'everything declarative (`chunk-bound-from-embedder`, règles de l'analyse 14). Voir « Fermeture ».
- **Pour** : la session qui découpe le code (`default_scope_chunking`, `src/code.rs`) ; embarquement pour la mesure
- **Famille** : produit
- **Gravité** : haute pour la recherche par le vecteur. Sous granite-278m, la fin de **près de trois morceaux sur quatre** n'est pas
  cherchable par le vecteur. Elle est coupée à 512 jetons par le modèle ; la ligne reste entière en base.
- **Atteignable en service** : oui, tout index de code embarqué par granite (le 7878 de luciepc)
- **Touche rag3weaver** : oui (`src/code.rs`, `default_scope_chunking` ; `src/chunker.rs`)

## Ce que c'est

Le code se découpe par lignes : 30 lignes ou 1 500 caractères (`default_scope_chunking`, `src/code.rs:223`). Granite
tronque à 512 jetons (`MAX_SEQ_LEN`). Mesuré le 20 novembre avec le tokenizer de granite, sur les 311 fichiers `.rs` de
`extension/rag3weaver` (master du 20 novembre), découpés comme le fait le crate :

| `max_size` | Morceaux | Au-dessus de 512 jetons |
|---|---|---|
| 1 500 (aujourd'hui) | 8 843 | **2 241 (25,3 %)** ; médiane 448, p95 587, max 742 jetons |
| 1 300 | 9 601 | 546 (5,7 %) |
| 1 200 | 10 190 | 74 (0,7 %) |
| 1 100 | 10 968 | 6 (0,1 %) |
| 1 000 | 11 991 | 1 (0,0 %) |

Ce Rust, commenté en français, compte environ 0,45 jeton par caractère : 1 500 caractères en font jusqu'à 740. Le
compteur du 7878 confirme l'ordre de grandeur : `truncated_texts` vaut **146 463** le 20 novembre à 1 h 31, depuis son
démarrage du 19 à 10 h 47, chauffe comprise (169). C'est surtout le repassage de la bascule du soir. Le service ne
compte pas le total des textes qu'il embarque.

La mesure ne compte pas l'en-tête que l'index ajoute peut-être au texte d'un morceau : la réalité est au moins aussi
haute.

## Ce qu'il faudrait

- **Une taille de morceau à la mesure du modèle**, pas une constante : bge-m3 a 8 192 jetons de fenêtre, granite 512.
  Le plus simple serait `max_size` en caractères déduit de la fenêtre du modèle déclaré (`max_tokens_per_text` de
  l'identité du service, lot `token-cap`), à environ 2,2 caractères par jeton pour le code, avec une marge. Le plus
  juste : un découpage compté en jetons (`text-splitter` sait dimensionner par un tokenizer).
- **D'ici là, pour granite** : 1 100 caractères (0,1 % au-dessus de 512), au prix de 24 % de morceaux en plus.
- **La preuve** : le compteur `truncated_texts` du service, et la ligne du journal d'index qui nomme les morceaux coupés
  (lot `truncation-relay`, en file), sur un index de rag3db avant et après.

Le script de mesure est dans la page des bornes (`docs/embarquement/18-11-2026/02-…`, point C).

## Taille

À voir par la session qui découpe ; la mesure se rejoue en une minute.

## Corrigé le 20 novembre, 2 h : le contexte compte, et c'est pire

La mesure ci-dessus découpait sans le **contexte**. Or `chunk_lines` (`src/chunker.rs:153`) ajoute au cœur (30 lignes
ou `max_size` octets) jusqu'à `overlap_lines` (5) lignes de chaque côté, et **chaque côté** est borné à `max_size`
octets, pas le morceau entier. Le texte embarqué (`_text`, sans titre ni en-tête : `text_field` vaut `_text`,
`src/dataflow/record_nodes.rs:2139`) est donc le cœur plus son contexte. Mesuré avec `chunk_lines` porté à l'identique
(`docs/embarquement/20-11-2026/mesure-des-morceaux-en-jetons.py`), sur les mêmes 311 fichiers, découpés fichier par
fichier :

| `max_size` | Morceaux | Au-dessus de 512 jetons | Médiane | p95 | Max |
|---|---|---|---|---|---|
| 1 500 (aujourd'hui) | 8 859 | **6 420 (72,5 %)** | 598 | 856 | 1 231 |
| 1 100 | 11 082 | 5 748 (51,9 %) | 517 | 714 | 1 105 |
| 900 | 13 472 | 3 732 (27,7 %) | 455 | 637 | 958 |
| 700 | 17 465 | 1 844 (10,6 %) | 391 | 563 | 799 |
| 600 | 20 542 | 1 214 (5,9 %) | 358 | 522 | 793 |

- **Réduire `max_size` ne suffit pas** : le contexte double presque le morceau. Il faut **borner le morceau entier,
  cœur et contexte**, sous la fenêtre du modèle. Le contexte s'arrête quand le total atteindrait la borne.
- **Une réserve** : le crate découpe chaque scope à part, et le contexte ne sort pas d'un scope. Découper le fichier
  entier, comme ici, donne plus de contexte que le réel pour les petites fonctions. La part réelle est sans doute plus
  basse, mais du même ordre pour les fonctions longues. Le script se rejoue en une minute :
  `uvx --with tokenizers python -I mesure-des-morceaux-en-jetons.py <dossier> 1500,1100,900`.
- **La forme attendue par c7** : la borne du morceau entier se déduit de la déclaration de l'embarqueur (sa fenêtre
  `max_tokens_per_text`, et un ratio de jetons par caractère déclaré par modèle : environ 0,45 pour granite sur ce
  Rust), pas une valeur en dur ; bge-m3 garde 1 500.

## Mesuré par l'index, scope par scope (20 novembre, 9 h 5x)
corpus, en plein texte seul et sans service. La comparaison porte sur master et sur la tête des morceaux d'ED
(`chunk-bound-from-embedder`, 1e082624c2, découpage pour granite), lib 4ad394f4d2. Les `_text` des morceaux sont
comptés par le tokenizer de granite.

| Corpus | Master : morceaux | > 512 jetons | ED : morceaux | > 512 jetons |
|---|---|---|---|---|
| `extension/rag3weaver/src/**/*.rs` (312 fichiers) | 24 479 | **3 206 (13,1 %)**, max 1 175 | 26 621 (**+8,7 %**) | **0**, max 510 |
| `src/include/**/*.h` du moteur (810) | 8 738 | **436 (5,0 %)**, max 1 290 | 9 083 (**+3,9 %**) | **0**, max 510 |

- **Le lot d'ED tient la promesse** : aucun morceau au-dessus de la fenêtre de granite, pour 4 à 9 % de morceaux en plus.
- **La part réelle sur master est 13,1 %** pour le Rust, et non les 72,5 % de l'estimation par fichiers entiers. Les
  scopes sont petits (médiane de 110 jetons), et le contexte ne sort pas d'un scope. La réserve de l'estimation tenait.
- Le témoin de mesure était jetable (jamais commité) ; les journaux sont à luciepc dans
  `~/.cache/rag3weaver-build/mesure-point-c/`.

## Fermeture (20 novembre 2026, session everything declarative)

**Le correctif**, dans le lot des morceaux (règles de l'analyse 14) :
- **la borne vaut pour le morceau entier**, cœur et contexte, la queue fusionnée et un texte gardé entier ; le contexte
  cède d'abord, une ligne à la fois de chaque côté (`ChunkingConfig::max_chunk_size`) ;
- **elle vient du plus petit embarqueur de la base**, celui que le backend déclare et ceux dont la base garde des
  vecteurs : fenêtre × marge ÷ ratio en octets d'abord (1 024 pour granite), puis en jetons comptés par son propre
  tokenizer (`src/tokens.rs`) ;
- **le tokenizer est une capacité** : s'il manque, la base refuse de s'ouvrir pour l'index en nommant le fichier, et
  jamais un découpage par les octets seuls ;
- bge-m3 seul garde ses 1 500.

**La preuve** : la mesure par l'index ci-dessus, sur la tête du lot. Aucun morceau au-dessus de 512 jetons sur les deux
corpus entiers (max 510), pour 8,7 % et 3,9 % de morceaux en plus. Les témoins du lot, vus rouges puis verts :
- tous les en-têtes du moteur découpés fichier par fichier : un morceau à 602 jetons sans l'étage des jetons, aucun
  au-dessus de 512 avec ;
- une base de code qui indexe quatre en-têtes denses scope par scope : 21 morceaux sur 184 au-dessus de 512 avec le
  découpage d'avant (jusqu'à 1 290), aucun avec la borne ;
- un modèle sans tokenizer refusé, en nommant le fichier.
