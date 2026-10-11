# Rapport de session — mémoire longue, 11 octobre 2026 (matin, midi et après-midi)

*Pour Lucie et pour l'orchestration. Ce qui est sur master, ce qui reste, ce qui attend.*

## Sur master

**La page C, avec l'arbre principal** (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
Mes parties : le client mince (§3), quand le démon n'est pas là (§4), ma part des
marches (§7), puis l'alignement sur tes treize décisions. Deux faits de la carte
portent ma partie : le protocole client ↔ backend existe déjà (le chat le parle, il
est donc déjà un client mince), et `mcp.rs` a déjà sa couture (`trait Hote`) — le
client de C est un autre `impl Hote`. Et un défaut que C rend atteignable, nommé et
réglé par ta décision 12 : la bascule de cellule du catalogue entier, devenue une
course dès que plusieurs clients partagent un backend.

**Le lien générique, quatre marches sur cinq** (`01-un-lien-entre-deux-identites-declarees.md`) :

| | commit | ce que c'est |
|---|---|---|
| témoin | `49142d58f` | la garde de `RelationBatchNode` ne protégeait aucune garantie déclarée — établi champ à champ |
| 1 | `6d212551e`, `cde65dc9d` | `RelationBatchNode` devient le nœud de lien général ; existence vérifiée au catalogue (base ou file) ; propriétés déclarées et typées ; `add_ref` ne prétend plus avoir lié une fiche absente |
| 2 | `13c6d950f` | `from_states` / `to_states`, une politique d'écriture du backend, appliquée par les deux nœuds qui lient |
| 3 | `a6c2a61dd` | `link.mmd` ; `anchor` et `supersede_link` dans le gabarit `memory` ; la réaction ancre par l'outil |
| 4 | `ad51a6630` | le huitième refus, lu sur la marque « relation lue / écrite » des paramètres |

**Ta mémoire branchée, rafraîchie** (`9734e75e0`) : neuf outils, et la chaîne entière
éprouvée par MCP sur une copie de ta base — ancrer, changer le sujet, la mémoire passe
« à revoir » d'elle-même. Ta base n'est pas touchée, l'ancien est gardé.

## L'après-midi, sur master

| commit | ce que c'est |
|---|---|
| `59718e806` | **SIGTERM vaut la fermeture du tube** dans `servir_mcp` : l'appel en cours finit, les indexations s'arrêtent au prochain paquet, la base se ferme ; témoin `e2e_mcp_sigterm` (sortie 0, la base se rouvre) |
| `2d0687a71` | **le rapport des liens dit la vérité sur `atomic`** : une instruction, atomique par le moteur ; plusieurs instructions de liens seuls, enveloppées d'une transaction ; une fermeture qui porte des créations, `atomic: false` et sa raison. Rouge réel par faute injectée (deux variables, jamais active sans les deux) |
| `d25cad18b` | **le client mince de la forme C** : `mcp --daemon <adresse>` parle à l'hôte du démon par un `impl Hote` (`DaemonClient`), sans rien posséder ; une session perdue se rouvre une fois ; `--demon` est refusé en nommant `--daemon`. `e2e_mcp_stdio` rejoué entier par l'hôte, et deux sessions MCP partagent un backend |
| `fa04ccd3c` | **marche 1 bis du lien** : `Catalog::link` refuse un bout absent en le nommant, quand le lien se pose maintenant ; les liens différés se comptent au drain comme avant. Le ticket du bout absent est fermé |
| `b1b673d2b` | la page du lien, §3 bis : **la cellule des bouts** — avec l'identité par cellule (variante a), un lien résout ses bouts dans la cellule de la session ; un lien entre cellules se déclare. À faire avec la migration de l'arbre principal, pas avant |
| `442791fde` … `d03fbfe31` | **la page de l'enveloppe d'un outil d'écriture**, avec l'arbre principal (`03-l-enveloppe-d-un-outil-d-ecriture.md`) : le recensement, ce que le paquet garantit, les formes A à D et leur coût |
| `cd919b053` | **forme A de l'enveloppe** : chaque outil d'écriture dit `atomic` et sa raison ; l'échec d'un put dit si la ligne existe (dans la cellule de l'appel) et ce qui est prêt d'elle ; `add_ref` et `create_ref_type` lisent enfin leur rapport (ticket fermé). Témoin `e2e_outils_disent_ce_qui_reste` (trois tests, faute injectée à l'insertion) |
| `e161365d5` | **le banc `pg_trgm` tranché** (voir plus bas) |

**`e2e_postgres`, première exécution réelle** (conteneur de Lucie, port 5433) : 19 passés
sur 20. Le rouge, `ou_vit_la_frontiere_entre_le_vrai_et_le_bruit`, est identique au
commit d'avant l'échappement : il n'est pas dû au travail de F. Mesuré dans le
conteneur, `word_similarity('sauterau baroqe', …)` vaut 0,5789, au-dessus du plancher
0,3 : l'attente du banc est juste. La cause est le pool `deadpool` : le plancher est
posé par un `SET` joué une seule fois, donc sur une seule session. Confié à F
(`SET LOCAL` dans la transaction de la requête, sur la connexion épinglée).

## Ce qui reste

- **Forme B de l'enveloppe** (envelopper un put, invalider les seuls caches de sa
  ligne, rouvrir la poignée FTS à la génération marquée ; deux passes) : attend les
  deux vérifications de l'arbre principal et le mot de Lucie sur la §6.
- **La cellule des bouts d'un lien** (`to_cell` / `from_cell`, `entity_uuid_in`) :
  attend la migration de l'arbre principal.
- **La marche 5 du lien** (les gabarits qui en profitent) : pas demandée.

## Tickets

Fermés aujourd'hui par cette session : le nœud qui ne relierait pas deux entités
gérées ; l'outil d'ancrage absent ; le huitième refus ; un lien vers un bout absent
qui rend `Ok` ; `add_ref` / `create_ref_type` qui jettent leur rapport.
Ouverts par elle, toujours ouverts : le canal d'échecs d'une réaction sans lecteur ;
un `put` sans le champ de cycle de vie ; une réaction qui écrit dans la cellule
basculée ; le banc `pg_trgm` (cause trouvée, à F).

## Ce que la session a appris

- **Une prémisse se mesure avant d'être promise.** J'ai écrit « six lignes » pour
  l'ancre ; l'orchestration a dit oui sur ma parole ; un essai de cinq minutes a
  montré que le nœud refusait. Le bon ordre est l'essai, puis la promesse.
- **Une garde sans témoin se prouve avant de s'enlever, et la preuve suffit à la
  trancher.** Le témoin champ à champ a rendu la décision évidente pour tous les
  relecteurs.
- **Les relectures ont attrapé trois vrais défauts** que j'aurais livrés : drainer
  « ses seuls liens » (un lien perdu sans erreur), la paire donnée deux fois
  (perdue sans un mot), et l'affirmation fausse sur les transactions. C'est le prix
  d'une page avant le code, et il est bas.
- **Un témoin écrit avant le code attrape le code.** La forme A disait « the record
  exists » pour une ligne absente : `get` rend `Ok(None)`, j'avais testé `is_ok()`.
  Le témoin l'a vu à sa première passe.
- **Un rouge s'attribue avant de s'expliquer.** Rejouer le banc au commit d'avant
  l'échappement a mis F hors de cause en une passe. Ensuite, une requête SQL
  directe a tranché ce que le banc ne séparait pas, sans une heure de carte.
