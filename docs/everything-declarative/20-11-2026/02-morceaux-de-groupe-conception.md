# Les morceaux de groupe — conception, à relire par Lucie avant le code

20 novembre 2026, session everything declarative. L'idée est de Lucie (10 h 29, par c7) : plutôt que de seulement
retirer du vecteur les petits scopes `variable`, réunir une suite de petits scopes frères juxtaposés en **un seul
morceau d'embarquement**. Rien n'est codé : cette page attend son mot.

## Ce qui précède

- **2.12** : un conteneur replié n'est pas candidat (ni plein texte, ni morceaux).
- **`variable-not-candidate`** (règles 12) : un `variable` de moins de 80 caractères n'a plus de morceau, donc plus de
  vecteur. Il garde son document de plein texte, donc il reste trouvable par son nom. C'est ce qui corrige le 88 % de
  luciepc : `s=index.read_text()`, 18 caractères, gagnait au vecteur.
- **Les morceaux à la fenêtre** (règles 13) : un morceau tient dans la fenêtre du plus petit embarqueur de la base, en
  octets puis en jetons.

**Ce que l'exclusion perd** : une suite de déclarations courtes (une table de constantes de configuration, les
réglages en tête d'un script, les champs d'une dataclass) n'est plus atteignable par le sens. Le morceau de groupe la
rend atteignable, sans le défaut d'avant : ni un morceau de 18 caractères qui gagne seul, ni les doublons des règles 8
(844 Mo, chaque enfant recopié dans son parent).

## Ce qui ne bouge pas

- **Le graphe, les arêtes, `usages`, `outline`** : chaque scope reste une ligne, avec ses arêtes.
- **Le plein texte** : chaque scope garde son document. Un nom se trouve par son nom.
- **Le contenu d'un parent** : il replie déjà ses enfants (`content` avec `folds`, depuis 57bfc142bf). Le texte d'une
  `variable` n'existe donc que dans son propre scope : le morceau de groupe ne doublonne rien.

Le groupe vit **uniquement au niveau de l'embarquement** : des lignes `…_Chunk` de plus, rien d'autre.

## Ce qui entre dans un groupe

- **Les types** : ceux de la règle d'exclusion des variables (`variable`), et c'est le seul type au premier lot. Une
  liste déclarée dans la règle permet d'y ajouter plus tard des types qui ont le même défaut, si une mesure le montre :
  `constant`, un champ court, un alias de type. Je ne les mets pas sans mesure.
- **Le seuil** : celui de l'exclusion (`workspace.variable_min_chars`, 80). Un scope qui serait exclu entre dans un
  groupe à la place. Un scope au-dessus du seuil garde son propre morceau, comme aujourd'hui, et **coupe** le groupe.
- **Un groupe trop court reste exclu.** Un groupe dont le texte total n'atteint pas le seuil (un `s=…` seul) n'a pas
  de morceau : c'est le cas que l'exclusion règle déjà, et il ne doit pas revenir par le groupe.

**La règle devient** : `TakesOut::Chunks` (ce que fait `variable-not-candidate`), plus une option `groups: true`. Les
scopes exclus de leur propre morceau sont réunis par groupes ; les autres règles (les conteneurs de 2.12) ne groupent
pas.

## Où un groupe coupe

Un groupe est une suite **maximale** de scopes groupables qui sont :
1. **frères** : le même parent (fichier, classe, fonction, module) ;
2. **juxtaposés dans l'ordre du fichier** : aucun scope frère non groupable entre eux. Une fonction au milieu coupe le
   groupe en deux. Les lignes vides et les commentaires entre eux ne coupent pas ;
3. **sous la fenêtre** du plus petit embarqueur (lot 13) : en octets, puis en jetons comptés par son tokenizer. Un
   groupe qui la dépasse se coupe **entre deux scopes**, jamais au milieu d'un scope. Un scope seul plus long que la
   fenêtre n'est pas groupable par définition, puisqu'il est au-dessus du seuil.

**Chaque scope entre dans un seul groupe**, toujours au même. L'appartenance est calculée sur les octets
(`start_byte`, `end_byte`), pas sur les lignes. Un bundle minifié d'une ligne qui porte mille déclarations donne donc
des groupes successifs **sans recouvrement**, coupés à la fenêtre. Aucun contexte autour d'un groupe : pas de lignes
de recouvrement comme `chunk_lines` en ajoute. C'est ce qui garantit l'absence de doublon.

## Ce que porte la ligne d'un morceau de groupe

