# Le juge des bilans n'a pas de plafond d'appels

- **État** : corrigé (7 novembre 2026) par le lot « bilans.max_per_hour » (everything declarative) : au plus N runs jugés par heure glissante et par backend, 20 par défaut, le saut dit au journal, sur la sortie d'erreur et dans la boîte d'un équipier.
- **Famille** : produit
- **Gravité** : dépense non bornée (chaque tour fini d'une conversation qui déclare `bilans` coûte au moins un appel au modèle du juge, sans limite).
- **Atteignable en service** : oui, dès qu'un manifeste déclare `bilans.memoire` (Lucie l'allume sur son chat le 7 novembre au soir).
- **Touche rag3weaver** : oui (`src/daemon_host/juge.rs`, `src/backend.rs`).

## Ce que c'est

`demander_un_bilan` (`juge.rs:421`) envoie chaque run fini au fil du juge dès que le manifeste
déclare `bilans` ; le fil (`servir_les_bilans`, `juge.rs:431`) juge chacun (le verdict, puis le
bilan des outils : au moins un appel au modèle par run), sans compte ni plafond. La clé
`BilansDeclares` (`backend.rs:75`) n'a que `memoire`. Le hook de Claude Code, lui, borne son juge
(`recall_judge_max_per_hour`, 20).

## Ce qu'il faut pour le fermer

Une politique au manifeste, à côté de `memoire` : `bilans.max_per_hour` (défaut déclaré, 20 comme le
hook), comptée par backend sur une heure glissante ; au-delà, le run n'est pas jugé, et c'est dit
au journal du run (`bilan: "sauté : plafond de N par heure atteint"`) et, pour une tâche
d'équipier, dans sa boîte. Témoin : un manifeste à `max_per_hour: 2`, trois runs finis dans
l'heure, deux bilans et un saut dit.
