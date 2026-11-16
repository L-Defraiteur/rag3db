# Les ex æquo de la recherche n'ont pas d'ordre stable

- **État** : fermé (16 novembre 2026, ED) — `fts_handle::search_hits_with_ties` demande à lucivy `limit` + 8 puis double tant que le dernier hit est ex æquo avec le `limit`-ième (au plus 4 096) ; `search_bm25_chunked` trie par score puis uuid (stable : le contenu ou la clé le donne, pas la base) et coupe lui-même. Témoin `fts_handle::tests::a_search_returns_every_tie_of_its_cut` : six ex æquo dans deux index d'ordres opposés, ROUGE sur 154f72e75 (`{1, 5}` sur six), vert ensuite.
- **Gravité** : réponse instable. Les mêmes documents et les mêmes scores sortent dans un ordre qui dépend de la
  disposition des segments : deux bases qui tiennent le même contenu rendent leurs ex æquo dans des ordres
  différents, et un juge de parité les voit différer.
- **Atteignable en service** : oui. Toute recherche dont deux résultats ont le même score : des morceaux au contenu
  identique, ou des scores qui tombent égaux. Une coupe du top k sur des ex æquo change aussi qui entre.
- **Touche rag3weaver** : oui, le tri des hits de la recherche (`search_code`, le nœud lexical).
- **Famille** : produit
- **Pour** : everything declarative (ED), qui tient le nœud lexical et l'Admission

## Ce qui se passe

Lucivy départage les ex æquo par l'adresse du document (segment, puis document), comme tantivy. C'est stable pour
un même index, mais **pas d'un index à l'autre** : un index rebâti ou écrit autrement répartit ses documents dans
d'autres segments. Les `_node_id` ne valent pas mieux, puisqu'ils changent d'une base à l'autre (ce sont des
décalages de lignes).

Mesuré le 16 novembre 2026 (lucivy seule, `test_compare_two_indexes`, branche lucivy `bm25-longueur-de-la-vue`), sur
les plein texte `ScopeContent` du banc : `ecr-base-ecrivain-neuf-5-parent-reindex-20261115` (neuf) et
`preuve-elague2-20261116` (rebâti depuis les lignes). Mêmes contenus, mêmes N et tokens, scores égaux à 1,6e-7 près ;
trois requêtes sur quinze ont pourtant un top 10 dans un autre ordre, toutes avec un **écart de score nul** au rang
qui diverge :
- `contains read_text` diverge au rang 4 ;
- `contains value` au rang 3 ;
- `contains transaction rollback` au rang 4.

## Ce qu'il faudrait

Un ordre total et stable au-dessus de lucivy : le score, puis une identité qui ne dépend pas de la base. Pour un
morceau de code, ce serait le chemin, puis la ligne de début, puis le nom (ou l'empreinte du contenu), appliqué là où
rag3weaver trie les hits et où il coupe le top k.

Pour que la coupe elle-même soit stable, demander à lucivy un peu plus que k (k plus les ex æquo du k-ième), puis
trier et couper en haut. Sinon, le k-ième rang reste celui que la disposition des segments a choisi.

## Complément qualifié : identité stable entre bases

La première clôture ci-dessus décrit le correctif de coupe déjà livré. Le complément sourcebd76 trie score puis identité chemin/ligne/nom et UUID résiduel ; il s'applique au lexical, aux résultats vectoriels et à la fusion. L'UUID d'occurrence seul ne garantit pas l'ordre entre base réécrite et base neuve car il porte _depuis. Le contenu, l'identité et l'occurrence restent distincts ; aucune mutation d'UUID/coordonnées ni nouvelle APIoutil. Si chemin/ligne/nom manquent ou changent, la stabilité interbases n'est pas promise. Limite de surcollecte4096 inchangée ; le juge de six occurrences ne prouve pas toutes les variantes de fusion/vectoriel.

Qualification réelle sourcebd76f557928b5078f98a235f1cb0b5f78e7360d8/pilote13c8bf21e6c34d4480f034e66453bec22330e6a0, preuves574134fb1e1a8330a38f51d23bc6c4b4209b3f86 : F88/8,lib2595/0/72ignorés,NAV45/0/0,code36/lectures27 et deux ciblés distincts verts, canonique complète sous délai initial sansreset. [Rapport et manifeste](../../everything-declarative/16-11-2026/09-preuves-canon-ties-v3.md). RO Archi et Mémoire favorables bornées ; anciensrougesconservés. Candidat préparé surmasterf83 documents entrants seuls, aucunGOmaster/déploiement/retrait déduit.
