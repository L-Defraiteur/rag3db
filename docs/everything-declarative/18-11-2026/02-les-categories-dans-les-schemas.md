# Les catégories nommées dans les schémas déclaratifs

18 novembre 2026, session ED. Graine de Lucie, relayée par c7, à penser avec mémoire (96). Mémoire tient la
conception du côté du gabarit `memory` ; cette page dit seulement où la chose se pose dans les schémas déclaratifs.
**Rien n'est à coder pour l'instant.**

## 1. La demande, en une ligne par point

- **Une catégorie.** Les agents la construisent eux-mêmes. Elle a un nom et une description, et elle vit dans une
  fiche de définitions.
- **Ses valeurs.** Chaque valeur a un identifiant stable, indépendant du nom affiché, et sa propre description.
- **Les références.** Une fiche peut citer une valeur, ou la catégorie entière.
- **Le lien indirect.** Deux fiches qui partagent une valeur sont reliées sans lien direct : fiche A → valeur ←
  fiche B.
- **Un juge.** Il refuse une valeur inconnue et un doublon. Devant une valeur proche par le sens, il demande de
  réutiliser l'existante ou de justifier la nouvelle.
- **Un changement de définition** signale les fiches qui en dépendent, sans les modifier.
- **« Une valeur valide n'est pas un fait vrai. »**
- **Une capacité générale** : états de workflow, types d'incidents, rôles. Pas un moteur de fiction.

## 2. Ce que le gabarit `memory` a déjà, et qui sert tel quel

| Besoin | Ce qui existe | Où |
|---|---|---|
| une catégorie nommée, avec description | un **genre** de référence : `RefType` (`name`, `description`, `forme`, `script` facultatif, `etat`) | `schemas/ref_type.json` |
| créer une catégorie sans en doubler une autre | `create_ref_type` : le premier appel n'écrit rien, il montre les genres existants ; il refuse un nom pris, une description vide, et un harnais qui reconnaît ce qu'un genre existant reconnaît déjà | `graphs/create_ref_type.mmd` |
| une valeur, point de rendez-vous | une **référence** `Ref` (`genre`, `valeur`, `dit_comme`, `empreinte`, `etat`) : « que sait-on de X ? » est un filtre sur genre et valeur | `schemas/ref.json` |
| identifiant stable et nom affiché séparés | `valeur` est la forme normale, et c'est elle le rendez-vous ; `dit_comme`, ce sont les mots qui l'ont citée, ceux qui s'embarquent et se cherchent. Renommer, c'est changer `dit_comme`, jamais `valeur` | `schemas/ref.json` |
| fiche → valeur | la relation **`ANCRE`** Fiche → Ref ; `add_ref` / `anchor` la posent | `backend.json`, `relations` |
| fiche A → valeur ← fiche B | `cited_by` : les fiches ancrées à une valeur, avec ce qui est dessous | `graphs/cited_by.mmd` |
| un changement signale sans modifier | `review_anchored` : quand une ligne change, les lignes qui l'ancrent passent par une transition déclarée (« à revoir »). Leur contenu ne change pas, seul leur état bouge | `graphs/review_anchored.mmd` |
| une valeur valide n'est pas un fait vrai | `Ref.etat` dit seulement que la chose citée existe (`vue`, `a_revoir`, `introuvable`). Les faits vivent dans `Etat.faits`, chacun avec sa nature (`dit` ou `deduit`) et les apports d'où il vient | `schemas/ref.json`, `schemas/etat.json` |

**Ce que donnent ces pièces une fois posées ensemble :**

- **Une catégorie est un genre.** Exemple : `incident_type`, avec sa description.
- **Une valeur est une `Ref` de ce genre.** Exemple : la valeur `incident_type:v-7k2q`, avec `dit_comme` égal à
  « panne de disque ».
- **Classer une fiche, c'est l'ancrer à cette `Ref`.** Deux fiches classées pareil se retrouvent par `cited_by`,
  sans lien entre elles.
- **Citer la catégorie entière**, c'est ancrer à une `Ref` du genre `category`, dont la valeur est le nom du genre
  (`category:incident_type`). Ce genre `category` se crée une fois, par `create_ref_type`.

## 3. Ce qui manque

1. **La description d'une valeur.** Une `Ref` n'a pas de champ pour cela : `dit_comme` est un nom, pas une
   définition. Deux voies, sans nouveau type d'entité.
   - **(a) Un champ facultatif `definition` sur `Ref`**, dans `schemas/ref.json`. C'est le plus petit changement :
     la valeur porte sa description. La modifier fait monter la `revision` de la ligne, et `review_anchored` signale
     les fiches ancrées.
   - **(b) La description dans la fiche de définitions de la catégorie.** Cette fiche (« catégorie :
     incident_type ») est ancrée à chacune de ses valeurs, et chaque valeur y a une section, donc un apport. La prose
     reste dans les fiches, comme tout le reste de la mémoire. En revanche, « signaler les dépendants » demande de
     remonter de la section changée à la valeur, puis aux fiches ancrées : deux sauts, pas un.

   **Je propose (a).** C'est un champ, pas un type ; la valeur reste lisible seule ; le signal tient en un saut. La
   fiche de définitions de (b) reste utile pour la prose de la catégorie, et elle ne porte pas la définition de
   chaque valeur. **Question pour mémoire et Lucie (n° 1)** : (a) ou (b).

2. **Le juge.** Le harnais `script` d'un genre ne lit rien : il ne fait aucune entrée-sortie. Il ne peut donc pas
   dire « valeur inconnue de cette catégorie ». Il faut **un geste à part**, comme `create_ref_type` l'est pour les
   genres : un outil de classement, qui dans ce sens n'écrit rien sans `confirm`. Il refuse :
   - une valeur absente de la catégorie, en donnant la liste ;
   - un doublon, c'est-à-dire la même `valeur`, ou le même `dit_comme` normalisé ;
   - une valeur proche par le sens d'une valeur existante (l'embarquement de `dit_comme` est déjà là). Il propose
     alors de réutiliser l'existante, ou de justifier la neuve dans un champ obligatoire.

   C'est un graphe et un nœud. Le modèle est celui de `CreateRefTypeNode`, qui refuse déjà un genre qui recouvre un
   autre. Ce n'est pas un type de plus. Ce juge fait partie de la conception de mémoire.

3. **Rien d'autre dans les schémas.** États de workflow, types d'incidents et rôles sont trois genres, avec leurs
   valeurs. Aucun n'a besoin d'une entité à lui. La généricité tient parce qu'une catégorie ne connaît que ses
   valeurs : rien d'une organisation particulière n'entre dans `EntityConfig`
   ([[genericite-des-schemas]]).

## 4. Ce que cette page ne décide pas

- **Le nom des outils**, en anglais selon la règle des identifiants, et la forme exacte du refus du juge : à
  mémoire.
- **La forme de l'identifiant stable**, un court aléatoire (`v-7k2q`) ou un slug figé à la création : à mémoire.
  Une seule contrainte : `valeur` ne change jamais après la création.
- **Le retrait d'une valeur** (`Ref.etat = retiree`) et ce que deviennent les fiches qui l'ancrent. Elles restent
  ancrées à une valeur retirée, et `review_anchored` les signale. Rien n'est réécrit.

## 5. Ma part, si (a) est retenu

Le champ `definition` dans `schemas/ref.json`, et sa lecture par `add_ref` et `cited_by` : un lot court, avec un
témoin rouge d'abord. Ce témoin vérifierait qu'une définition changée fait passer « à revoir » une fiche ancrée,
sans toucher à son contenu. Le juge et les gestes reviennent à mémoire.
