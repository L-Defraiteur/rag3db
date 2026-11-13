# Le creux d'une version, avant son embarquement, doit dire « pas prêt » (un témoin manque)

- **État** : ouvert (13 novembre 2026), arbre principal, sur un point de la relecture de recherche.
- **Famille** : produit
- **Gravité** : une couverture manque : rien ne prouve qu'une recherche creuse sur une version pas
  encore embarquée se dise « pas prêt » au lieu de répondre vide.
- **Atteignable en service** : non aujourd'hui. Le schéma de code ne déclare pas le creux sur
  `ScopeContent`, et aucun manifeste réel ne le demande.
- **Touche rag3weaver** : oui (`tests/e2e_entites_derivees.rs`, le mécanisme des entités de contenu).

## Ce que c'est

Le mécanisme générique prend le creux sur une entité de contenu (`derived.identity = content`).
`e2e_entites_derivees` l'éprouve jusqu'au masque avant le top-k
(`le_masque_creux_precede_le_topk_des_versions`), mais toujours **après** l'embarquement creux.

Un refus générique du creux sur ces entités, codé au lot 1a sur le conseil de recherche, a été
retiré le 13 novembre au soir, parce qu'il cassait ces témoins
(`docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`, § 3). Recherche garde un
seul point : jamais une réponse vide en silence sur un index creux qui n'est pas encore rempli.

## Ce qu'il faut pour le fermer

Un témoin dans `e2e_entites_derivees`, rouge d'abord s'il le faut :
1. une entité de contenu déclarée `HYBRID | SPARSE` ;
2. des racines ingérées, sans embarquement creux (embarqueur creux absent ou dette laissée) ;
3. une recherche `SPARSE` avec `strict_signals`.

Elle doit dire que le signal n'est pas prêt (`expliquer_le_silence_d_un_signal`), pas rendre une
liste vide.
