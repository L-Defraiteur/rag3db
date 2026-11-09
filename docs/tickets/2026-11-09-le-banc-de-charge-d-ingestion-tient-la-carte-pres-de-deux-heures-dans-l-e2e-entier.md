# Le banc de charge d'ingestion tient la carte près de deux heures dans l'e2e entier

- **État** : ouvert (9 novembre 2026, session recherche, à la demande de c7).
- **Famille** : produit
- **Gravité** : gêne pour tout le poste. Pendant qu'il tourne, la `/sante` de bge-m3 (7879) met
  17 à 19 s à répondre, et les rappels de Lucie et de memoire-longue embarquent lentement.
- **Atteignable en service** : non (une suite de tests) ; mais elle prend les services d'embarquement
  du poste à tout le monde.
- **Touche rag3weaver** : oui, `tests/e2e_charge_ingestion.rs` et la place de cette mesure dans
  la passe.

## Ce que c'est

`e2e_charge_ingestion::ingerer_notre_propre_source_avec_le_vrai_embedder` n'est pas un test de
correction mais un **banc de charge** : il ingère tout `extension/rag3weaver/src` (248 à 251
fichiers, environ 14 000 scopes) dans une base en mémoire, avec bge-m3 en double (dense et creux,
`set_dual_embedder`), et échantillonne toutes les 20 ms l'occupation des cartes
(`/sys/class/drm/card*/device/gpu_busy_percent`). Sa seule assertion : rien de perdu
(`report.failed == 0`). Il est dans l'e2e entier comme une suite ordinaire.

## La mesure (luciepc)

| passe | scopes | durée | carte |
|---|---|---|---|
| 8 nov., 19 h 54 – 21 h 40 (residus-de-suite, finie) | 14 012 | 6 380 325 ms (1 h 46) | card2 moyenne 94 %, 93,3 % des échantillons ≥ 90 % ; card0 moyenne 82 % |
| 9 nov., 11 h 10 – 12 h 25 (residus-de-suite-2) | 14 245 | arrêtée à 1 h 14 min 33 s (c7 : les rappels de Lucie ralentissaient) | — |

Régime de ces passes : `Confort`, budget par lot 8 192 caractères, rapport cyclique 100 %. Le régime
doux (4 096, 70 %) n'est posé par les lanceurs qu'hors de luciepc.

La veille, à 21 h, la même suite avait déjà pris la carte au reste du poste (c7).

## Ce qui reste

- Le dire avec ses chiffres : morceaux embarqués, débit (morceaux et caractères par seconde, dense et
  creux séparés), pourquoi le creux est en plein pendant toute l'ingestion.
- Trancher sa place : un banc à part, joué sous `poste mesure` quand la carte est à personne, avec
  ses comptes écrits, plutôt qu'une suite de l'e2e entier, où il coûte près de deux heures de carte à
  chaque passe complète. Pas un seuil relâché : la mesure garde ses chiffres, elle change de place.
- En attendant : une passe entière qui doit rendre la carte l'arrête au premier mot (le groupe de
  `run_e2e.sh`, voir docs/procedures/push-d-un-lot.md).
