# Runner psql HOST — préparation source, aucun GO SQL

Autorisation source C7 b9139349d2, ticket FTS302 :
`docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md`.
Lot distinct `cx-optimiseur-psql-runner-source-20261117-r1`, depuis bundle5a9588.
Aucun fichier v1/Rust modifié. Les 15 bibliothèques sont qualifiées par
ELF/--list/SHA, pas par 15 mappings physiques observés. Programme psql et entrée
noyau chargeur distincts ; SSL/auth/dlopen/locale restent non qualifiés.

## Ordre rouge/source/CPU/relecture

La proposition initiale c6048fba9a a été refusée par C7 AVANT CPU : importer
un runner absent produit ERROR, sans juge métier. Aucun test exécuté.
Préparation corrective : baseline importable `RunnerUnavailable`, observe refuse
la capacité non implémentée sans appeler factory/clock/phase ni créer d'artefact.
Seul `WholeJob.test_success_publishes_new_fixture_kind_not_v1_grant` capture
CE type et appelle self.fail ; assertions positives nominales intactes. Toute
autre exception, notamment import/attribut/runtime, reste ERROR. Aucun catch
large, expectedFailure ou skip. Les 13 autres témoins restent NON JOUÉS.

Recette future proposée : sélectionner UNIQUEMENT ce nominal, Python système
`-B -m unittest test_runner.WholeJob.test_success_publishes_new_fixture_kind_not_v1_grant`
depuis son dossier, vrai POSTE32G/Common20/OS28TERM+KILL2=30MAX, une entrée sans
retry. Attendu : 1 test, 1 FAILURE explicite RunnerUnavailable, 0 ERROR ; rc1.
Aucun enfant RealCapture, psql/chargeur, Docker, HTTP, socket, Cargo, PG ou
fichier d'identifiants. Les futurs RealCapture lancent de vrais enfants Python,
et devront avoir leur propre recette/GO ; ce ne sont pas de faux processus.
Source/Common/recette hashés sous racine privée neuve avant toute entrée.
Les anciennes préparations red-source-01/red-01 ne seront pas exécutées.
R a regroupé ses réserves statiques : publication baseline non bloquée.
Publication source/pins puis RO et
GO C7 distinct requis pour ce nominal. Aucun GO acquis à ce stade.
Après vrai rouge seulement : implémentation et publication source complètes,
puis recette verte distincte relue avant son CPU. Aucun contact SQL autorisé.

## Frontières prévues

`capture(argv,env,stdin,deadline)` : enfant propre, stdin SQL constant seulement,
lecture non bloquante stdout/stderr bornée64KiB chacune ; TERM/KILL par pidfd
attribué, wait/reap/EOF avant END ORIGINAL, réserve de cleanup à l'intérieur.
Refus tardif reste rouge même si le nettoyage finit ensuite. Pas de boucle
réessayant la requête ou redémarrant une session après rouge.

`validate_profile` : profil fixture explicite et fermé, port15433 (jamais5433),
base g_pg_probe_<fixture_nonce12>, g_pg_observer non propriétaire. OID/starttime
externes de la future allocation fraîche. Aucune conversion vers le contrat v1.
`validate_session` : exactement deux lignes JSON sans doublon, GUC RO et rôles
mesurés dans LA session de l'identité/postcheck, PID identique entre ces lignes.
Identité originale SQL réutilisée sans renommer ses champs. Postcheck original
avec PID OU application_name, observateur distinct et nouvelle application:check.

`observe` : un whole5 commence avant factory/validation/pins/credentials et finit
après captures, parsing, reap, postcheck, hashes et dernier fsync/publication.
Phase/END externes revérifiés, jamais remis à zéro ; aucun défaut10s. Factory
native future dans le même budget, transport injecté seulement pour tests CPU.
Publication fixture type `psql-private-fixture-observation-1`, jamais admission
v1/grant. Réutiliser decode/shape/sql_shape/raw helpers et primitives exclusives
fsync de pg_receipt sans modifier le validateur/publisher v1 gelé. Une future
voie v1 devra utiliser ses pins exacts et son publisher existant, séparément ;
elle n'est pas autorisée par le profil fixture et ne doit pas être simulée.

`PsqlSession` réel à préparer : bundle exact scellé, HOME/TMP privés700,
PGPASSFILE600 exact du fournisseur fixture frais, jamais .env/.vault ni
credentials utilisateur, aucun secret dans argv/SQL/stdin/journaux. Environnement
reconstruit par liste blanche, `-X --no-password -qAt -v ON_ERROR_STOP=1 -f -`,
hôte/port/base/user explicites. PGOPTIONS bornés : read-only+statement_timeout+
lock_timeout+idle_in_transaction_session_timeout, connect_timeout1. BEGIN READ
ONLY / GUC + identité / ROLLBACK. Aucune allocation PG tant que client prêt et
GO contact distinct. Autorité Common/lease/allocation réelle demeure obligatoire
au lanceur ; un callback de test n'est pas une admission réelle.

## État

Tests et baseline source préparés avant CPU ; aucun rouge ni vert constaté.
Aucun runner fonctionnel livré. R autorise la publication statique de baseline,
sans GO CPU ni avis runtime. Avant futur vert : expiration au vrai fsync plutôt
qu’au quatrième callback, postcheck PID/app/cible erronés, stderr flood/EOF
retenu/cleanup en échec à couvrir. Pins/secret/native non qualifiés.
Bundle antérieur non rejoué. Ticket reste ouvert.
