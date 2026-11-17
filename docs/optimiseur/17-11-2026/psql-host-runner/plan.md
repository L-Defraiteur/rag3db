# Runner psql HOST — préparation source, aucun GO SQL

Autorisation source C7 b9139349d2, ticket FTS302 :
`docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md`.
Lot distinct `cx-optimiseur-psql-runner-source-20261117-r1`, depuis bundle5a9588.
Aucun fichier v1/Rust modifié. Les 15 bibliothèques sont qualifiées par
ELF/--list/SHA, pas par 15 mappings physiques observés. Programme psql et entrée
noyau chargeur distincts ; SSL/auth/dlopen/locale restent non qualifiés.

## Ordre rouge/source/CPU/relecture

Cette première publication contient 14 tests, PAS runner.py. Aucun test exécuté.
Le rouge attendu est l'absence du raccord runner, à constater sous Common avant
implémentation. Le témoin complet, inchangé, doit ensuite exercer un vrai enfant
Python (pipes, flood, hang, code7, reap) et le vrai publisher avec transport faux.
Les faux ne sont pas des preuves PG. Les tests ne lancent ni psql/chargeur,
Docker, HTTP, sockets, Cargo, PG ni fichier d'identifiants réel.

Recette CPU rouge proposée : Python système, `-B`, unittest test_runner.py depuis
son dossier, sous vrai POSTE32G/Common20/OS28TERM+KILL2=30MAX, une entrée sans retry.
Source/Common et recette hashés dans une nouvelle racine privée avant entrée ;
stdout/stderr/rc+autorité/quiescence conservés deux postes. Attendre relecture RO
et mot C7 pour cette entrée, aucun vieux budget/token/END réutilisé.
Après rouge constaté : implémentation et nouvelles publications, puis recette
verte distincte relue avant son CPU. Rien ici n'autorise contact SQL/fixture.

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

Source des tests publiée avant CPU ; aucun rouge ni vert constaté pour ce lot,
aucun runner livré à ce stade. Bundle antérieur non rejoué. Ticket reste ouvert.
