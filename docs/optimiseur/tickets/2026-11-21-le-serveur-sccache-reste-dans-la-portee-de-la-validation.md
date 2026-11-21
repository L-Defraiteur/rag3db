# Le serveur sccache reste dans la portée de la validation, qui rougit « descendants laissés »

- **État** : ouvert (21 novembre 2026, écrit par G à la demande de c7 ; correctif déposé : lot poste-sccache-server-outside)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : `poste` et le confineur de validation)
- **Gravité** : moyenne. Un faux rouge à chaque passe sur un target neuf, à luciepc ; sans doute aussi le rouge
  `integres` du 20 novembre.
- **Atteignable en service** : non, les passes seulement
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/poste`)

## Ce que c'est

Depuis le 20 novembre, `poste` exporte `RUSTC_WRAPPER=sccache` par défaut. Au premier bâti d'un target neuf, aucun
serveur sccache ne tourne : le client que cargo appelle en démarre un, **dans la portée de la commande**. Ce serveur
survit à la commande, et le confineur de validation le relève comme un descendant laissé (« validation : ROUGE
(descendants laissés après la commande) », `verdict.json` de `validations/validation-_f948t1c`, relevé par codeparsers).

## Le correctif (lot poste-sccache-server-outside)

`poste` démarre le serveur lui-même (`sccache --start-server`), avant la commande et dans son propre processus, chaque
fois que `RUSTC_WRAPPER` vaut `sccache`. Le client trouve alors un serveur vivant et n'en lance plus dans la portée. Le
relevé des descendants du confineur n'est pas relâché. Témoin (`test_poste.sh`, cas 60) : un port sccache neuf, une
commande qui appelle le client ; le cgroup du serveur doit différer de celui de la commande (rouge avant le correctif).
