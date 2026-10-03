# Le poids de la marque de test — mesuré au banc, calibré à e2e_code

**Session recherche, 3 octobre 2026, 22h40.** Demande de l'orchestration :
la pile codeparsers (branche `codeparsers-pile`, 5d376ff78) fait des
fonctions de `mod tests` des scopes — 415 de plus sur `src/dataflow`, dont
~350 fonctions de test — et `e2e_code.rs:331` casse : sur la requête
`merge_port_values`, deux fonctions de test qui l'appellent passent devant
sa définition. Choisir le poids de `test_role` (case, suite, support) au
banc, sur une référence refaite avec la pile ; l'assertion doit redevenir
verte sans être touchée.

Banc : `e2e_banc_etage`, granite-278m par le service distant
(127.0.0.1:7979-7981), régime doux, base = `codeparsers-pile` rebasée
localement sur master (submodule codeparsers a569417).

## La valeur retenue : 0,5 uniforme

```json
{ "field": "test_role",
  "weights": { "case": 0.5, "suite": 0.5, "support": 0.5 },
  "default": 1.0 }
```

La valeur vide — un scope qui n'est pas un test — reste pleine. Le produit
avec le `scope_type` 0,85 existant se compose seul dans `FieldWeightNode`.

**Et la déclaration a une seconde pièce, indispensable** : `"test_role"`
dans `return_fields` de `Scope`. `FieldWeightNode` ne pèse que ce que les
données des résultats portent ; un champ absent est neutre, et il l'avoue
en méta (« champ absent des données — pondération neutre »). Le premier
essai, poids seul, laissait e2e_code rouge avec des scores strictement
inchangés. `scope_type` ne marche que parce qu'il est déjà enrichi. Leçon
pour l'échelle du pas C : **tout champ pondéré doit être dans
`return_fields`, sinon le poids est un vœu.**

## Le plafond vient d'e2e_code, le plateau vient du banc

Scores mesurés sur `search("merge_port_values")` avec la pile, sans poids :

| rang | scope | score | signaux |
|---|---|---|---|
| 1 | `tests::la_fusion_ne_repete_pas_le_meme_avertissement` | 0,0159 | vector+bm25 |
| 2 | `tests::deux_metas_se_fusionnent_sur_un_port` | 0,0097 | bm25 |
| 3 | `merge_port_values` (la définition) | 0,0095 | bm25 |

Pour que la définition passe première : w < 0,0095/0,0159 = **0,597**.
0,5 donne 19 % de marge, 0,55 n'en donne que 8.

Au banc, la ligne T (étage appelant, même table) est un **plateau** :
0,4, 0,5, 0,55, 0,6 et 0,7 rendent exactement 0,405 / 13 / 26 sur les
45 questions. Le banc ne départage pas les valeurs ; la marge d'e2e_code
décide : **0,5**. (Même conclusion que les couples de fusion du même
jour : plus de marge, pas plus de poids.)

## Ce que la pile coûte, ce que le poids répare

| mesure (tel quel, granite-278m) | MRR | R@1 | R@5 |
|---|---|---|---|
| avant la pile (référence du 3 octobre, ~4 800 scopes) | 0,406–0,407 | 13 | 27 |
| avec la pile, sans poids (7 372 scopes) | 0,361 | 11 | 24 |
| avec la pile, déclaration posée (0,5 + enrichissement) | 0,405 | 13 | 26 |

Le bruit des ~2 500 scopes de test coûte ~0,045 de MRR au tel quel ; la
déclaration récupère quasi tout (le R@5 26 contre 27 est de l'ordre de la
variance). Les bonnes réponses restaient donc dans les candidats du HNSW —
elles étaient déclassées, pas évincées : un poids suffit, pas besoin de
toucher au rappel.

Avec la déclaration, e2e_code `read_and_grep` est **vert, assertion
intouchée** : la définition première (0,0095), le premier test rival à
0,0079 (0,0159 × 0,5), le second hors du top 5. Le rendu montre au passage
`· test_role=case` sur les résultats de test — conséquence de
`return_fields`, utile à l'agent.

## Les limites, avouées

- **Aucune question du banc n'a un test pour bonne réponse.** La mesure
  dit ce que la dévaluation rend aux définitions, pas ce qu'elle coûte à
  « où est le test de X ». C'est pour ces questions-là que la valeur reste
  douce et que `default` vaut 1,0 — un test reste trouvable, il cède le
  pas quand une définition rivalise.
- La règle « une correspondance exacte du nom passe en tête » n'est **pas**
  implémentée : c'est une règle de classement neuve, posée à Lucie par
  l'orchestration. Avec le poids, l'exact repasse premier sur ce cas sans
  elle.
- La première passe du banc avait une ligne T parfaitement plate — elle
  mesurait la pondération neutre (champ non enrichi), pas le poids. Un
  « zéro effet » se vérifie en branchant le levier avant de conclure ;
  c'est l'aveu en méta du nœud qui a mis sur la piste.

La déclaration est à la session de l'arbre principal (tenante de
`code.rs`), qui la fusionne avec la pile ; la ligne T du banc sera poussée
ensuite (elle n'a de sens qu'avec le champ).
