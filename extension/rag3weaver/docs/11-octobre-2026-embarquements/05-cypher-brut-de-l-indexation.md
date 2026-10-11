# Le Cypher brut de l'indexation du code : relevé et lots

*11 octobre 2026, session embarquements. Ticket
`docs/tickets/2026-10-11-l-indexation-du-code-envoie-du-cypher-brut.md`.*

## Le relevé

Sept endroits envoient encore du texte Cypher hors du dialecte, sur le chemin
d'`ingest_code` et de la synchronisation. Les outils du gabarit (usages, impact,
callees, links) n'en ont plus aucun.

| # | où | ce qu'il fait | remplacement |
|---|---|---|---|
| 1 | `code.rs:2349` (`parents_et_fichiers`) | parent, fichier et genre des scopes définisseurs, par uuids | `Select::by_uuids("Scope", [parent_name, file_path, scope_type])` : exact |
| 2 | `code.rs:2396` (`resoudre_les_types_differes`) | source et dépôt des scopes mentionneurs, par uuids | `Select::by_uuids("Scope", [source, repo])` : exact |
| 3 | `code.rs:2509` (`charger_les_types_ecrits`) | les scopes de noms donnés, dans une source et un dépôt | **manque** : un « champ dans une liste » sur un autre champ que `_uuid`, et un ET |
| 4 | `code_sync.rs:265` (`oublier_les_aretes_du_fichier`, ×11 relations) | supprime les arêtes sortantes des scopes d'un fichier | **manque** : une suppression d'arêtes par une condition sur le nœud de départ, et un ET (`source` et `file_path`) |
| 5 | `code_sync.rs:270` (même fonction) | supprime les `CONSUMED_BY` qui arrivent sur ces scopes | même manque, du côté de l'arrivée |
| 6 | `code_sync.rs:900` | lit `copy_journal_fallbacks` (profil d'ingestion seulement) | **manque** : lire un réglage du moteur |
| 7 | `code_sync.rs:1036` | `CALL force_checkpoint_on_copy=true` avant le paquet | **manque** : un réglage du moteur pour une session de COPY |

En plus :
- **`code_tools.rs:1034`** (`voisins_des_scopes`) : un saut sans table d'arrivée qui lit
  des champs. PostgreSQL le refuse (« Hop (champ sans table d'arrivée) ») ; la table
  d'arrivée est `Scope`.
- **`catalog.rs:843`**, hors du chemin, à l'ouverture : un `MATCH (b:_index_blobs)
  RETURN b._key LIMIT 1` en texte brut ; c'est un `Select::all` borné à 1.

## Les lots

1. **Ce qui se traduit exactement.** Les endroits 1 et 2, le saut de `code_tools.rs` et
   l'ouverture de `catalog.rs`. Aucune forme nouvelle.
2. **Deux prédicats de plus dans l'IR** : `Predicate::AllOf` (le ET, qui manque à côté
   d'`AnyOf`) et `Predicate::In { field, param }` (un champ dans une liste). Ils couvrent
   l'endroit 3 exactement, en une instruction, sur les deux dialectes.
3. **Défaire les arêtes d'un ensemble de nœuds.** Les endroits 4 et 5 : onze relations,
   une condition sur le nœud. Deux voies :
   - une forme `Write::Detach { relation, table, side, filter }` (une instruction par
     relation, comme aujourd'hui) ;
   - ou la composition `Select` → `Hop` → `Unlink`, sans forme nouvelle, en trois
     instructions par relation au lieu d'une.
4. **Les réglages du moteur** : les endroits 6 et 7. Ce sont des crochets du dialecte,
   pas des formes : `copy_session_setup()` rend les instructions à envoyer avant un
   paquet de COPY (rag3db : le `CALL` d'aujourd'hui ; PostgreSQL : rien) ; un réglage
   lu rend `None` hors rag3db.

## Les témoins

- **`e2e_code` rejoué à l'identique sur rag3db** : c'est un corpus vivant, donc mêmes
  comptes qu'avant la conversion, ou l'écart expliqué. Le relevé d'avant se prend avant
  le premier lot.
- **La mesure d'impact sur PostgreSQL**, comme témoin final : `src/daemon` indexé sur le
  conteneur (recette dans le ticket).
