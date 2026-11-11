# La fermeture FTS en erreur laisse le handle actif

- **État** : corrigé par le passage aux paquets Lucivy 4.3.3 ; témoin rouge/vert public vérifié le 11 novembre 2026, passe d'intégration en cours au moment de cette rédaction.
- **Famille** : produit
- **Gravité** : une fermeture en erreur peut laisser les acteurs travailler après la fin supposée du stockage ; une nouvelle écriture reste acceptée.
- **Atteignable en service** : oui, une erreur de persistance peut faire sortir close avant l'arrêt des pools dans la dépendance 4.3.2.
- **Touche rag3weaver** : oui, minimums et Cargo.lock, test de son API FTS ; correction du mécanisme publiée en amont.

## Reproduction et preuve publique

Le témoin `fts_handle::repro_drop::une_fermeture_fts_en_erreur_rend_le_handle_inerte_et_le_processus_sort` crée un index de deux shards dans son propre dossier temporaire, indexe un document, commite puis supprime ce stockage. Il exige l'erreur de close, le refus d'indexer ensuite avec une erreur « closed », la destruction du handle et la sortie normale du processus enfant. Le parent borne cette sortie à 20 s.

Avant le changement des dépendances, source `820f29840`, 4.3.2 : 0 réussi/1 échoué/0 ignoré en 0,05 s. Close rend une erreur de sauvegarde de shard_0, puis `index_apres=Ok(())` ; le témoin échoue sur l'exigence d'inertie. Journal luciepc `~/.cache/rag3weaver-build/journaux-cx-recherche-lucivy-433-11nov/rouge-432.log`, SHA256 `73d2dd9e011a89b6084f39a7e3a5d124bfa73d2108af353833cefa04e0d8edc1`.

Après le changement, source `c1463b9ee`, même test inchangé : 1 réussi/0 échoué/0 ignoré en 0,05 s. La même erreur de close est conservée, `index_apres=Err("handle is closed")`, le handle est détruit et l'enfant termine avec succès. Journal `vert-433/close-erreur.log` sous le même dossier.

## Portée du correctif

Les minimums `lucivy-core`, `ld-lucivy` et `sparse-vector` passent à 4.3.3. Le lock ne change que ces trois paquets, `luciole` et `lucistore` (versions et checksums), tous depuis le registre, sans patch local ni mise à jour globale.

La preuve interne que les pools ne gardent plus leurs shards est le témoin amont `a_failed_close_still_stops_its_pools`, joué par la session lucivy sur le tag `663975adf11fc4a9d6cadf90e70ee68f6c9db075` (voir `docs/lucivy/11-11-2026/01-rapport-de-session.md`). Le témoin public de rag3weaver la complète ; il ne prétend pas inspecter les Arc privés ni reproduire chaque SIGSEGV historique. Format inchangé : aucun scénario de compatibilité, migration ou banc ajouté. Recherche et réouverture sont éprouvées par les tests existants de la passe normale.
