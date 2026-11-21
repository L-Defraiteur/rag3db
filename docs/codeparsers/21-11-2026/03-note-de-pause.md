# Note de pause, session codeparsers (21 novembre 2026, redémarrage de la tablette)

## Où j'en suis

Le lot 2 de l'intra-scope : les occurrences au graphe. Lucie a choisi la **voie (a)**, des liens à clé. Le détail est
dans les pages [01](01-le-cout-des-occurrences.md) et [02](02-les-liens-a-cle.md), et dans la fiche de memoire-longue
« Les occurrences des références au graphe ».

- **Pas 1 et 2** : déposés au pousseur, lot `keyed-links-1`, branche `codeparsers/keyed-links`, tête
  e8535aaa1f8399dbf4f5bb61944ac491b361dc88, `VUE=luciepc:codeparsers/21-11-2026-liens-a-cle`, attestation
  `CHECK_FEATURES VERT 8/8`. Le lot retire le filtre `derniers` et ajoute les témoins moteur des liens parallèles.
- **Pas 3** : en cours, branche `codeparsers/keyed-links-2`, en pile sur `keyed-links-1`. Ses commits, dans l'ordre :
  - 72ecf5d595 : `RelationDef.key` ;
  - 8fb6c7b208 : la page 02 ;
  - 06f4259ba8 : les témoins lib, vus rouges ;
  - d97c9556c8 : le comportement (IR, dialectes, drain, annulation, refus). Les témoins lib sont **verts**
    (`a_keyed_relation_keeps_every_link_by_batches_and_by_copy`, `a_relation_that_had_a_key_cannot_lose_it`) ;
  - ec1b18f482 : le témoin Postgres `keyed_links_hold_on_postgres_and_an_existing_relation_takes_its_key`
    (e2e_postgres). **Son rouge n'est pas encore joué** : la passe a été prise dans la boucle du confineur, puis
    arrêtée ;
  - 2db9538fc8 : le correctif Postgres, la clé primaire déplacée par un bloc DO et le refus d'une clé NULL.
    **Commité avant son rouge**, à cause de la pause.

## Ce qui tourne à luciepc

Rien. Ma passe Postgres (756529 et ses quatre descendants) est arrêtée.

Incident de 15 h 10 : un `kill` sur une liste fausse. c7 l'a relayé, et la leçon est dans la mémoire de Claude Code,
`arreter-par-pstree-jamais-par-liste-calculee`.

## Worktrees et copies que je tiens

| poste | chemin | état |
|---|---|---|
| tablette | `~/git_workspaces/codeparsers/21-11-2026-liens-a-cle` | branche `codeparsers/keyed-links-2` |
| luciepc | `~/git_workspaces/codeparsers/21-11-2026-liens-a-cle` | détaché sur ec1b18f482 ; à avancer sur 2db9538fc8 |
| luciepc | `~/.cache/rag3weaver-build/target-liens-a-cle-e2e` et `-cf` | targets du lot |
| tablette | `~/git_workspaces/codeparsers/20-11-2026-occurrences` | prototype de mesure, jamais déposé (`codeparsers/occurrences-cost-probe`) ; à retirer |
| tablette | `~/git_workspaces/codeparsers/20-11-2026-occurrences-lot` | détaché, pour la sonde ; à retirer |
| tablette | `~/git_workspaces/codeparsers/21-11-2026-docs-occurrences` | page 01, atterrie (PR 344) puis redéposée avec son complément ; à retirer à l'atterrissage |
| tablette | `~/.cache/rag3weaver-build/mesures/occurrences-{sans,avec}-20261121-*` | reflinks gardés 48 h (jusqu'au 23 nov. 7 h) |

## Ce qui attend un dépôt

- `keyed-links-1` est en file.
- Le complément de la page 01 (`docs-occurrences-cost`, d0280c04bf) est en file, ou a atterri.
- `keyed-links-2` attend :
  - le rouge puis le vert du témoin Postgres ;
  - les suites e2e_copy_liens, e2e_usages, e2e_code_sync et e2e_postgres ;
  - l'attestation ;
  - le dépôt en pile sur `keyed-links-1`, ou sur master si celui-ci a atterri.

## Prochaine action, après l'annonce de c7 (`confiner-poste-classes` atterri)

1. À luciepc :
   - avancer le worktree sur ec1b18f482 et jouer le rouge Postgres, avec
     `bash ~/.cache/rag3weaver-build/liens-a-cle-pg.sh pg-rouge keyed_links_hold` sous
     `poste cpu -- timeout 3600`, lancé par `setsid` ;
   - puis le vert sur 2db9538fc8.
2. Une partie du pas 3 n'est pas faite : faire déclarer `key` par la relation des occurrences elle-même. C'est le lot
   des occurrences proprement dit, avec la relation `OCCURS`, `usages` qui rend la colonne, la montée des règles en
   16, sa ligne d'empreinte et la ligne de nouveautés.
3. Avant le lot 3, comprendre l'écart de `MENTIONS` (+12) que le prototype faisait apparaître.
