# Rapport de la session mémoire longue — 19 octobre 2026 (nuit)

Session rag3db-96. Le fil de la veille est dans `../18-octobre-2026-memoire-longue/` (pages 01
à 04) et dans le fil de l'orchestration du 18.

## La pièce 1 : « le catalogue dit tout »

**Ce que c'est.** Chaque chemin d'écriture du catalogue publie sur le sujet `catalog`, en JSON
structuré, ce qu'il a **réellement** écrit :

- `EntitiesChanged`, avec `change` à `created`, `updated`, `transitioned` (avec `from_state` et
  `to_state`), `ingested` ou `removed` ;
- `EntityDeleted` ;
- `RelationsChanged`, avec `added` et `removed` en état net, `count`, et `truncated` au-delà de
  500 paires par liste ;
- sur chaque événement, `at_ms` et `run`.

Sous une transaction, l'annonce part au commit et jamais au rollback. C'est la base du miroir
des fichiers composés (page 04, lot M2) et de toute réaction qui suit une écriture.

**Où elle en est.**

- 5dac25654 (branche `catalogue-dit-tout-2`) : la mécanique. Relue par l'arbre principal (73),
  qui a validé ses trois exigences.
- Second lot, non poussé, worktree `wt-p1r-96` rebasé sur master 863ba4a83 :
  - les deux corrections de 73 :
    - le journal n'est plus prêté à la vue de lecture ;
    - une écriture qui change le contenu **et** l'état émet `updated` puis `transitioned` ;
  - les quatre réserves de c7 :
    1. **La déliaison s'annonce.**
       - Le `Detach` de `code_sync` relit ses paires juste avant (`edges_to_detach`, Cypher
         et Postgres).
       - La ré-indexation d'un fichier ouvre une fenêtre de retenue, et les `RelationsChanged`
         retenus se fondent en état net : un fichier inchangé ne publie aucun lien.
       - Nouveau verbe public `Catalog::unlink` / `unlink_jusqu_a`, par la même file que
         `link`. Un lien et une déliaison d'une même paire s'annulent dans la file.
    2. **`added` ne compte que les paires neuves.**
       - COPY : les paires réellement écrites.
       - Chemin par lots : la différence des paires lues avant et après chaque tranche.
       - Postgres reçoit `existing_links` et `count_links`.
    3. **`run` est le run qui écrit.** Le runtime le pose sur le fil de chaque nœud
       (`events::pendant_le_run`, aux trois sites d'exécution), et `publier` l'appose.
    4. **Le coût est mesuré** sur l'arbre de code-lourd, à luciepc : voir plus bas.
- Trouvé en route : le graphe de `poser_la_donnee` n'avait pas le service `config`. `link` et
  `unlink` y passaient sans bouts étiquetés, et jamais par COPY. Il l'a maintenant, comme le
  drain.
- Le miroir régénère tous ses fichiers sur une annonce tronquée d'une relation qu'il lit.
- Deux tickets ouverts :
  - « Des écritures ne passent pas par le journal » (le PlanNode, l'aside,
    `mark_absent_since`, `drain_resume`) ;
  - « L'annulation d'un lot de liens retire aussi les paires qui existaient avant lui ».

**Témoins.** `e2e_catalogue_dit_tout` 14/14 vert, sur la lib `courant` (cf71c929d), sans
MOTEUR_ANCIEN, à luciepc.

**La mesure** (réserve 4).

- Méthode :
  - à luciepc, `e2e_mesure_sync_source` sous `poste mesure` et `RAG3WEAVER_INGEST_PROFILE=1` ;
  - « sans » = master 863ba4a83 plus la seule mesure ; « avec » = le lot entier ;
  - corpus : l'arbre de code-lourd, `rag3db-lourd` a72acfeef, 7 278 fichiers, 82 888 scopes,
    270 119 relations, plein texte seul ; l'arbre est inchangé avant et après.
- **Indexation entière** : pas de surcoût mesurable.

  | | passe 1 | passe 2 | passe 3 |
  |---|---|---|---|
  | sans la pièce | 277,1 s | 287,2 s | 289,1 s |
  | avec la pièce | 283,1 s | 282,2 s | 274,4 s |

- **Ré-indexation de 200 fichiers inchangés** (`reingest_files`, paquets de 64) :
  - première version : +56 % (19,7 s et 18,7 s sans ; 30,8 s et 29,3 s avec). La relecture
    des paires avant chaque `Detach` refaisait le balayage filtré des scopes, par fichier et
    par relation ;
  - corrigée : la déliaison rend ses paires dans la même instruction
    (`detach_edges_returning` : `DELETE r RETURN`, `RETURNING` en Postgres), et le
    `count_links` par groupe du chemin par lots, qui comptait toute la relation, est supprimé ;
  - après correction : 22,75 s contre 21,0 s, soit +8 %, sur « oublier les arêtes du fichier »
    (11,2 s contre 7,9 s).
- **Événements** : rien à regrouper, ils le sont déjà.
  - L'indexation entière publie 11 `RelationsChanged`, annonçant 1 046 379 paires, et 420
    `ingested`.
  - La ré-indexation de fichiers inchangés n'annonce aucun lien.

**Poussée : 60348d75a** (master, 19 octobre 2026, 8 h 30).

- Le rejeu du tour de push, sur master c2ef8a941 et la lib `courant` 4f9274a38, sous
  `poste push` à luciepc :
  - check_features 5/5 (la crate vérifiée dans les cinq jeux, après un `cargo clean -p`) ;
  - la lib entière 1 744/1 744 ;
  - 16 suites vertes : catalogue_dit_tout 18, fichier_compose, reaction_montee, copy_liens,
    copy_refuse_rouvre, liens, garde_des_liens, code, code_sync, impact, postgres, estimate,
    arret_propre, tx_par_paquet_arret, lectures.
- La batterie complète hors carte locale avait été jouée sur 55115bb20. Ses seuls rouges :
  - les services d'embarquement de luciepc, que j'avais appelés sur les ports des tunnels d'ici
    (7979 à 7981) au lieu de 7878 à 7880 ;
  - `e2e_estimate`, rouge de master, que G a corrigé en 5d60ba6d8.
- Trouvé par les tours de push, et corrigé avant le push : un COPY de liens sans propriété, vers
  une relation qui en déclare, ne nommait aucune colonne, et le moteur attendait toutes celles
  de la table.
  - Nommer toutes les colonnes au mot NULL a cassé les colonnes entières : le moteur ne lit pas
    ce mot dans un INT64 (ticket moteur).
  - La forme gardée : une propriété texte au mot NULL, sinon le chemin par lots.

## Ce qui attend

- **M3** (la reconstruction et sa preuve de parité), après la pièce 1.
- **Le PlanNode et `cited_by`** quand l'IR de 73 est sur master.
- **La migration de la fiche de Lucie**, quand son hôte tourne sur une lib saine.
- **La tentative 3 rejouée en gemini-3.8-flash**, au quota.
- **En fin de session, à effacer :**
  - les worktrees `wt-p1-96`, `wt-p1r-96`, `wt-m2-96`, `wt-cite-96`, `wt-rel-96`, `wt-banc-96` ;
  - à luciepc, `rag3db-p1-96`, `rag3db-p1r-96`, `rag3db-p1sans-96`, `rag3db-m1-96`,
    `rag3db-seed96` ;
  - les targets `target-96*`.