Une ligne de `ScopeContent_Chunk`, comme un morceau ordinaire, avec :
- `_parent_uuid` : la version du **parent** (`ScopeContent` du fichier ou de la fonction) ;
- `_text` : les textes des membres, **dans l'ordre du fichier**, séparés par un saut de ligne. Pas de titre préfixé
  par membre : le titre est celui du parent ;
- `_start_line`, `_end_line` : la plage de lignes réunies, de la première ligne du premier membre à la dernière du
  dernier ;
- `_group` (neuf) : la liste des clés des membres (`key`, la clé stable de codeparsers, `blake3(file:name:type:signature)`),
  dans l'ordre ;
- `_index` : son rang parmi les morceaux du parent, après les morceaux propres du parent.

**L'identité de la version du parent** doit changer quand un membre change, sinon un groupe modifié pendrait à une
version immuable. Or le parent replie ses enfants, donc son rendu ne change pas quand une `variable` change. **Je
propose** : le rendu du parent porte un champ `groups_hash` (le hachage des textes de ses groupes). Une variable
éditée change ce hachage, donc la version du parent (`derived_content_uuid` hache le rendu), donc ses morceaux sont
réécrits sous une nouvelle génération, avec la machinerie qui existe. L'autre forme serait une entité de version
propre aux groupes, avec ses arêtes et sa visibilité par génération : plus juste en théorie, beaucoup plus lourde. Je
ne la prends pas sans ton mot.

## Comment la recherche le rend

Un résultat porté par un morceau de groupe se rend comme **le parent, à la plage du groupe** : le fichier (ou la
fonction), les lignes `_start_line`–`_end_line` lisibles par `read_file`, et la liste des noms des membres
(`_group` → `name`).

Quand la même requête trouve aussi un membre par le plein texte, la fusion garde **le membre**, plus précis, et le
groupe ne s'ajoute pas en doublon : la clé de dédoublonnage du résultat est la plage de lignes. C'est le point le plus
délicat de la page : la fusion est à lire dans `generic_search_nodes.rs` avant d'écrire.

## L'incrémental

- **Éditer une variable** change le texte de son groupe, donc `groups_hash` du parent, donc la version du parent : le
  parent est réembarqué avec ses groupes. Le coût est celui du parent et de ses groupes, rien de plus.
- **Ajouter une fonction entre deux variables** coupe un groupe en deux. Pareil, par la même voie.
- **Un fichier inchangé** ne bouge pas : même rendu, même version, mêmes morceaux.

## Le repassage

**Règles de l'analyse 14** : les morceaux changent. Chaque base repasse une fois. G fixe la fenêtre, et l'on cherche à
la faire coïncider avec celle des lots 12 et 13 pour un seul repassage, si le calendrier le permet.

## La mesure qui décide (sur la copie migrée de code-principal tenue par G)

- **Le recouvrement au critère de G** (les 10 requêtes, k = 10, seuil ≥ 90 %) : il ne baisse pas par rapport au
  lot 12 (exclusion seule).
- **Des requêtes qui visent de courtes déclarations** : une dizaine, écrites à partir des tables de constantes et des
  réglages en tête de script de l'arbre (par exemple « le chemin du fichier de résultats », « le délai d'attente du
  flux »). Le premier résultat juste remonte **mieux que sous l'exclusion seule**, et au moins aussi bien que sous les
  règles 11 (avant l'exclusion).
- **`s=index.read_text()` et ses quatre jumeaux** ne remontent pas par le vecteur : leur groupe est trop court, ou il
  ne gagne plus seul.
- **Les totaux de `usages`** : aucun usage perdu par section.
- **La taille** : le nombre de morceaux et d'octets de vecteurs, contre le lot 12 et contre les règles 8 (les 844 Mo).
  On attend bien moins que les règles 8, et un peu plus que le lot 12.

## La taille du lot

Environ une journée d'agent :
- la règle et le regroupement dans `compute_chunks` (par parent, sur les octets) ;
- `groups_hash` dans le rendu du parent ;
- la ligne de morceau et son champ `_group` ;
- le rendu et la fusion dans la recherche ;
- les témoins : un groupe par suite, coupé par un frère non groupable et par la fenêtre ; aucun recouvrement sur un
  bundle d'une ligne ; une variable éditée réembarque son seul parent ; la garde des comptes inchangée ;
- puis le juge sur la copie de G.

## Ce que je te demande, Lucie

1. **Le seul type `variable` au premier lot**, les autres types par mesure plus tard : d'accord ?
2. **`groups_hash` dans le rendu du parent** plutôt qu'une entité de version propre aux groupes : d'accord ?
3. **Le rendu d'un résultat de groupe** comme le parent à la plage du groupe, avec la liste des noms, et le membre
   préféré au groupe quand le plein texte le trouve : d'accord ?
