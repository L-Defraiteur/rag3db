# Un index lucivy écrit avant la 4.3.5 pendant des fusions de fond garde ses mots perdus

- **État** : corrigé (13 novembre 2026, lot lucivy-4-3-5, recherche). La montée à lucivy 4.3.5
  arrête la perte, ce que prouve le témoin `fts_handle::mots_neufs_a_travers_une_fusion`, rouge sous
  4.3.3 et vert sous 4.3.5. Le chiffrage des bases de Lucie donne 0 identifiant inconnu : rien à
  reconstruire.
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

## Le chiffre (13 novembre 2026)

Chiffré le 13 novembre sur des copies neuves de l'instantané avant-835d5298f, hôtes arrêtés,
0 segment en attente partout (hotfixs, page `docs/coeur-cpp-hotfixs-tickets/13-11-2026/01-sonde-des-bases-de-lucie.md`,
section « Seconde sonde », 5e50e2800). Les `.fts` ont **0 identifiant inconnu** :

| base | identifiants inconnus | identifiants du dictionnaire |
|---|---|---|
| code-lourd | 0 | 6 945 348 |
| code-principal | 0 | 5 732 393 |
| code-rag3db-depot | 0 | 11 818 307 |
| memoire-longue | 0 | 261 925 |

La sonde a d'abord été vue rouge sur l'index sauvé de lucivy, dans une copie neuve : 31 862 sur
2 693 974, le chiffre de lucivy. L'outil est `diag_ids_sans_texte_tous_champs`, sur la branche lucivy
`diag-mots-perdus-tous-champs` (a4e2d16). Il parcourt chaque `Lucivy_*`, chaque shard et chaque
champ texte qui a un `.gmap`, et lit `pending_segments`.

**Rien à reconstruire** : aucune base de Lucie n'a perdu de mots.

## Correctif

- Un témoin dans rag3weaver, `fts_handle::mots_neufs_a_travers_une_fusion`, joue le scénario
  déterministe de lucivy sur le schéma v4 que construit rag3weaver. Sous 4.3.3, après réouverture,
  l'exact ne retrouve aucun des 20 identifiants neufs ; sous 4.3.5, il les retrouve tous, et le flou
  aussi. Un contrôle sans fusion passe sous les deux versions.
- lucivy-core, ld-lucivy et sparse-vector montent à 4.3.5 (format inchangé).

## Lot lié

La montée de lucivy-core, ld-lucivy et sparse-vector à 4.3.5 (branche `lucivy-4-3-5`, recherche).
