# Chaque scope d'une ligne minifiée reçoit la ligne entière : 98 Mo de doublons dans un seul fichier

- **État** : ouvert (15 novembre 2026, session recherche, sur la mesure de lucivy).
- **Pour** : codeparsers (le contenu d'un scope). Le contournement côté rag3weaver, `workspace.index_exclude`, est sur master depuis 1bb929755, et le poser dans un manifeste est la décision de Lucie.
- **Famille** : produit
- **Gravité** : poids et bruit. Un fichier fait 57,6 % du texte du plein texte du dépôt. Une recherche qui touche ce fichier rend des centaines de scopes au même contenu.
- **Atteignable en service** : oui. La base du dépôt de Lucie (`code-rag3db`) indexe `extension/rag3weaver/templates/ide/app/app.js`, puisque son manifeste ne déclare pas `index_exclude`.
- **Touche rag3weaver** : oui (via codeparsers ; `exclusion.rs` pour le contournement).

## Ce que c'est

Mesure de lucivy le 15 novembre (banc `mesure-poids-familles`, cb3e2a0), sur une copie de l'instantané
`recherche-compte-orphelins-2-20261115-1500` du dépôt. Lucivy_Scope fait 1,39 Go pour 174,8 Mo de texte.
- **`extension/rag3weaver/templates/ide/app/app.js`** : 1 581 documents, **100,0 Mo de texte, soit 57,6 %
  de tout le texte**, dont **98,0 Mo de doublons à l'intérieur du fichier**.
- Le fichier fait 503 266 octets en 86 lignes ; sa plus longue ligne fait 133 294 octets. C'est le bundle
  de l'IDE.
- Chaque scope qui commence et finit sur la même ligne longue reçoit **la ligne entière** pour contenu, et
  non la portion de la ligne qu'il couvre.
- Le reste n'est qu'une longue traîne : utf8proc_data.h 1,9 Mo, json.hpp 0,9 Mo, catalog.rs 0,8 Mo…

Lucivy a rebâti l'index sans le .js, puis l'a compacté : −436,4 Mo, soit 36,4 % de l'index rebâti entier.
Sans app.js et après un compact, l'index passerait d'environ 1,39 Go à environ 0,76 Go.

## Pourquoi l'exclusion ne l'a pas écarté

`ExclusionPolicy` est vide quand `workspace.index_exclude` est absent : rien n'est écarté, et c'est voulu,
puisque les règles sont la décision de Lucie. Le gabarit `templates/backends/code/backend.json` les pose
(`minified_line_max_bytes` 5 000, `code_file_max_bytes` 1 Mio). Le manifeste du dépôt de Lucie
(`~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json`) est d'avant la règle et ne les a
pas. Avec la clé, app.js serait écarté comme minifié : sa ligne de 133 Ko dépasse le seuil de 5 000.

## Ce qu'il faudrait

1. **codeparsers** : le contenu d'un scope est sa portion de texte (colonnes de début et de fin), pas ses
   lignes entières, quand il ne couvre qu'une partie d'une ligne. Le témoin : un fichier d'une ligne à deux
   fonctions, et deux scopes au contenu distinct.
2. **La décision de Lucie** : poser `index_exclude` dans ses manifestes de code (code-rag3db, code-lourd,
   code-principal), puis réindexer. Les scopes d'app.js partent alors par la fin d'instantané, avec leurs
   documents depuis 262be6fc4, puis un compact rend la place.
