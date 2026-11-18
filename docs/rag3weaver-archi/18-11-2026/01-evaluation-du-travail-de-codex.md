# Évaluation du travail de Codex dans le périmètre d'archi — le mercredi 18 novembre 2026

Le périmètre d'archi couvre :
- la boucle de l'équipier (`daemon_host/teammate.rs`) ;
- l'invite et les outils d'un tour ;
- les tâches et le socle d'agent de code ;
- le pont navigateur ;
- le bus entre sessions.

La fenêtre évaluée va du 16 novembre, 11 h, au 17 novembre, 21 h 08 (arrêt sur quota). Le point de
départ est ma passation du 16 (`16-11-2026/01-passation-pour-codex.md`), avec la branche
`pas-va-au-bout-2` (`5e25a110d`, verte sur `6b672afa6`) et douze tickets. Tout ce qui suit a été lu
dans git, sans rien rejouer.

## 1. Commits et branches

### Sur master

**Aucun commit de Codex ne touche le périmètre d'archi.** Les 76 commits arrivés depuis `927941a67`
touchent d'autres rôles : catalogue, ordre des ex æquo, source Vertex renouvelée (`936f13daa`,
`4b3225669`), gardes de poste, docs. La source Vertex sert à l'équipier (e2e_equipier passe par
elle). Elle n'est pas évaluée ici, mais elle entre dans le rejeu de `pas-va-au-bout`.

### Branches de la fenêtre (16 et 17 novembre)

Toutes ces branches sont des empilements, chacune contenant la précédente. Aucune ne contient
`5e25a110d` par ascendance : Codex a **re-posé mes deux commits** (`e0f084f4e`/`3ae1e7414`, puis
`2592f48e4`/`1b3d99076`). Le code est identique au mien (diff vide sur `daemon_host`).

| Branche (tête) | Ce qu'elle apporte en propre | Verdict |
|---|---|---|
| `cx-archi-pas-va-au-bout-r1` (`e30e453b5`) | Mes deux commits reposés sur `927941a67`, sans conflit, plus deux pages de reprise. Rien n'est rejoué sur cette tête. | **utile mais à requalifier** : c'est un rebase, à refaire sur le master courant et à rejouer |
| `cx-archi-pas-va-au-bout-r2` (`234b94aa6`) | La même chose sur `68590270e` (épingle de la lib 59ac), plus la page 04 : **e2e_equipier rouge sous Vertex**. L'équipier demande l'accord `rm Cargo.lock`, personne ne répond, et le délai de 900 s du test tombe. Ticket ouvert : `le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur`. | **utile, à garder** (le constat et le ticket) : c'est le seul vrai résultat de la fenêtre |
| `cx-archi-doc-supervisor-r1` … `cx-archi-doc-green-pilot-r1` (`ea67054a1` → `ae8f311b1`) | Un témoin déterministe : un faux modèle demande `rm Cargo.lock`, le superviseur refuse, l'équipier lit, écrit `README.md` et finit sans seconde demande ni accord. Il est en anglais, a été **vu rouge** (page 11, la fixture ne répondait jamais) puis **vert** (page 14, six témoins ciblés). S'y ajoute `tests/common/documentation_fixture.rs` (232 lignes), un superviseur de fixture qui ne refuse que `rm Cargo.lock`. | **utile mais à requalifier** : le témoin unitaire est bon ; la fixture est trop étroite (ci-dessous) |
| `cx-archi-doc-supervisor-fix-r1`, `cx-archi-doc-bac-*`, `cx-archi-pas-va-au-bout-union-r1/r2`, `-full-pilot-r1` | e2e_equipier repasse par cette fixture et exige que **toute** demande d'accord soit `rm Cargo.lock`. Le « Bac privé » : `connect_declared_sandbox::<SandboxConfig>` injecte une config de bac à sable dans le manifeste des deux fixtures, avec un témoin pour chaque. Union sur `99fef92ce` à la tête `f1e8a8d50`. | **utile mais à requalifier** : **e2e_equipier modifié n'a jamais été joué** (pages 14 et 31 : « NON EXÉCUTÉ ») ; une suite non jouée est rouge |
| `cx-archi-global-cfg-red-r1` (`d53d98695`), `cx-archi-pas-va-au-bout-global-pilot-r2` (`282937a17`) | 82 commits et 133 fichiers de docs sur le « pilote global D6 » : Lifecycle global, lecteur de publications, Broker, manifestes, coupons et GO, P/Q/S/T. S'y ajoutent `scripts/execution_entry.py` (« validateur pur », 51 tests Python verts) et `scripts/execution_entry_runner.py` (« MODELED / FIXTURE_ONLY », dernier rouge non corrigé). | **inutile ou nuisible** : rien n'y sert le produit ni la campagne D6 ; c'est un harnais de preuves autour du harnais. À ne pas poser sur master ; la branche reste, rien n'est effacé |

### Branches d'avant la fenêtre (11 au 13 novembre)

