# Un index lucivy écrit avant la 4.3.5 pendant des fusions de fond garde ses mots perdus

- **État** : ouvert (13 novembre 2026). Relevé par c7 à la publication de lucivy 4.3.5. Pour
  recherche, qui fait le chiffrage avec la montée, et pour hotfixs, qui joue la sonde sur les
  copies.
- **Famille** : produit
- **Gravité** : une recherche plein texte qui ne trouve plus un mot présent dans le code. L'exact
  manque un identifiant que seuls quelques documents portent, et le flou écarte ces documents.
  C'est silencieux : rien ne signale la perte.
- **Atteignable en service** : oui. Les bases de code de Lucie sont alimentées au fil de l'eau, et
  leurs `.fts` ont été écrits sous lucivy ≤ 4.3.3 pendant des fusions de fond.
- **Touche rag3weaver** : oui, par ses index plein texte (`.fts`) ; le correctif vit dans
  lucivy 4.3.5.

## Ce que c'est

Avant la 4.3.5, une fusion de fond lancée entre deux commits faisait perdre au dictionnaire
partagé (format 4) les mots des segments écrits depuis le dernier commit. On l'a vu à deux
endroits :

- le rouge intermittent de `test_playground_repro` ;
- un index sauvé, avec 31 862 identifiants inconnus.

Ce qui en découle (CHANGELOG de lucivy, entrées 4.3.4 et 4.3.5) :

- la montée à 4.3.5 arrête la perte pour ce qui s'écrit ensuite ;
- elle ne répare pas un index déjà touché, qui garde ses pertes ;
- le format d'index ne change pas.

## Ce qui reste à faire

1. **Chiffrer, sur des copies.** Il faut compter les identifiants inconnus de chaque `.fts` des
   bases de code de Lucie, sur les copies de leurs instantanés et jamais sur les bases elles-mêmes.
   On procède comme pour la sonde des orphelines : l'outil vient de lucivy, hotfixs le joue. Le
   chiffre se donne en clair, base par base.
2. **Reconstruire, au mot de Lucie seulement.** La reconstruction lexicale existe : c'est le rebâti
   du plein texte. Si des `.fts` ont perdu des mots, la reconstruire est un lot à part, présenté à
   Lucie avec le chiffre devant elle. Aucune base de Lucie n'est reconstruite sans son mot.

## Lot lié

La montée de lucivy-core, ld-lucivy et sparse-vector à 4.3.5 (branche `lucivy-4-3-5`, recherche).
