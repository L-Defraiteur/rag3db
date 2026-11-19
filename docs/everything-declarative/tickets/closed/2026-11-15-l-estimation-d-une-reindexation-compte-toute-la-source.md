# L'estimation d'une réindexation compte toute la source comme un premier index

- **État** : fermé (19 novembre 2026, ED) — sur une base déjà indexée, `estimate_of` ne compte que les fichiers dont l'empreinte indexée (contenu et version de l'analyse, la même que la synchronisation : `code_sync::indexed_fingerprints`) diffère du disque, plus la dette réelle (`dense_missing`, à la taille moyenne d'un morceau de la base) ; les inchangés ne coûtent rien ; une source jamais indexée, ou après une passe en masse interrompue, s'estime comme avant. L'estimation dit la réindexation (« N changé(s), M inchangé(s), K morceau(x) déjà dû(s) »). Témoin `dataflow::index_nodes::tests::the_estimate_of_an_unchanged_reindexing_announces_no_vector`, ROUGE sur b4efce008a (894 s annoncées, confirmation demandée), vert avec le correctif. **Hors de ce lot** : l'estimation au rythme observé (relevé du 15 nov. ci-dessous), qui demande un étalonnage par poste.
- **Famille** : produit
- **Gravité** : réponse fausse et blocage. L'outil `index` refuse sans `confirm=true` une réindexation
  qui ne coûte presque rien, sur la foi d'une estimation d'un premier index (environ 10 min annoncées
  à la répétition de la bascule)
- **Atteignable en service** : oui. Toute réindexation d'une base déjà indexée dont la source passe le
  seuil de confirmation
- **Touche rag3weaver** : oui (`src/dataflow/index_nodes.rs`, `estimate_of` ; `src/estimate.rs`)
- **Pour** : everything declarative (ED)
- **Signalé par** : c7, répétition de la bascule par arbre principal (15 novembre 2026, 18 h)

## Ce que c'est

`estimate_of` estime le temps des vecteurs à partir des fichiers de la source, de leurs octets et du
rythme d'embarquement. Il ne lit ni la dette réelle de la base (`dense_missing`) ni ce qu'une
réindexation ajoutera. Une réindexation d'une base indexée s'estime donc comme un premier index.

## Ce que coûte vraiment un repassage de l'écrivain

Les `Scope` sortent identiques, avec les mêmes morceaux et sans vecteur neuf. Chaque ligne `File` neuve
reçoit en revanche son morceau (`File_CHUNKED_FROM` 7 608 → 15 218 chez banc), soit environ un petit
morceau par fichier. Le coût de l'écrivain est à part : un `File` dont seule l'analyse change pourrait
garder son morceau.

## Le témoin

Rouge d'abord : sur une base déjà indexée, sans dette, l'estimation d'une réindexation sans changement
annonce zéro vecteur et ne demande pas de confirmation.

## Ce qu'il faut pour le fermer

L'estimation d'une base déjà indexée part de la dette réelle et de ce qui changera (les fichiers dont
l'empreinte diffère), pas de la source entière ; le témoin vert.

## Relevé du 15 nov., 21 h (arbre, répétition de la bascule sur copies)

L'estimation du rythme observé se trompe aussi, 2 à 2,5 fois : au paquet 1, le journal annonçait « ~27 min au
rythme observé » pour code-lourd et « ~19 min » pour code-principal ; le repassage a pris 61 et 49 min. Le rythme
du premier paquet ne compte ni les paquets plus lourds ni la phase des vecteurs. L'estimation de l'outil `index`
avant le lancement disait « vecteurs : environ 10 min » (d'où `confirm=true`) ; le réel des vecteurs seuls est
dans le relevé. Les lignes « vecteurs X sur Y, reste environ Z s » tenaient à la minute près.

Le relevé horodaté (la dernière ligne du journal toutes les 15 s, avec son instant) est à luciepc :
`~/.cache/rag3weaver-build/bascule/journal-code-{lourd-20261115-1838,principal-20261115-1835}/repassage.jsonl`.
Il donne le temps de chaque paquet et de chaque phase, ce que le journal seul ne dit pas ; les deux estimations
(avant le lancement, au rythme observé) se jugent sur ce même relevé.

Pour une estimation juste, sans constante (vision du 15 nov., point 7) : horodater chaque ligne du journal
d'index ; un débit observé par phase (plein texte, relations, vecteurs), lissé sur les derniers paquets ; le
reste compté en fichiers de la vue (pointe moins génération publiée) ; un débit de départ par poste et par
service d'embarquement, mesuré par un banc d'étalonnage joué à l'avance.
