# Le repassage d'une base par l'écrivain des générations coûte dix fois le chemin d'avant

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit
- **Gravité** : coût. Une base d'avant repasse en entier une fois, puis à chaque montée de
  `REGLES_DE_L_ANALYSE` ; sur `code-rag3db-depot`, environ 45 min au lieu de 4 à 5
- **Atteignable en service** : oui, dès que l'écrivain est sur master : la première synchronisation
  de chaque base de Lucie, puis chaque montée des règles
- **Touche rag3weaver** : oui (`src/code_sync.rs` : `ecrire_la_table`, `oublier_les_aretes_du_fichier`,
  `reingest_files_par_generations`)
- **Pour** : everything declarative (ED)
- **Signalé par** : banc, parité de l'écrivain d087e02c7 contre e95a1b8c6 (15 novembre 2026, 14 h 48 ;
  `docs/banc/14-11-2026/02-le-juge-de-parite-de-search-code.md` § 7)

## Ce que c'est

Le repassage d'une base sans version de l'analyse (`analyse` vide) passe chaque fichier par
l'écrivain. Mesuré par banc :
- parité propre : 2 656 s contre 254 s au parent ;
- base migrée : 2 971 s contre 282 s.
Le parent réécrit sur place. L'écrivain, lui, fait **par fichier** :
- deux lectures du grain (Scope, File) ;
- au plus quatre SET de `_jusqu_a` ;
- les DELETE ;
- environ sept DETACH d'oubli des arêtes ;
- puis la recopie, par ligne remplacée et par relation.
Sur des milliers de fichiers, ce sont des dizaines de milliers d'instructions courtes.

## La piste

Passer du grain du fichier au grain du paquet, comme l'ingestion :
- une lecture par table pour tout le paquet (`file_path IN $chemins`, sous la visibilité) ;
- les SET et les DELETE en un UNWIND par paquet ;
- l'oubli des arêtes par paquet ;
- la recopie en un UNWIND des paires (ancien, neuf) par relation.
Le contrat de l'IN par clé du cœur (7897f2453, accès par clé) et l'IN haché (e5e626803) aident dès la
lib qui les porte.

## Le témoin

Il compte, sur le repassage d'une source de N fichiers, les instructions envoyées au moteur. Elles
doivent croître avec le nombre de paquets, pas avec N. Rouge attendu sur la tête du lot. Puis la
mesure de banc rejouée, avec pour cible un facteur au plus deux sur le parent.

## Ce qu'il faut pour le fermer

Le témoin rouge puis vert, et la mesure de banc rejouée sous la cible.
