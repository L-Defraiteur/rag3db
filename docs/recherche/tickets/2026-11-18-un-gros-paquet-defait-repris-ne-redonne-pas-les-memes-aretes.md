# Un gros paquet défait puis repris ne redonne pas les mêmes arêtes que le témoin

- **État** : ouvert (18 novembre 2026, session recherche, en jouant la passe du lot de la garde des comptes).
- **Gravité** : divergence de données après une reprise. Les comptes sont égaux, mais onze tables d'arêtes
  diffèrent par leur empreinte ; aucun uuid n'est introuvable par sa clé.
- **Atteignable en service** : oui, après l'arrêt d'un gros paquet en cours d'écriture puis sa reprise.
- **Touche rag3weaver** : oui (le paquet en transaction, la reprise, l'écrivain des générations).
- **Famille** : produit
- **Pour** : recherche pour le constat ; la cause va à qui tient l'écrivain des générations ou le paquet en
  transaction (everything declarative, ou hotfixs). Réattribution par c7.
- **Signalé par** : recherche.

## Ce que c'est

`e2e_tx_par_paquet_arret::un_gros_paquet_defait_se_reprend_et_son_point_de_reprise_finit`
(`tests/e2e_tx_par_paquet_arret.rs:762`) compare, ligne à ligne, une base reprise après un paquet défait à
une base témoin indexée d'un trait :

```
▸ ligne à ligne : 50 tables, 11 écarts ; introuvables par la clé : reprise 0, témoin 0
la reprise et le témoin, ligne à ligne (propriétés et arêtes) :
  arêtes DEFINED_IN Scope→File (direct) : reprise "100000:60c6376efc912ccd", témoin "100000:6555f95d1a0d2a7b"
  arêtes DEFINED_IN Scope→File (inverse) : même écart
  arêtes DEFINES Scope→Symbol (direct) : reprise "100000:2fba040672fde966", …
```

Les compteurs sont égaux (100 000 arêtes), et les empreintes diffèrent : ce sont les mêmes nombres d'arêtes,
mais pas entre les mêmes extrémités.

**Rouge sur master seul** : 867ff28f8b, le 18 novembre, lib 59ac9c86b, à luciepc
(`~/.cache/rag3weaver-build/garde-base-tx.log`), en 341 s. Même rouge sur le lot de la garde des comptes
(`passe-garde-115633/e2e_tx_par_paquet_arret.log`), qui ne touche pas aux arêtes.

La suite n'est exigée par aucun chemin de `tools/poste/required_suites.txt`. Elle n'a donc pas été jouée par
les lots qui l'ont rougie. Son dernier changement est e621519045 (13 novembre). Le rouge est probablement
venu avec l'écrivain des générations (2a′-2), qui réécrit les arêtes d'une ligne renée ; ce n'est pas
vérifié.

## Ce qu'il faut pour le fermer

- Départager par bissection entre le dernier vert connu de cette suite et 867ff28f8b, sous `poste lourd`.
- Le paquet repris doit redonner les mêmes extrémités que le témoin. Si l'écrivain les reprend sur des
  lignes renées (un uuid neuf), le témoin doit comparer ce qui est visible, par la clé, et non les offsets.
  À trancher avec le propriétaire de l'écrivain.
- Ajouter cette suite à `tools/poste/required_suites.txt` pour les chemins de l'écrivain, pour qu'elle ne
  rougisse plus sans être jouée.
