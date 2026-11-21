# Le coût des occurrences (lot 2 de l'intra-scope)

21 novembre 2026, session codeparsers, à la demande de c7. Conception :
[`19-11-2026/04-l-intra-scope-conception.md`](../19-11-2026/04-l-intra-scope-conception.md).

Le lot 2 porte au graphe **chaque référence avec sa plage d'octets et son rôle**, pour que `usages` rende la colonne.
Avant tout code, c7 demandait le coût d'écriture : aujourd'hui une réécriture coûte 1,0 s par fichier, surtout dans
« entités · dérivations », et les occurrences ne devaient pas doubler ce temps sans qu'on le sache.

## La mesure

- **Instrument** : la sonde d'écriture d'arbre (`history_layers::tests::sonde_ecriture_en_couche`, reprise de
  2329f7082e), mode ordinaire (`reingest_files`). C'est celle de la page
  [`arbre-principal/20-11-2026/04-ce-que-coute-une-fourche.md`](../../arbre-principal/20-11-2026/04-ce-que-coute-une-fourche.md).
- **Ce qui est réécrit** : les 419 fichiers qui diffèrent entre l'arbre indexé de la copie de G
  (`repetition-versions-code-rag3db-depot-20261119-194739`) et master.
- **Les deux passes** : chacune sur un reflink neuf de la copie, avec le même binaire release (features `code` et
  `token-count`), sans puis avec `RAG3WEAVER_PROTO_OCCURRENCES`.
- **La lib** : prouvée par la sonde, `CALL BUILD_ID()` = `rag3db-build-4753938c9d…`, dans les deux passes.
- **Le poste** : à la tablette, sous `poste mesure` bloquant, dans la fenêtre de c7 (06 h 19 à 06 h 52).
- **Le prototype** est jetable, sur la branche `codeparsers/occurrences-cost-probe` qui ne sera jamais déposée. Il
  écrit une arête `OCCURS` Scope → Symbol par site de référence, avec `line`, `column`, `byte_start`, `byte_end` et
  `role`. Les octets sont factices, uniques par site : leur valeur ne pèse pas sur le coût.

| | total | par fichier | entités · dérivations | drain · liens | symboles |
|---|---|---|---|---|---|
| sans | 388,7 s | 0,93 s | 167,9 s | 8,7 s | 15,7 s |
| avec | 396,5 s | 0,95 s | 175,6 s | 9,5 s | 17,3 s |

**Le coût : +7,8 s, soit +2 %, environ 19 ms par fichier.** La référence de 385 s se retrouve à 1 % près. Une passe
de chaque côté, donc le bruit n'est pas mesuré. L'écart se loge surtout dans « entités · dérivations » (+7,7 s), sans
explication encore.

## Les 46 % perdus

248 610 occurrences ont été mises en file (environ 590 par fichier), mais **seulement 134 592 arêtes `OCCURS` sont en
base** : 46 % des sites sont perdus.

La cause est l'écriture des liens. Elle ne déduplique rien avant le moteur (`LinkRecordNode` groupe par relation et
par propriétés), mais son Cypher fusionne sur les deux bouts seuls :
`MERGE (a)-[r:REL]->(b) SET r.x = item.x` (`dialect.rs`, `batch_link_labeled`). Deux sites d'un même couple
scope → symbole tombent donc sur la même arête, et le second écrase les propriétés du premier. Une arête par site,
telle quelle, ne tient pas la promesse du lot.

## Les deux voies

1. **Une arête par site, dont la clé de MERGE porte les octets** (voie de c7, essayée d'abord) :
   `MERGE (a)-[r:OCCURS {byte_start: item.byte_start, byte_end: item.byte_end}]->(b)`. Le moteur accepte plusieurs
   arêtes entre deux mêmes nœuds, et la plage d'octets vit sur l'arête.
   - **Ce qu'elle ne touche pas** : le catalogue.
   - **Ce qu'elle touche** : une clé de MERGE déclarée par relation, dans `RelationConfig` et `Write::Link` de l'IR,
     dite par chaque dialecte, Postgres compris.
   - **Le prototype** (96374241ba) la limite à `OCCURS`, dans le dialecte rag3db.
   - **Coût attendu** : celui de la mesure, plus les 46 % de liens qui n'étaient pas écrits ; à mesurer dans une
     seconde fenêtre.
2. **Une entité `Occurrence`** (clé = scope et octets du site), simple : ni plein texte, ni morceaux, ni vecteur,
   comme Lucie l'a dit.
   - **Ce qu'elle touche** : la règle du catalogue « toute entité est cherchable ». Une entité sans champ de contenu
     est refusée aujourd'hui, et `Symbol` paie le pipeline complet pour un nom (le commentaire de `symbol_config` :
     12,5 s pour 3 275 symboles).
   - **Coût** : non mesuré. Les entités passent par les dérivations, l'étape la plus lourde de la réécriture.
   - **C'est une décision de conception, à Lucie**, si la voie 1 échoue.

## La voie 1 essayée : la clé de MERGE ne suffit pas

Le prototype 96374241ba (la clé de MERGE d'`OCCURS` porte `byte_start` et `byte_end`) a été joué sous `poste lourd` à
06 h 57 : un contrôle de justesse, pas une mesure. Lib prouvée 4753938c9d. Résultat : **toujours 134 592 `OCCURS`
pour 248 610 mises en file**, exactement comme sans la clé.

La fusion est en amont du Cypher : **le drain tient un lien pour une paire** (`LinkRecordNode`,
`dataflow/record_nodes.rs`) :

- **Le chemin COPY** (au-delà de `copy_seuil`) « garde la sémantique de MERGE » : il dédoublonne dans le lot, et il
  écarte les paires déjà en base (`copier_les_liens`, `presents_connus`).
- **Le chemin par tranches** garde « une paire, une fois, sa dernière valeur » (`derniers`, recette du banc du
  4 octobre 2026 : le moteur perdait un `SET` sur une paire répétée).
- **Le journal des liens neufs** (`crees`, `noter_liee`) et **l'annulation d'un lot** raisonnent aussi par paire
  (ticket du 19 octobre 2026).

La voie 1 demande donc **une identité de lien déclarée par relation** : la paire, plus des propriétés-clés. Elle se
lirait dans ces trois endroits et dans chaque dialecte. C'est un changement du modèle de lien du catalogue, aussi
profond que la voie 2 : **les deux voies sont des décisions de conception**, à Lucie.


- **Le choix entre les deux voies**, à Lucie. La mesure de la voie retenue suivra, dans une fenêtre de c7.
- **L'écart de `MENTIONS`** : 733 332 dans la passe sans, 733 344 dans les deux passes avec (la mesure, puis le
  contrôle de 06 h 57). Il suit le prototype, il n'est pas aléatoire : pas de ticket. Il reste à comprendre par où
  une arête `OCCURS` change douze `MENTIONS`, avant tout code du lot.
- **Les reflinks** sont gardés 48 h, sous `~/.cache/rag3weaver-build/mesures/occurrences-{sans,avec}-20261121-*`
  (manifeste « garder 2 »).
