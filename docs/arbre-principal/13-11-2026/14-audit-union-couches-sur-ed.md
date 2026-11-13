# Préservation de l’union couches2a / ED

Base master exacte4e2e2bfbb7c8ce670a49a06a36580d481d9abbc0 après landing ED.
Branche/WT neufs cx-arbre-couches-union-r1/cx-wt-couches-union, rebase depuis
paquet99b9796a01b8c6a98e2d44a591d446813333a806 sans conflit.

Audit indépendant [JSON](couches-2a/union-audit.json) et
[range-diff](couches-2a/union-range-diff.txt) :9patches strictement égaux.
Trois fichiers produit2a publication.rs/layers.rs/layers/tests.rs byte-identiques
(modes+OID) à application qualifiée69dc ; entrant atelier_fichiers.rs identique
à ED4e2e. Produit de l’union=overlay EXACT master4e2e+patchesproduit2a,
aucun delta parasite.7930 entrées master hors chevauchement préservées,
suppressions comprises. Révision de code après rebase d77046bb90e4a655375caa689ea5c7ae623604f8 ;
ce rapport et son audit sont ensuite ajoutés sans nouveau delta produit.

Aucune qualification rétroactive : preuves72db restent app69/piloteabaf.
Sourceunion différente doit recevoir qualification F8+liblarge+ciblé2a
selon plan13 et recette/pilote exacts encore à faire relire avant GO.
Tickets2a et parent restent ouverts, pas de master/retrait autorisé.


## Gel révisé après bilan ED Markdown

Master940ad65bf57461b714ff2671266bf84cb74f6966 = bilanED MD seulement
après4e2e. Branche/WT ENCORE NEUFS cx-arbre-couches-union-r2, rebase
depuis2009 sans conflit ;10range-diff égaux, produit byte-identique2009
(donc3fichiers2a identiques69/entrantatelierED préservé).
[Audit r2](couches-2a/union-r2-audit.json), [range-diff r2](couches-2a/union-r2-range-diff.txt).
Apports master hors chevauchements et suppressions préservés. Les fichiers
de ce nouveau bilan de gel restent documentaires, aucun runtime union joué.
Cette r2 remplace r1 comme application à qualifier ; ancien état conservé.
