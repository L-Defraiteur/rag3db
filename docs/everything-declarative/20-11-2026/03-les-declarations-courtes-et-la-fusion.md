# Les déclarations courtes, hors du vecteur et toujours trouvées par leur nom

20 novembre 2026, session everything declarative. Le lot `variable-not-candidate` (règles de l'analyse 13), et ce que
le juge du banc a fait changer en route.

## Le défaut de départ

Le 88 % de luciepc. Depuis que le contenu d'un scope s'arrête aux octets de son nœud (57bfc142bf, 14 novembre), une
variable Python d'une instruction a son propre morceau : `s=index.read_text()`, 18 caractères. Son vecteur gagnait
des recherches où elle n'avait rien à faire.

## Ce que fait le lot, en trois pièces

1. **L'exclusion, par taille, des déclarations de valeur.** Une `variable`, une `constant` ou un `type_alias` (un champ
   de classe est une `variable`), qui ne replie aucun enfant et porte moins de 80 caractères
   (`workspace.variable_min_chars`), n'a plus de morceau, donc plus de vecteur, quel que soit son type parmi elles. Elle
   garde son document de plein texte, sa ligne, ses arêtes, sa place dans `outline` et `usages`.
   - **Par taille et pas par type** (Lucie, 10 h 36) : une constante ne doit pas se chercher autrement qu'une variable.
   - **Les déclarations de valeur, pas toutes les feuilles** (Lucie, 11 h 46, réponse (ii)) : la règle sur toutes les
     feuilles retirait leur vecteur aux fonctions d'une ligne, et 28 tests rougissaient, tous sur des corpus de petites
     fonctions. Le nom et le corps d'une fonction ont un sens ; son vecteur n'est pas du bruit.
2. **La fusion juge une ligne sans vecteur sur les signaux qu'elle peut avoir** (c7, décision (A)). Une ligne dont le
   parent n'a aucun morceau est marquée `sans_vecteur`. La fusion RRF ramène son score au poids total, comme la moyenne
   pondérée des signaux qu'elle peut avoir : le dense et le creux lisent les morceaux et ne peuvent pas la nommer. Une
   ligne simplement absente du dense (un vecteur dû, ou pas assez proche) garde la fusion d'avant.
3. **Un nom tapé exactement rend sa définition en tête** (c7, décision (B)). Un résultat dont le nom est la requête, à la
   casse près, passe devant, sauf un conteneur qui replie des enfants (`exact_name_leaf='folds'`) : un namespace replié
   ou un `impl` n'est jamais promu par son nom. Tout est déclaré dans `search_workspace.mmd`, rien n'est écrit pour le
   code dans le nœud de fusion.

## Pourquoi (A) puis (B)

Le juge du banc sur l'exclusion seule : le critère de G tenu, mais **12 des 14 noms courts hors du top 10**.
- **Le plein texte était intact** : en BM25 seul, `CONFIGS` restait 3ᵉ et `ROOT` 8ᵉ, des deux côtés.
- **La fusion les enterrait** : `CONFIGS` passait du rang 1 au rang 26, `ROOT` du rang 2 au rang 40. Une ligne absente du
  dense n'y gagnait rien, et tout ce qui avait les deux signaux passait devant. Même `ROOT`, que le dense ne mettait
  qu'au rang 40 sur master, tenait le rang 2 par la seule présence de ce second signal.

Avec (A), les 14 noms restaient dans le top 10, mais 7 sur 10 seulement étaient premiers. `research` passait du rang 1 au
rang 7 : sur master, il était premier par **son propre vecteur**, un texte presque égal à son nom, donc très proche de la
requête. Aucune repondération ne rend cela ; seul le nom exact le fait. D'où (B).

Le premier (B) promouvait aussi `namespace rag3db {` (un conteneur replié) et `impl PortDef {` : d'où la règle des plis.
Quatre requêtes du critère de G changent de premier vers la définition qui porte exactement le nom (`commit` →
`fn commit`, `mutex` → `mutex()`, `chunk_uuid` → `fn chunk_uuid`, `acquireLock` → `Transaction::acquireLock`) : c7 l'a
accepté, c'est l'intention de (B).

## La fonction `lines`

Le juge de 16 h 30 donnait « `lines` 6ᵉ sur master » pour « lire chaque ligne d'un fichier jsonl comme un objet json »
(`prepare_engine_relations.py:12`, une fonction d'une ligne), et hors du top 10 avec le lot. **C'était le rang du
fichier, pas de la fonction** : le client du banc comptait le premier résultat dans le bon fichier, et ce 6ᵉ était
`cards`, la ligne 14 du même fichier, une variable de 220 caractères. **La fonction `lines` n'est dans le top 10 d'aucun
côté**, ni sur master ni avec le lot, rejouée avec chaque côté sur ses propres gabarits (feuille-5d).

Ce qui change, c'est `cards` : elle sort du top 10 avec le lot, et `events` (une variable de 92 caractères) y entre au
2ᵉ rang. Les deux sont au-dessus du seuil de 80 et gardent leur vecteur : l'exclusion ne les touche pas. C'est la fusion
(A), qui remonte les lignes sans vecteur, qui déplace des lignes à deux signaux de quelques rangs. Le top 10 en BM25 seul
est identique des deux côtés, et aucune déclaration courte n'entre dans le top 10.

## Les témoins

- la constante courte `PLANTE_MAX_KG` gardait son morceau (rouge sur la règle par `variable`) ;
- la fonction d'une ligne `aire` perdait le sien (rouge sur la règle de toutes les feuilles) ;
- une variable courte introuvable par son nom (rouge sous la forme stricte, qui retirait aussi le plein texte) ;
- la garde des comptes qui soustrairait toutes les règles (rouge : 7 documents pour 3 lignes) ;
- la fusion : la ligne sans vecteur, première en BM25, tombait 4ᵉ derrière trois concurrents à deux signaux ;
- le nom exact : la déclaration nommée par la requête arrivait dernière ; puis le namespace replié passait 2ᵉ.

**Une limite** : je n'ai pas reproduit sur un petit index le classement du banc (la dominance de l'exact y tient la
déclaration en tête). Les témoins de la fusion sont unitaires, au nœud ; la preuve qui décide est le juge du banc sur la
copie migrée de code-principal.
