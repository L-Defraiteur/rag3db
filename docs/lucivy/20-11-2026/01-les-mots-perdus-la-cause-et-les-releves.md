# Les mots perdus du plein texte : la cause, la 4.3.12 et les relevés du 20 novembre

Session lucivy, 20 novembre 2026. Ticket : `docs/lucivy/tickets/2026-11-19-l-index-de-code-a-perdu-des-mots-a-la-compaction.md`.

## La cause, en une phrase

Quand un document crée un mot neuf, le dictionnaire du shard le note dans une table d'attente. Cette table doit
l'oublier une fois son texte nommé par un commit, ce que règle l'époque de commit. Or seul `IndexWriter::prepare_commit`
faisait tourner cette époque, et le commit des shards de `ShardedHandle`, celui de rag3weaver, ne passe pas par là :
la table gardait donc chaque mot pour toujours. Elle rend en outre un id sans vérifier que son texte existe encore.
Quand la compaction nocturne avait retiré le texte d'un mot qu'aucun document ne citait plus, le fichier réindexé qui
le contenait de nouveau reprenait l'ancien id, sans texte, et ne sortait plus en recherche sur ce mot.

Corrigé dans lucivy 4.3.12 (PR #54) : `flush_workers` fait tourner l'époque, et la compaction retire de la table les
ids qu'elle jette. S'y ajoutent deux filets : le sursis d'un cycle (#50), et `maintain`, qui nomme les textes qu'il
reprend (#52). Tag `v4.3.12` le 19 novembre à 22:46 ; publiée sur crates.io, npm et PyPI ; pointeur dans rag3weaver
c42baa7c0f ; hôtes des deux postes sur 4.3.12 depuis 00:26.

## Relevé 1 : la sonde de nuit (par le chemin de l'hôte)

Le test ignoré `the_nightly_compaction_during_reindexing_loses_no_word` (`dataflow::index_nodes`) : 120 tours de
`spawn_index` sur un dépôt dont les fichiers changent, pendant que `fts_maintenance_nocturne`, avec compaction, tourne
en boucle sur le même catalogue. Joué à luciepc sous `poste lourd`, lib 6e2476d41.

| lucivy | maintenances | ids perdus |
|---|---:|---:|
| 4.3.10 (ba6058e990) | 495 | **534** (`File.path`, 158 + 128 + 120 + 128) |
| 4.3.12 (2a604f63f8) | 431 | **0** |

## Relevé 2 : `check_dictionary` sur les trois bases de code

Copies du seul dossier `.fts`, faites par G de 08:51 à 08:55, chaque base fermée quelques secondes, sous l'hôte
baccb2ea6e (lucivy 4.3.12, lib 4ad394f4d2). Chaque index `Lucivy_*` contrôlé.

| base | ids perdus | en sursis |
|---|---|---:|
| code-rag3db-depot | `Lucivy_Scope` : content 532, docstring 133, name 25 ; tous les autres index : 0 | 0 |
| code-lourd | 0 | 0 |
| code-principal | 0 | 0 |

Le 19 novembre, `Lucivy_Scope` du dépôt en comptait 4 748 (content), 2 213 (docstring) et 283 (name). Le repassage du
matin a réécrit la plupart des documents touchés. Les 690 qui restent datent d'avant 4.3.12, et seul un rebâti les
rendra.

## Relevé 3 : la compaction rallumée

Avec l'accord de c7, les conditions sont réunies : les hôtes sont en 4.3.12, la sonde est verte et prouvée rouge sur
4.3.10, et les bases ne portent aucune perte neuve ni texte en sursis. Les cinq manifestes coupés le 19 à 17:44 sont
hors du dépôt, dans la configuration des hôtes. Consigne donnée à G : rallumer `fts.maintenance.compact`, garder les
copies `.avant-compact`, puis fermer et rouvrir chaque backend, une base à la fois. Le rebâti du `Lucivy_Scope` du
dépôt se fera à sa fenêtre de re-bascule des bases de luciepc.

**Verdict attendu demain matin** : l'état des hôtes (la note de maintenance dit les textes repris du sursis), puis un
`check_dictionary` neuf sur des copies. On attend 0 perte, hors les 690 connus tant que le rebâti n'est pas fait.
Le ticket se ferme sur ce verdict.
