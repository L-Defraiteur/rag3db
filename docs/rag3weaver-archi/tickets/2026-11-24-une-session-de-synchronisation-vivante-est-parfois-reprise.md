# Une session de synchronisation vivante est parfois reprise, sous la charge de la lib entière

- **État** : ouvert, le 24 novembre 2026
- **Famille** : produit
- **Gravité** : moyenne : un témoin rougit une fois dans la lib entière, ce qui coûte une passe à qui la joue ; et si la
  cause est dans le produit, deux écrivains d'un même catalogue peuvent se croire seuls
- **Atteignable en service** : à voir (selon la cause : une lecture de l'identité du processus qui échoue sous charge
  serait atteignable)
- **Touche rag3weaver** : oui (`src/catalog/synchronisation`, `session_orpheline_tests.rs`, l'identité du processus
  propriétaire d'une session)
- **Pour** : qui tient `catalog::synchronisation` (à attribuer)
- **Signalé par** : archi, passe du lot `write-findings-first` à luciepc, le 24 novembre 2026

## Ce qui s'est passé

`catalog::synchronisation::session_orpheline_tests::a_session_whose_owner_runs_is_refused` a rougi une fois dans la lib
entière (3 140 verts, 1 rouge), à `src/catalog/session_orpheline_tests.rs:70` :

> called `Result::unwrap_err()` on an `Ok` value: SnapshotSession { session: "1795493267817-491050c39807",
> opened_at: 1795493267817, replaced: Some("1795493267816-5bb046a37a2d"), suspended_at: None,
> owner: Some("2322929:183914865") }

Le second `begin_snapshot` a **repris** (`replaced`) la session que le premier venait d'ouvrir, une milliseconde plus
tôt, alors que son propriétaire est le même processus, bien vivant. Le témoin attend un refus (« takeover »).

Rejoué seul (`session_orpheline_tests::`), trois fois de suite : 3/3 vert.

## L'hypothèse, non vérifiée

La reprise juge le propriétaire mort quand son identité (`pid:début`) ne se retrouve pas. Sous la charge de la lib
entière, la lecture de l'heure de départ du processus (`/proc/<pid>/stat`) pourrait échouer ou rendre autre chose, et le
propriétaire vivant passe alors pour mort. Une autre piste : un voisin de la lib qui partage un registre des sessions
vivantes (`live_sessions`) ou une horloge.

## Ce qu'on attend

Qu'une erreur de lecture de l'identité ne vaille jamais « mort » (dans le doute, le propriétaire est vivant et la
reprise est refusée) ; et un témoin qui force cette erreur. Le rejeu du témoin en boucle sous charge (`cargo test --lib`
entier, dix fois) dira si l'hypothèse tient.
