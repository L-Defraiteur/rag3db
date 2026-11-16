# Relecture indépendante du replay des469 morceaux

Relecture des preuves publiées par Arbre1760fa03a5, exécution ae5d3ac4ff.
Ticket porteur toujours ouvert :
`docs/arbre-principal/tickets/2026-11-16-la-migration-de-scopes-laisse-469-morceaux-sans-vecteur-transfere.md`.
Aucun DB, HTTP, compilateur ni replay exécuté par cette relecture.

## Résultat recalculé

Le manifeste des121 fichiers, SHA256
`6d4667936fc876e4a415d5cd12698486f0d231b7f8ebd1cca57250b475868cc6`,
est vérifié sur la copie portable `vector-debt-replay-pc-local-01` : toutes les
tailles et empreintes concordent. Les sorties BEFORE/AFTER stockées et chaque
TSV de replay sont redécodés indépendamment, avec contrôle des bornes et comptes.
Les multiplicités sont préservées par Counter :

| Contrôle | Résultat |
| --- | ---: |
| Parents aux deux côtés | 51 |
| Morceaux anciens | 1008 |
| Morceaux nouveaux | 1157 |
| Tuples communs | 688 |
| Anciens seuls | 320 |
| Nouveaux seuls | 469 |
| Parents dont les deux replays égalent les tuples stockés | 51/51 |

Le tuple comparé comprend parent, index, bornes complètes, bornes cœur et hash.
Les469UUID uniques de coverage.json et les469signatures sont confrontés à la
preuve géométrique antérieure, manifeste dd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568 :
ils correspondent exactement à la cohorte brute `cohort-new-geometry` et aux
nouveaux seuls. Aucune signature inconnue. L'égalité des contenus parents et des
paramètres reste celle des preuves d'Arbre ; aucun corps de source relu ici.

## La clé de transfert n'est pas le seul hash

Contrôle complémentaire sur les mêmes lignes brutes :74 des nouveaux seuls ont
un ancien morceau avec le même parent et hash, mais un index différent. **Aucun**
n'a de candidat ancien avec le même triplet parent/index/hash. Le code927
`migration_des_versions.rs::transferer_les_vecteurs` utilise une clé plus stricte :
représentant, champ parent, index, hash et titre, sur un ancien vecteur dont le
marqueur correspond au hash. L'absence du triplet implique ici l'absence de la
clé complète ; les74hashes communs ne réfutent donc pas l'attribution et ne
prouvent pas un vecteur transférable oublié. Aucun correctif de transfert proposé.

Les modules réels sont rehachés depuis git, sans compilation :

- 49644f064f9f9ece4c9c4bd8714eb86838afa4a0 :
  `b2b4a0a916b81a25bab50f18b28bd9355e6094f7ee42a456f8dcabf33bb6e9dd` ;
- 479a46549e9d3eb04459bbfd05445e3750fbdfd6 et927941a67a :
  `1b35738216ab11ec7d0ba88f47b8724ef88eb48cfc858292790b79e39d114622`.

Ils concordent avec le ledger ; le module après479a est identique dans927.
La preuve étaye l'attribution au redécoupage479a **pour cette cohorte**, sans
étendre sa portée à d'autres bases, parents ou campagnes.

## Préservation et limites

Les inventaires enregistrés source-before/source-after et dependencies-before/
dependencies-after sont égaux ; le binaire de replay concorde avec son SHA de
ledger. Ce contrôle porte sur les pièces et inventaires, jamais sur une ouverture
ou un nouveau hachage physique de la DB. Les preuves d'exécution d'Arbre restent
la référence pour la portée commune validation-qwbuqe__, verdict et quiescence.

**Dette non payée, migration non qualifiée, aucun état vivant déduit.** Les
nouveaux textes demandent leur propre qualification vectorielle ; la lecture de
ces preuves n'autorise ni copie forcée de vecteurs anciens ni nouvelle écriture.
Le ticket469 reste ouvert. La suite relève de l'arbitrage séparé de C7.
L'oracle global source reste NONMATCHED avec ses limites antérieures.

Script et consolidé indépendants locaux :
`~/.cache/rag3db-banc-notes/debt-replay-review-16nov/{review.py,review.json}`.
SHA256 du consolidé :
`31585107444934bd0844eda4fc2327faf3316afb5b6e15c5403f09135c0d2454`.
Les121artefacts et les premières preuves/rouges restent conservés.
