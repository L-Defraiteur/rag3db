# Le chemin sans positions relit tout pour une requête longue

- **État** : ouvert (14 novembre 2026), pour un jour : les index de rag3weaver gardent leurs positions.
- **Gravité** : performance. Les réponses sont justes, mais une requête longue et floue prend une minute.
- **Atteignable en service** : non tant qu'aucun backend ne pose `fts_positions: false`. Oui dès qu'un backend le
  pose.
- **Touche rag3weaver** : non (lucivy seule), mais c'est ce qui interdit le profil léger aux index de rag3weaver.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Un index créé avec `positions: false` garde, dans ses postings, les documents et les fréquences, mais pas les
positions. Chaque correspondance est retrouvée comme documents candidats, puis vérifiée sur le texte stocké. La
doc de l'option l'annonce : mêmes réponses, et une relecture des candidats pour les spans et les correspondances à
travers plusieurs tokens.

Mesuré le 14 novembre 2026 à luciepc (5 cœurs) sur une réindexation de `Lucivy_Scope` (89 894 documents, 932 Mo de
texte), avec les requêtes que `search_code` envoie (contains de la requête entière sur trois champs, spans
toujours demandés) :

| requête | avec positions | sans positions |
|---|---|---|
| 1 mot, contains d1 | 146 à 168 ms | 967 à 975 ms |
| 4 mots, contains d1 | 35 ms | 4 398 à 5 478 ms |
| requête longue de 300 caractères, d1 (ce qu'envoie l'archiviste) | 176 à 180 ms | **60,1 à 62,9 s** |
| requête de plus de 900 caractères, d0 | 340 à 427 ms | 1 561 à 1 675 ms |

La requête longue en distance 1 ne trouve **aucun** document, et prend pourtant une minute. La vérification floue à
travers les tokens se fait sur le texte stocké de tous les candidats, alors que les candidats d'une requête longue
pourraient être réduits d'abord par l'intersection des postings de ses tokens rares.

## Comment le rejouer

`lucivy_core/tests/test_light_profile.rs`, branche lucivy `mesure-profil-leger` (f89e6df) : réindexation d'une
copie dans chaque profil, puis le banc. Commande et protocole dans
`docs/lucivy/14-11-2026/02-les-fils-de-lucivy-et-le-profil-leger.md`, partie 2.

## Ce qu'on pourrait faire

- Réduire les candidats d'une requête de plusieurs tokens par les postings de ses tokens les plus rares avant de
  relire le texte, comme le fait le chemin avec positions.
- Couper la vérification d'un document dès que l'un des tokens exigés y manque.
