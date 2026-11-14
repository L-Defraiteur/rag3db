# Le dictionnaire garde les textes des documents effacés

- **État** : ouvert (14 novembre 2026). Corrigé dans lucivy main (PR #37 et #38, d3b8c4a6a, 15 novembre) : sur
  `Lucivy_Scope`, le dictionnaire passe de 513,8 à 400,7 Mo après la purge et la compaction. Se ferme quand
  rag3weaver est en 4.3.8 et que `maintain` a tourné sur les vraies bases.
- **Gravité** : poids. Les réponses ne changent pas, mais une purge suivie d'une compaction laisse dans le
  dictionnaire du shard les textes de tous les documents effacés : 93,7 Mo sur `Lucivy_Scope` de
  code-rag3db-depot, 7 % de l'index. C'est une part du « cent fois ses sources »
  (`2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources.md`).
- **Atteignable en service** : oui. Toute table qui efface ou remplace des documents (réindexation d'un fichier,
  purge des orphelins) accumule des textes morts dans le dictionnaire.
- **Touche rag3weaver** : non (lucivy seule), mais c'est rag3weaver qui en paie les octets.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Avec le dictionnaire partagé (sfx v4, le défaut), chaque shard tient un dictionnaire des textes de tokens commun à
ses segments. Un commit y replie les textes neufs. Une fusion de segments réécrit les postings et purge les
documents supprimés, mais **rien ne retire du dictionnaire un texte que plus aucun document vivant ne porte**.

Sur la fixture de `test_maintain.rs` (400 documents, la moitié purgés puis compactés), le dictionnaire reste à
69 704 octets avant comme après. Sur une copie neuve de l'instantané `avant-835d5298f` de code-rag3db-depot, mesuré
à luciepc le 14 novembre 2026 avec `maintain` (lucivy 6f8c61b) :

| `Lucivy_Scope` | documents vivants | dictionnaire | positions | postings | texte stocké | total |
|---|---|---|---|---|---|---|
| avant | 88 471 (11 879 supprimés en segments) | 513,8 Mo | 1,58 Go | 1,74 Go | 440,5 Mo | 4,27 Go |
| purge de 8 809 orphelins, puis compaction | 79 662 (0 supprimé) | **513,8 Mo** | 334,0 Mo | 370,0 Mo | 83,7 Mo | 1,30 Go |
| les mêmes 79 662 réindexés à neuf | 79 662 | **420,1 Mo** | 324,3 Mo | 373,4 Mo | 87,0 Mo | 1,20 Go |

Le dictionnaire de la table compactée porte **93,7 Mo de textes morts** (18 % de son dictionnaire, 7 % de
l'index). Même neuf, le dictionnaire reste la plus grosse famille : 420 Mo pour 171,5 Mo de texte stocké.

## Comment le rejouer

Sur une copie, avec les outils de main :
1. `test_maintain.rs`, `maintain_an_index` (`LUCIVY_MAINTAIN_INDEX`, `LUCIVY_MAINTAIN_IDS`,
   `LUCIVY_MAINTAIN_COMPACT=1`) : la famille `dictionary` d'avant et d'après.
2. `test_light_profile.rs`, `light_profile_of_an_index` sur la copie compactée (`LUCIVY_PROFILE_VARIANTS=positions`) :
   le dictionnaire d'un index neuf des mêmes documents.

## Ce qu'on pourrait faire

- Une compaction du dictionnaire : réécrire celui du shard en ne gardant que les textes que référencent encore les
  postings des segments vivants. Elle peut s'appeler à la fin d'une compaction complète (`compact`, `maintain`), là
  où tous les segments viennent d'être réécrits.
- Ou un compte de références par texte, tenu au fil des fusions, qui permettrait de retirer au fil de l'eau.
