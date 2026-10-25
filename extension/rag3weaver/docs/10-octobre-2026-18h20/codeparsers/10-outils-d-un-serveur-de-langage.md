# Les outils d'un serveur de langage, en gabarits sur notre graphe (graine)

*Étude de codeparsers, 25 octobre 2026, lecture seule, à la demande de l'orchestration : une
graine à lire avant d'écrire, pas un lot. Sources : le gabarit code et `src/code.rs` sur master
(`2c44d9f38`), relus par un repérage ; citations relatives à `extension/rag3weaver/`.*

L'idée : reproduire en gabarits, sur notre graphe, les outils qu'un agent attend d'un serveur
de langage. Ce sont la définition, les références, le type sous le curseur, le plan du fichier,
les implémentations et la hiérarchie d'appels. Chaque outil serait un `.mmd` qui passe par l'IR
du Plan, et non du code Rust par outil. Avant d'écrire quoi que ce soit, voici ce que nous avons
et ce qui nous manque.

## Ce que nous avons, outil par outil

| outil d'un serveur de langage | le plus proche chez nous | ce qui manque |
|---|---|---|
| aller à la définition | `usages` (`templates/tools/usages.mmd`) rend les définitions d'un **nom** | partir d'une position (fichier, ligne, colonne) |
| références | `usages`, groupés par genre, avec fichier:ligne | un site par référence ; nous avons une arête par (scope, nom) |
| type sous le curseur | rien | le type lu se garde par nom, agrégé ; aucun type par position |
| plan du fichier | rien comme outil ; `scopes_of` et `narrowest` (`src/code_tools.rs:446-476`) le font en interne | un outil, et le parent et la signature dans la lecture |
| implémentations | `impact`, par `also_path` (IMPLEMENTS, INHERITS_FROM) | la liste directe des implémentations d'une méthode de trait |
| appels entrants / sortants | `impact` / `callees` (`NeighborhoodNode`) | rien d'essentiel : c'est notre point fort |

## Ce qui manque dans le graphe, du plus simple au plus profond

1. **Le plan du fichier est à portée de main.** Les scopes ont `start_line`, `end_line`,
   `parent_name`, `signature` et `scope_type` (`src/code.rs:282-355`). Un gabarit qui lit les
   scopes d'un fichier, triés, avec leur parent, est un Plan déclaré : un `select` sur Scope
   filtré par `file_path`. Il n'y a rien à ajouter au graphe.
2. **Aucune colonne.** Ni les arêtes ni les scopes ne gardent de colonne : `UsageSite` ne porte
   que `{usage, line}`, et `usage_properties` ne garde que la ligne la plus basse. Un outil « sous
   le curseur » ne peut donc travailler qu'à la ligne près. Deux noms sur la même ligne
   (`a.get(b.get())`) se confondent. Les octets d'un scope (`start_byte`, `end_byte`) existent,
   mais pas ceux d'une mention.
3. **La résolution est agrégée par (scope, nom), pas par référence** (`types_lus`,
   `pending_qualifier_types`, `src/code.rs:1383-1607`). Deux appels `x.f()` de types différents
   dans le même scope donnent une seule arête vers f. Une référence sans type lu efface le type
   des autres, ce que le test `le_rendez_vous_porte_le_type_du_qualificatif` défend exprès. Un
   « aller à la définition » exact demande une mention par site, avec sa ligne, sa colonne et son
   type. C'est un changement de schéma (MENTIONS par site), et le volume des mentions se
   multiplierait.
4. **Les locales et les paramètres ne sont pas des scopes.** Un serveur de langage les résout ;
   nous les écartons (`LocalScope`). Un « aller à la définition » sur une variable locale ne
   trouverait rien : il faudrait lire le fichier, pas le graphe.
5. **La précision elle-même.** L'oracle clangd (`scripts/oracle_clangd.py`) mesure nos CONSUMES
   C++ : sur 300 tirées au hasard, 75 % sont justes, 8 % fausses et 12 % muettes. Elles sont
   fausses à 17 % quand elles sont résolues par le seul nom. Un serveur de langage promet 100 %.
   Nos outils doivent donc dire la marque de chaque réponse (`resolution`, déjà rendue par
   `usages`), et non se présenter comme un compilateur.

## Ce que l'IR du Plan apporte

Un Plan déclaré (`ir/src/declared.rs`, `src/dataflow/plan_node.rs`) est un graphe de formes
`select` / `count` / `hop` en lecture seule. Il est vérifié au chargement contre le schéma et
exécuté dans un seul instantané. Les outils 1 (plan du fichier), « références d'un nom » et
« implémentations d'une méthode » s'écrivent ainsi, sans Rust. Aucun dossier `plans/` n'existe
encore sous `templates/` : le plan du fichier serait le premier, et le plus petit.

## L'ordre proposé, si l'on y va

1. **`outline`** (le plan d'un fichier) en Plan déclaré : rien à changer au graphe, et il sert
   déjà à un agent qui lit un fichier.
2. **`implementations`** d'une méthode de trait ou de base, en Plan : un saut par HAS_PARENT,
   IMPLEMENTS/INHERITS_FROM et le nom.
3. **`definition_at`**, à la ligne près : le scope le plus étroit à cette ligne, puis ses
   CONSUMES dont le nom figure sur la ligne. Il faut dire qu'il est à la ligne près.
4. Ensuite seulement, si les mesures le demandent : une colonne par site, puis une mention par
   référence. Ce sont des lots de schéma, à mesurer par la sonde et par l'oracle.
