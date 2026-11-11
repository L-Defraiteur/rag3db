# La fermeture FTS en erreur laisse le handle actif

- **État** : corrigé `c1463b9eeae78b997af5c86d2f93a7597dc240dc` ; même témoin rouge/vert et passe complète validés, tour de push accordé par orchestration.
- **Famille** : produit
- **Gravité** : une fermeture en erreur peut laisser les acteurs travailler après la fin supposée du stockage ; une nouvelle écriture reste acceptée.
- **Atteignable en service** : oui, une erreur de persistance peut faire sortir close avant l'arrêt des pools dans la dépendance 4.3.2.
- **Touche rag3weaver** : oui, minimums et Cargo.lock, test de son API FTS ; correction du mécanisme publiée en amont.
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../../recherche/11-11-2026/01-rapport-de-session.md).

## Reproduction et preuve publique

Le témoin `fts_handle::repro_drop::une_fermeture_fts_en_erreur_rend_le_handle_inerte_et_le_processus_sort` crée un index de deux shards dans son propre dossier temporaire, indexe un document, commite puis supprime ce stockage. Il exige l'erreur de close, le refus d'indexer ensuite avec une erreur « closed », la destruction du handle et la sortie normale du processus enfant. Le parent borne cette sortie à 20 s.

Avant le changement des dépendances, source `820f29840`, 4.3.2 : 0 réussi/1 échoué/0 ignoré en 0,05 s. Close rend une erreur de sauvegarde de shard_0, puis `index_apres=Ok(())` ; le témoin échoue sur l'exigence d'inertie. Journal luciepc `~/.cache/rag3weaver-build/journaux-cx-recherche-lucivy-433-11nov/rouge-432.log`, SHA256 `73d2dd9e011a89b6084f39a7e3a5d124bfa73d2108af353833cefa04e0d8edc1`.

Après le changement, source `c1463b9ee`, même test inchangé : 1 réussi/0 échoué/0 ignoré en 0,05 s. La même erreur de close est conservée, `index_apres=Err("handle is closed")`, le handle est détruit et l'enfant termine avec succès. Journal `vert-433/close-erreur.log` sous le même dossier.

## Portée du correctif

Les minimums `lucivy-core`, `ld-lucivy` et `sparse-vector` passent à 4.3.3. Le lock ne change que ces trois paquets, `luciole` et `lucistore` (versions et checksums), tous depuis le registre, sans patch local ni mise à jour globale.

La preuve interne que les pools ne gardent plus leurs shards est le témoin amont `a_failed_close_still_stops_its_pools`, joué par la session lucivy sur le tag `663975adf11fc4a9d6cadf90e70ee68f6c9db075` (voir `docs/lucivy/11-11-2026/01-rapport-de-session.md`). Le témoin public de rag3weaver la complète ; il ne prétend pas inspecter les Arc privés ni reproduire chaque SIGSEGV historique. Format inchangé : aucun scénario de compatibilité, migration ou banc ajouté. Recherche et réouverture sont éprouvées par les tests existants de la passe normale.

## Critère de fin vérifié

Porteur : recherche, lot `cx-recherche-lucivy-4-3-3`. Correctif et validation complète accomplis ; tour de push explicite reçu. Cette clôture et le déplacement dans closed sont dans le même commit de fin du lot validé, sans changement de source. Le push et ses trois contrôles suivent la procédure ; leur preuve et le nettoyage sont consignés dans `vert-433/pousse.log` après exécution.

## Passe complète verte

Source testée `c1463b9eeae78b997af5c86d2f93a7597dc240dc`, base `fcf23114c`. Neuf e2e, 121 tests réussis/0 échec/0 ignoré : search 40 (réouverture sparse existante comprise), dense après suppressions 2, cited_by 3, lectures 24, code 27, arrêt par paquet 18, vecteurs après rollback 3, merge puis COPY 1, ligne à ligne 3. Features 5/5 et preuve de compilation lue ; lib 2105/0/61 ignorés en 57,93 s, témoins FTS de recherche/réouverture existants nommés verts ; navigateur 36/0/0 en 132,34 s. Aucun saut nommé dans les étapes exigées. Pas de scénario de compatibilité ni nouveau banc, pas de juge Gemini inchangé rejoué.

Preuve luciepc : `~/.cache/rag3weaver-build/journaux-cx-recherche-lucivy-433-11nov/vert-433/verdict-final.log`, vérificateur `cx-recherche-lucivy433-verifier.py`. Ce dernier relit les comptes et noms utiles, les trois minimums, les cinq seuls paquets modifiés du lock (registre), les SHA sources et la preuve rouge. Attestation `attestation-artefacts.json` : 212 contenus courants dans les binaires lib/e2e/navigateur, chemins générés et `.d` du bon worktree. Générés copiés hors target. Lib native complète épinglée 29a7bc52a, sans dérogation ; sources inchangées à la fin. Fenêtre code réservée par orchestration jusqu'au verdict, tour reçu après lecture de ces preuves par orchestration.
