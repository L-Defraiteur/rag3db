# Un gros paquet défait puis repris ne redonne pas les mêmes arêtes que le témoin

- **État** : fermé (18 novembre 2026, ED) — cause bissectée à 365dc75f74 (une première passe interrompue reprenait par l'écrivain des générations, le paquet repris naissait au rang 1) ; correctif `code_sync::first_pass_interrupted` (branche first-sync-resumes-by-snapshot) ; le test, rouge sur master et sur 365dc75f74, vert avec le correctif.
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

## La cause (ED, 18 novembre 2026)

**Les nœuds diffèrent aussi, pas seulement les arêtes.** Les écarts portent sur Scope, File et File_Chunk, en plus des
arêtes. L'empreinte des nœuds compte `_uuid` et `_depuis`. Relevé sur les bases laissées par un rouge, à luciepc
(`~/.cache/rag3weaver-build/e2e-gardes/e2e_tx_par_paquet_arret-20261118-144341-4031639`) :

- dans la reprise, 50 000 scopes ont `_depuis = 1` et 50 000 ont `_depuis = 0` ;
- dans le témoin, les 100 000 ont `_depuis = 0`.

Le contenu est le même. L'uuid d'une occurrence porte `_depuis` : les 50 000 du paquet repris ont donc d'autres
uuids, et les arêtes qui partent d'eux aussi.

**Le commit fautif est 365dc75f74** (« sync_source d'une source déjà indexée passe par l'écrivain des
générations »). Bissection à luciepc, lib 59ac, avec ce seul test :

| Commit | Verdict |
|---|---|
| c693496a8 | vert |
| 483479fc57 | vert |
| 3ad8950c4f | vert |
| d82ec8c916 | vert |
| de006b6de0 | vert |
| 365dc75f74 | rouge |
| 9556cb949b | rouge |
| 513ed0d54a | rouge |
| c33574eb9 (sous 59ac et sous 373b) | rouge |

La lib est hors de cause.

Le mécanisme :

1. La reprise relance `sync_source` sur une source dont la première passe s'est arrêtée entre deux paquets.
2. `source_deja_indexee` y voit un scope : la source passe donc par l'écrivain des générations.
3. L'écrivain fait naître le paquet repris au rang en attente (1).
4. Avant ce commit, la reprise prenait sa session d'instantané (takeover), et tout naissait au rang 0.

**Le correctif** (branche `first-sync-resumes-by-snapshot`) : une session d'instantané ouverte sur la source, alors
que rien n'est publié dans la couche, est une première passe interrompue ; elle reprend par l'instantané. La route de
l'écrivain garde ce qu'elle protège, les lignes publiées retirées : rien n'est publié dans ce cas.

L'autre voie, juger le test par la clé visible, n'est pas prise : deux bases au même contenu doivent avoir les mêmes
identités (parité de lucivy, ordre des ex æquo).