Les branches `cx-archi-chat-erreurs`, `-ide-par-le-flux*`, `-ui-contextuel*`, `-symboles*`, `-grep-*`
et `-accord-exterieur-*` datent d'une ronde Codex antérieure. Leur contenu est déjà sur master, sauf
**un ou deux commits sur sept d'entre elles**, presque tous des docs de reprise. Le seul code non
posé est un témoin (`test(symboles)` sur `cx-archi-symboles-1`, `test(accords)` sur
`-accord-exterieur-baseline-3`) et le grep enrichi en symboles (`cx-archi-grep-symboles-1`, deux
commits). Ces restes sont à relire par leurs rôles actuels (recherche, ED), hors de cette évaluation.

### Le respect des règles

| Règle | Constat |
|---|---|
| Anglais du code neuf | Tenu : noms de tests, fixture, messages d'assertion. |
| Rouge d'abord | Tenu pour le témoin du refus (page 11 puis page 14). |
| Suites exigées par chemins | **Pas tenu** : e2e_equipier est modifié mais pas joué ; la lib entière et les hooks restent « NON JOUÉS » (page 04). |
| Lib épinglée | Tenu : 59ac partout. |
| Identité gmail, sans attribution | Tenu. |
| Commits | Les messages n'ont pas de préfixe de type ; ce n'est pas une faute. |

## 2. Verdict d'ensemble

1. **Ce qui avance vraiment** : le rouge e2e_equipier sous Vertex, avec le nouveau moteur. Une demande d'accord sans superviseur bloque le témoin. C'est le premier obstacle au landing de `pas-va-au-bout`.
2. **Ce qui avance aussi** : un témoin unitaire du refus puis de la fin, propre et vu rouge puis vert, à reprendre tel quel.
3. **Ce qui est du papier** : tout le « pilote global » et le protocole d'entrée, soit environ 30 000 lignes de docs, deux scripts Python et des preuves hachées sur deux hôtes. Rien n'atteint le produit ni la campagne D6.
4. **Aucun des douze tickets du 16 n'est traité** (S1 à S5, J1, garde B, position, index, delegate, mode pair, affichage). `pas-va-au-bout` n'a pas atterri.
5. **Ce qui gêne** : une fixture e2e qui refuse seulement `rm Cargo.lock` et rougit sur toute autre demande d'accord. Elle est fragile face à un vrai modèle. S'y ajoute un jargon (APP, GO, END, RO, coupon, Bac, P/Q/S/T) qu'aucune autre page ne définit.

## 3. Ce que ça change à la passation du 16

| Entrée du 16 | État |
|---|---|
| 1. Faire atterrir `pas-va-au-bout` | **Commencée par Codex**, pas finie. Ses reposes (`cx-archi-pas-va-au-bout-r1` `e30e453b5`, union `f1e8a8d50`) ne sont pas qualifiées. Archi la reprend depuis sa propre tête `5e25a110d` : rebase sur master, rejeu complet. Le rouge e2e_equipier (ticket de Codex) se traite dans le même mouvement, par un superviseur de test qui **refuse toute demande d'accord**, quelle que soit la commande. Le témoin unitaire du refus (`a_documentation_supervisor_denies_lock_deletion_and_the_teammate_finishes_the_original_request`) se reprend en lot séparé, avec la fixture généralisée. |
| 2 à 13 (S1, S2, S3, garde B consigne, S4, S5, J1, position du worktree, attente d'index, delegate, mode pair, affichage tty) | **Intactes.** |
| Tickets neufs de Codex | `le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur` : **à garder**, il bloque l'entrée 1. `le-nom-agent-accepte-un-blanc-interieur` : c'est la tâche d'écriture de la campagne D6 (« Om bre »), pas un défaut constaté. Il est à requalifier en tâche D6, et la règle de nommage revient à Lucie et ED. `docs/orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md` : il appartient au pilote global, donc **à fermer** avec lui. |

Aucune de ces branches ne se pose sur master. Ce qui doit être retiré n'y est pas : il suffit de
ne pas le fusionner. Les tickets de Codex ne sont, eux aussi, que sur ses branches. Archi reporte
sur master ceux qu'elle garde, par un commit de docs.

## 4. Lisibilité de la passation de Codex

**Non, elle n'est pas exploitable sans lui.** `17-11-2026/99-passation-pour-claude.md` parle de
coupons, de GO, d'END, de RO, de « suite31 », de SHA de scripts et d'enveloppes temporelles. Elle ne
dit nulle part ce que la branche apporte au produit, ni ce qui reste à faire pour `pas-va-au-bout`.
Ses « trois prochains pas » corrigent le runner Python, qui n'a pas d'usage.

Les pages 03 à 33 du 16 sont écrites en mots collés (« rouge11 », « masterbef », « deuxSO59ac ») et
ne se lisent qu'avec leur auteur. Seules quatre sont utiles :
- la page 04 (le rouge Vertex) ;
- son ticket ;
- la page 11 (le rouge du témoin unitaire) ;
- la page 14 (son vert).

Cette évaluation en garde l'essentiel.
