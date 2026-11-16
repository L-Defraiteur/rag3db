# SHOW_INDEXES : rouge binder réel, avant tout correctif produit

Même [ticket](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md).
GO distinct consommé UNE fois `C7-ROWS-RED-B307-16nov2400`, source
`71fbe786fcf0b2314e72b3eb4b8484faf590d78c`, pilote
`b307f74cc21587cb982d6ef44cdb95763f992cba`, recette
`37e29c250113c12bd4ff0385a1ca06590c41cfa519268c3590bf699223387867`.
L'ancien refus de compilation5099/DB833 reste une preuve distincte conservée.

## Le résultat demandé, réellement observé

Compilation1012/1012 terminée code0 en211,116s. ELF/extension/archive moteur/
header/cache/commandes exportés et hashés AVANT le test. Le seul exact
`VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero` dure32ms :
code natif1, un cas run/completed, une failure, aucun disabled/error/skip.
BUILD_ID natif émis une fois :
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.

Erreur réelle dans `expectKnownZero`, ligne60 :

```
[check: rows-outside-graph-is-readable] Binder exception: Variable rows_outside_graph is not in scope.
```

Le vrai callee est `conn->query("CALL SHOW_INDEXES() RETURN index_name, rows_outside_graph;")`.
Les prémisses de setup (extension disponible/chargée, table et quatre vecteurs
manuels créés, un index cible READY) ont passé avant cet échec. Ce n'est ni une
méthode C++ absente ni une évaluation Graph/count substituée à l'API.
Le parseur strict du pilote accepte le XML/message natif SANS modification.

Cela démontre la projection manquante, pas le nombre exact de lignes hors
graphe. Zéro connu/nonNULL, schéma neuf à neuf colonnes et huit champs stables
ne sont PAS encore jugés : ces assertions suivent le binder refusé. Les trois
autres cas (READ ONLY, plage mêlant lignes déjà indexées, NULL/suppression et
reconstruction) sont COMPILÉS mais **NON JOUÉS**. Aucun résultat extrapolé.

## Borne et quiescence

Vrai poste `run-p76362-i151009530.scope`, début1194331.929516048,
cpu.max500000/100000, POSTE_FILS5/j5 et quota enfant500%,32G/swap0. Clone6,083s,
checkout0,765s, configure0,715s, code0 et aucun résidu de phase. Unité privée
`cx-hotfixs-rows-red-02.service`, budget2400 unique après acquisition.

Fin219,838s, unitéinactive/dead, Resultsuccess, MainPID0, ControlGroupvide,
quiescenttrue, aucun PID attribué restant. Artéfacts réhashés après cette
quiescence. Collecteur code0 signifie uniquement **preuves du rouge recueillies** ;
le résumé réel finit `EXPECTED_BINDER_RED_OBSERVED; NOT A DELIVERY PASS`, puis
**ROUGE**. Aucun GO livraison/master ni passage produit vert. Poste rendu,
aucun retry/reset, aucun service/SDK/modèle/PG/base de Lucie touché.

## Preuves conservées et suite

[Manifeste complet](preuves/rows-red-02/manifest.json) :48fichiers,
755407689octets réhashés PC puis portable,0écart ; SHA256
`f49462395124e1e605d9303d743c127f665a3adc160e668e471431a60a5c52ee`.
[Archive des logs/recettes/reçus/XML/header/cache](preuves/rows-red-02/receipts-and-logs.tar.gz)
SHA256 `401b0aeb055452783ddfc6aad6fe5bfd2c1fd8c5b58de5333c75ddfcb206fde6`.
Seuls bundle/ELF/archive moteur/extension binaires volumineux exclus de cette
archive Git, tous conservés et rehashés dans le paquet entier aux DEUX postes :
PC `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v2`, portable
`~/.cache/rag3db-moteur-notes/rows-outside-graph-red-02`.

Six artéfacts vérifiés localement contre manifeste natif, notamment ELFtest
27057880octets SHA `c7e96109572fb990946872dc674410666ef25701277e3e7fad3309e4de6392a6`,
archive moteur SHA `e49fadfe1f30353c58117135e43f4bf0372d11589193915962b59c0599892e99`,
extension vector SHA `6eb1d59315b75bf08f765d0c768ebec162bc62d19d4cb05db9a3275102745b9e`.
Stdout natif exact SHA `0096bde52eadaff44e53b5cec917c4bd2d529eb85d0a93f2cc08278a075eba27`.
Aucun ELF ou journal réécrit, aucune preuve antérieure supprimée.

Après cette preuve, C7 demande PRÉPA du correctif existant : chemins réservés,
capacité de lecture commune, cohérence holder/graphe/snapshot, refusNULL et coût,
contre-exemples queuepositive/NULL/transactions avant édition de production.
Contrat inchangé : vecteurs éligibles visibles effectivement ABSENTS du graphe
réel, sinon NULL ; jamais nombre physique moins watermark sous ce nom. Les
quatre témoins restent inchangés. Le GO ci-dessus est CLOS : aucun build/rejeu,
nouvelle lecture native, SDK ou service dans cette préparation. Ticket ouvert.
