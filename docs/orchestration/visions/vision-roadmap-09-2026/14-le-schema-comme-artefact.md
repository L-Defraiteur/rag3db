# Le schéma comme artefact

**30 août 2026.** Écrit après la première soirée où le dialecte PostgreSQL a
parlé à une vraie base. Deux questions de Lucie s'y rejoignent : *« les
migrations devraient peut-être produire des artefacts rejouables qu'on puisse
commit »* et *« pas de RPC à créer automatiquement ? »*. Ce sont les deux faces
d'une même lacune.

## 1. Ce qui existe déjà, et qu'il ne faut pas réécrire

`src/dataflow/migrations.rs` est un lanceur de migrations complet :

- des fichiers `migrations/001_....mmd` — du **Mermaid**, donc lisibles,
  diffables, commitables ;
- exécutés comme des graphes de dataflow, donc avec checkpoints, reprise après
  incident, observabilité et *undo* ;
- `status`, `pending`, `dry_run` (qui rend un plan avant de toucher quoi que ce
  soit), `rollback`, et `check_reversible`.

C'est déjà l'artefact rejouable et commitable que la question appelle. Personne
ne devrait le redécouvrir : le manque n'est pas là.

## 2. L'asymétrie, qui est le vrai sujet

Une base peut changer de forme par **deux** chemins, et un seul laisse une
trace.

**Le chemin déclaré.** Quelqu'un écrit `003_ajoute_un_champ.mmd`, le relit, le
commite, le lance. Il existe sur disque, il se rejoue, il se relit dans six
mois.

**Le chemin induit.** `register_entity()` émet du DDL à la volée. Une
`EntityConfig` qui gagne un champ fait un `ALTER TABLE` au prochain
enregistrement. `migrate_scope_columns()` ajoute `_org` et `_project` à toutes
les tables connues, une fois, sous la clé méta `schema_version` — et le dit par
un `eprintln!`. Depuis aujourd'hui, `poser_index()` ajoute des index secondaires
de la même façon.

Aucun de ces changements ne devient un fichier. Ils sont **corrects** — ils sont
même testés — mais ils n'existent que dans la base vivante et dans le code qui
les a produits. On ne peut ni les relire avant qu'ils arrivent, ni les rejouer
ailleurs, ni les comparer à ce qu'une base contient réellement.

Tant qu'il n'y avait qu'un backend et qu'une base de développement, ça passait.
Avec un dialecte PostgreSQL qui vise des bases de production — celles qu'on ne
laisse pas un programme modifier sans qu'un humain ait lu le DDL — ça ne passe
plus.

## 3. Ce qu'un artefact devrait porter : l'intention, pas le SQL

La tentation serait d'enregistrer le SQL émis. C'est le mauvais niveau, et le
dialecte le prouve : le même changement s'écrit
`CREATE NODE TABLE Product(...)` d'un côté et `CREATE TABLE product (...)` de
l'autre, avec un `_row_id BIGSERIAL` que kuzu n'a pas. Un artefact en SQL est
l'artefact **d'un** backend.

L'artefact doit dire *« l'entité Product gagne un champ `price` de type
double »*, et chaque dialecte le rend. C'est exactement la couche qui existe
déjà : `SchemaDialect` prend une intention (`ColumnDef`, `ColumnType`) et rend
du DDL. Il manque seulement de **capturer l'intention en passant**, au lieu de
la consommer.

La forme naturelle est celle des migrations d'aujourd'hui — du Mermaid, que
[le document 04 de la session du 30 août](../30-aout-2026-06h00/04-le-mermaid-lu-et-ecrit-par-les-deux.md)
a vérifié lisible **et** écrivable par les deux modèles qu'on utilise. Un
changement induit devrait pouvoir s'écrire dans le même langage que celui qu'on
écrit à la main : c'est la condition pour qu'un agent propose une migration
qu'un humain relit.

## 4. Trois usages que ça débloque, et un seul est « rejouer »

