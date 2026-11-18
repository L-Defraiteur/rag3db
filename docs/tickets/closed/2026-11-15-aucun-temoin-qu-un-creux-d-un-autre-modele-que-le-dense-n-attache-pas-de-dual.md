# Aucun témoin qu'un creux d'un autre modèle que le dense n'attache pas de dual

- **État** : fait (18 novembre 2026, embarquement, branche `dual-witness`, b3627412ae) ; ouvert (15 novembre 2026, embarquement ; relevé avec c7 après le lot du dual)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : témoin manquant. Le comportement est juste dans le code d'aujourd'hui, mais rien ne
  l'éprouve : une régression passerait sans rouge.
- **Atteignable en service** : oui, pour un manifeste dont `models.sparse` déclare `dimensions` et cite
  un autre modèle que le dense (un dense granite, un creux bge-m3 à la même dimension).
- **Touche rag3weaver** : oui (`src/backend.rs`, `open_with_reason`)

## Ce que c'est

Depuis le lot du dual (92b7a09fa), un creux activé qui déclare `dimensions` rend un dual. Le backend
ne l'attache au catalogue que si le modèle du creux est celui du dense : `set_dual_embedder` seulement
si `source.model == self.embed.model`, dans `open_with_reason`. Sinon, un dense granite recevrait des
vecteurs bge-m3 par le dual.

Ce qui est éprouvé aujourd'hui :
- **la dimension** : un creux qui déclare une autre dimension que le dense se refuse au chargement
  (`backend::tests::un_creux_d_une_autre_dimension_que_le_dense_se_refuse_au_chargement`) ;
- **l'identité du fournisseur** : un fournisseur sans dual, ou d'une autre dimension, se refuse à
  l'activation (`dual_un_fournisseur_sans_dual_ou_d_une_autre_dimension_se_refuse`).

Ce qui ne l'est pas : un creux d'**un autre modèle** que le dense, à la **même** dimension, qui charge
sans refus et ne doit **pas** attacher de dual.

## Ce qu'il faut pour le fermer

Un témoin qui ouvre un backend avec un dense granite et un `models.sparse` bge-m3 activé, à la même
dimension et sur la fixture. Il vérifie que le catalogue n'a pas de dual (`has_dual` faux) et que la
requête dense passe par le dense, pas par `/embed_dual`. Puis le même backend avec un creux du même
modèle que le dense : le dual est attaché. Le témoin doit être rouge si la condition du backend tombe.

## Fait (18 novembre 2026)

Témoin `a_sparse_of_another_model_attaches_no_dual_and_the_same_model_does` (`src/backend.rs`, tests) : un backend de
code, dense « fixture-dense » à 384, creux activé à la demande sur la fixture (`--sparse --dual`) à la même dimension.
Un creux « fixture-other » n'attache pas de dual ; un creux « fixture-dense » l'attache ; l'ouverture ne lance aucun
fournisseur. **Son rouge, vu à la main** : la condition `source.model == self.embed.model` retirée, le cas
« fixture-other » attache un dual ; la condition remise à l'identique. `Catalog::has_dual` dit si un dual est attaché.
La requête dense n'est pas éprouvée par un appel : sans dual attaché, elle n'a pas d'autre chemin que le dense.
