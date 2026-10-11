# `add_ref` et `create_ref_type` jettent le rapport de leur ingestion — un échec partiel passe pour un succès

- **état** : ouvert (relevé par lecture le 11 octobre 2026, en recensant les outils d'écriture des gabarits livrés)
- **gravité** : réponse fausse — un outil rend `executed: true` sur une écriture qui a pu échouer en partie
- **atteignable en service** : oui, par les deux outils du gabarit `memory`, dès qu'une ingestion compte un échec (un embarquement refusé, une écriture du plein texte qui casse)
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

`Catalog::ingest_entities` rend un `FlushResult` : ce qui a été écrit, ce qui a échoué
(`failed`), les avertissements, et ce qui est prêt (`rendu_pret`). Un échec partiel
n'est **pas** une `Err` : il est compté dans `failed`.

- `EntityRecordNode` (les `put_*`) le lit, et refuse si `failed > 0` ;
- `AddRefNode` (`ref_nodes.rs`, l'ingestion de la `Ref`) et `CreateRefTypeNode`
  (l'ingestion du genre) appellent `ingest_entities(…)?` et **jettent le rapport**.
  Seule une `Err` les arrête.

Un échec compté passe donc pour un succès : `add_ref` rend `executed: true` et pose le
lien vers une `Ref` dont l'écriture a pu échouer en partie, et `create_ref_type` dit le
genre créé.

## La recette minimale

Par lecture : `grep -n "ingest_entities" src/dataflow/ref_nodes.rs`, deux appels, aucun
ne lit `failed`. Pas de recette jouée : il faut un échec compté, qu'un point
d'injection de faute dans l'ingestion fournirait (sur le modèle de celui du drain des
liens, deux variables).

## Le témoin

Aucun. À écrire avec le point d'injection : une ingestion qui compte un échec, et les
deux outils qui refusent en le disant.

## Ce qu'il faut pour le fermer

Lire le rapport, comme `EntityRecordNode` le fait : `failed > 0` est un refus qui dit
ce qui est resté (`rendu_pret`). Petit, et indépendant de la page de l'enveloppe d'un
outil d'écriture (`docs/11-octobre-2026-arbre-principal/03-l-enveloppe-d-un-outil-d-ecriture.md`),
qui le cite.