**Rejouer.** Le cas évident : le même changement, sur une autre base, dans le
bon ordre.

**Relire avant.** `dry_run` existe pour les migrations déclarées. Le chemin
induit n'a pas d'équivalent : on ne peut pas demander *« qu'est-ce que
`register_entity` va faire à ma base de production ? »* autrement qu'en le
faisant. Un plan émis sans exécution, c'est la même mécanique tournée d'un cran.

**Constater l'écart.** Le plus utile des trois, et le seul qui n'existe nulle
part : comparer ce que la configuration **veut** à ce que la base **contient**.
Une colonne ajoutée à la main, un index supprimé pour dépanner, une table restée
d'une version d'avant — aujourd'hui rien ne le dit. Or `initialize()` interroge
déjà la base pour migrer ; il lui manque de savoir *dire* la différence plutôt
que de la corriger en silence. C'est la règle n° 3 du domaine de travail
appliquée au schéma : distinguer « ça n'existe pas » de « ça n'est pas ce que
tu crois ».

## 5. Et les RPC — parce qu'un schéma n'est pas que des tables

`PostgresDialect` se décrit comme *« Supabase-compatible »*. C'est une
aspiration, pas un fait, et la distance vaut d'être nommée.

Sur Supabase, ce qui traverse le réseau passe par PostgREST. Or **une recherche
vectorielle ne s'exprime pas en PostgREST** : `ORDER BY embedding <=> $1` n'a
pas de traduction dans son langage d'URL. La seule voie est une fonction SQL
exposée en RPC :

```sql
CREATE FUNCTION rag3weaver.chercher(requete vector, k int) RETURNS ...
```

Ce qui veut dire qu'un dialecte qui n'émet que des tables ne produit pas un
projet Supabase utilisable — il produit une base sur laquelle il faut encore
écrire à la main ce que le moteur sait déjà faire. La même remarque vaut pour
les politiques RLS et les droits : dès qu'une base est multi-locataire et servie
directement à des clients, `_org` et `_project` ne sont plus des colonnes de
filtre, ce sont des frontières que la base doit défendre elle-même.

Ça élargit ce qu'un dialecte doit savoir décrire — non pas *« quel DDL pour
cette table »* mais *« de quoi est fait un schéma sur ce backend »* : tables,
index, **fonctions**, politiques, droits. Chacun de ces objets est un artefact
qu'on relit et qu'on commite, exactement comme les tables.

## 6. Ce qui n'est pas tranché

- **Qui écrit l'artefact.** Le moteur, en enregistrant ce qu'il s'apprête à
  faire ? Ou un agent, qui propose une migration relue avant d'être appliquée ?
  Les deux mènent au même fichier, pas à la même boucle.
- **Que faire d'un changement induit déjà appliqué.** Refuser de l'appliquer
  sans fichier serait cohérent, et rendrait le développement pénible. L'écrire
  après coup est plus doux et plus faux. Un mode par environnement — libre en
  développement, déclaré en production — est le compromis évident, donc celui
  dont il faut se méfier.
- **La granularité.** Un artefact par appel, ou un par session de changements ?
  L'unité de relecture n'est pas l'unité d'exécution.
- **Les fonctions et les politiques dans quel langage.** Le Mermaid décrit bien
  des structures et des liens ; une fonction SQL est du code. La frontière entre
  ce qui se décrit et ce qui s'écrit passe probablement là.

## 7. Où ça se branche

Rien de tout ça ne demande une couche nouvelle. `SchemaDialect` porte déjà
l'intention, `migrations.rs` porte déjà l'exécution rejouable, `dry_run` porte
déjà le plan-sans-effet, et le Mermaid porte déjà le format. Ce qui manque est
un **point de capture** entre les deux, et une méthode de dialecte de plus pour
dire ce qu'un backend appelle un schéma.

C'est à faire après que le backend PostgreSQL soit prouvé — recherche BM25,
relations, cellules — parce qu'un artefact qui décrit un schéma qu'on n'a pas
encore su faire tourner décrirait une supposition.
