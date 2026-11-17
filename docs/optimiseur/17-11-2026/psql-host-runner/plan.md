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


## Complément après RO exacte ab65ac242b

R favorable SOURCE/recette nominale uniquement, aucun rouge observé. Le paquet
privé source manquait lors de cette RO. Identité interpréteur relevée en
préparation RO sur PC : /usr/bin/python3 résout /usr/bin/python3.14, SHA256
a38210a53e07ac5eb912ee35c2d92ddb7eaa349301b60c188a5f496c7c87449d,
version 3.14.10/GCC16.2.1. La recette porte désormais ce pin, à revalider
avant admission. Cela ne qualifie pas toute sa fermeture dynamique. Source
runner/test inchangée. Copier les seules sources plus Common6439 dans le root
privé neuf, comparer chaque SHA, puis relire ce paquet avant GO distinct.
Ni unittest ni import runner/test lancé pour cette préparation.


Complément final C7 avant admission : timeout SHA9c3e4dc9…1942b7 et poste
SHA79e3a8bb…584c7b (chemins résolus/UID dans recette) ajoutés après lecture RO
de leurs seuls octets. Racine privée PC uid1000/mode700, trois sources uid1000
conformes, aucun common-proofs ni ancienne autorité d'entrée dans cette racine.
La recette précédente recipe.json/bb5 reste conservée ; nouvelle recette finale
à poser sous recipe-final.json, sans réécrire les anciens reçus de préparation.
Ces métadonnées ne sont pas un reçu d'autorité actif ni un GO CPU.


## Rouge nominal effectivement observé — GO3215b9ec5d

Entrée UNIQUE C7-PSQL-BASELINE-A5C2-17nov30, sourcea5c23cd7d8317ea34afdf59e800e54899b4bbd47,
recette finale99251084fd6e6033697d55bd07dc47efec0e15c3ef0be0ec1e6ccf4796a956ab.
Portée `validation-cniyids7` : Common ROUGE/code1, erreur superviseur null,
état terminé/nettoyé, restes[]. 1 test, 1 FAILURE explicite capacité nominale
indisponible / RunnerUnavailable, 0 ERROR, 0 skip ; rc1.
Monotone1278263.786415292→1278264.045580457, elapsed0.259165165014565s.
Ce rouge métier ne devient pas un vert ; les 13 autres restent NON JOUÉS.
Aucun RealCapture, natif psql, SQL, Docker, PG, provider, service ou secret.

Preuves PC : ~/.cache/optimiseur-notes/psql-host-runner-20261117-r1/red-baseline-02/ ;
portable : même parent/red-baseline-02-proof/. 16 fichiers comparés taille/SHA
sur les deux postes, Common public sans jeton et bruts/recettes/sources présents.
Manifeste e7c4dd95f00a95040c593ab90f95bca3d513a29e13cc50238cd3f3da17d403dc ;
archive617a8394a66079790f81cb558cb94048c274b819de038a1766957e281760e4d3.
L'autorité brute Common demeure privée sur le PC ; aucune autorité active ni
grant n'est transféré. Aucun rejeu effectué.

Préparation d'implémentation désormais autorisée par C7, après cette conservation
de preuves. Les trois réserves R de couverture restent ouvertes ; nouveaux
témoins à publier avant implémentation dédiée, nouvelle RO et GO CPU distinct
avant un éventuel vert. Aucun GO SQL/fixture déduit.


## Témoins de réserves R prépubliés avant implémentation

19 tests source désormais, tous nouveaux témoins NON JOUÉS. Le rouge acquis
reste celui de l'ancien paquet a5c2 privé immuable, jamais transféré au nouveau
test_runner. La recette red-baseline est HISTORIQUE et ses pins ne sont pas
ceux de ces tests complétés ; elle ne sera pas rejouée. runner.py reste la
baseline au refus explicite, aucune implémentation anticipée dans ce commit.

Expiration : wrapper de sync_directory appelle le vrai fsync puis franchit
END du faux clock ; le témoin exige que cette frontière ait réellement été
franchie et un refusal non positif. Ce n'est plus le quatrième phase_check.
Postcheck : désaccord PID RO/observer, mauvais PID cible, application originale
et suffixe:check, user/base de la session. Capture : flood stderr, SIGTERM
ignoré, parent fini avant EOF détenu par petit-enfant privé borné .35s ; ces
futurs tests lancent de vrais enfants Python sous recette/GO distincts.
Publication : captures non reaped/partielles/tronquées/tardives/timedout/code
booléen refusées avant observation positive. Injection de métadonnées négatives
ne prouve pas à elle seule le nettoyage OS, à qualifier par les témoins réels.
Ces témoins ne ferment aucune réserve tant qu'ils ne sont pas passés sur le
raccord implémenté ; relecture source demandée avant future qualification.

## Raccord source après prépublication19tests — non exécuté

`runner.py` remplace la baseline après le vrai rouge métier et la prépublication
10d48cf6ec. Aucune importation, compilation Python, unittest ou invocation native
pour ce raccord. Six pins source dans source-preparation-manifest.json. Le vieux
paquet a5c2 et sa recette rouge restent immuables ; pas de transfert de résultat.

Composition : pg_receipt.py/de69 et identity/postcheck exacts inchangés ; réemploi
explicite decode/shape/sql_shape et écriture exclusive/fsync/lecteur borné. Le SQL
RO existant f40eb6ebc867839303aa51229497e7d533c6708854bea9b63bcfe67f2012b8b9
est réutilisé sans modifier le code Rust/v2 gelé. Le publisher v1 reste intact,
non appelé avec des identités de fixture. Le nouveau publisher/consommateur
fixture ont leur type propre, aucun grant ou admission v1.

Capture : pipes non bloquants64KiB/canal, fin de travail avant réserve cleanup,
TERM puis KILL par pidfd du seul enfant attribué, wait borné au même END ;
EOF retenu après sortie directe = partiel. `/proc/exe/start/maps` observés si
possible, UNKNOWN sinon ; maps prefix16KiB explicitement incomplet. Un détenteur
de pipe descendant inconnu n'est pas prétendu arrêté par le reap du parent :
refus conservé et portée Common externe doit attester quiescence. Tout résidu,
nettoyage tardif ou inconnu interdit une qualification positive. Cette source
n'est pas une preuve de ces garanties, en attente de tests/RO/GO dédiés.

`observe` : whole5 avant factory/pins/secret jusqu'au dernier fsync + contrôle
phase ; captures et postcheck dans cet END, cleanup natif passfile avant
publication, marqueur refusal invalide même une pièce liée juste avant refus.
`read_fixture_observation` relit les bruts bornés dans le même répertoire et
refuse le marqueur avant/après. Le champ ended du corps est un instant pré-fsync ;
le dernier gate garantit seulement publication avant deadline, pas un fsync
physiquement instantané. Les nouvelles réserves de couverture restent OPEN.

`PsqlSession` prépare un vrai argv psql/chargeur du SEUL ledger qualifié824b59,
non relocalisé ; rehash299fichiers et garde externe avant chaque stade.
Identifiants uniquement de la future allocation privée par nonce, fichier
observer-password600/UID/nlink vérifiés, jamais utilisateur/.env/.vault.
PGPASSFILE600 neuf, HOME/TMP700, environnement liste blanche, -X/--no-password/
ON_ERROR_STOP. Mot de passe absent des argv/SQL/stdin/logs ; sortie contenant son
canari remplacée intégralement avant refus. Aucune de ces lectures secrètes ou
écritures de passfile exécutée à ce stade. Core0 doit être posé par le lanceur.
GUC/defaultRO mesurés sous BEGIN READ ONLY, timeout serveur/connect/lock/idle
bornés, postcheck PID OU app depuis une session distincte, retrait passfile dans
la fermeture du même whole5. Ce source ne contacte rien à son import ; sa seule
entrée script termine SOURCE ONLY, sans chemin live.

**Manques explicites** : lanceur d'admission réelle Common/lease/allocation
encore à qualifier (callbacks des faux ne le remplacent pas), recette CPU verte
exacte pas encore publiée ni GO acquis, fixtures/PGPASS synthétiques et raccord
native à tester, profils officiels v1 non exécutés, aucune vraie session PG.
Après erreur de connexion/query, l'absence backend n'est PAS attestée ; pas de
cancel/completion fictif, observation refusée. SSL/auth/dlopen/locale/transport
réels demeurent non qualifiés. Aucun nouveau PG à allouer avant client prêt et
GO contact séparé. R doit relire ce raccord et les réserves regroupées avant
nouveaux pins/GO CPU, puis résultats réels avant toute proposition de fixture.


## Réserves source R regroupées et quatre contre-exemples prépubliés

9a3 reste inchangé : pas de fix avant rouge. Réserves1/2+2/2 consignées au
ticket AVANT modifications des seuls témoins/documentation. Nouveau module
test_review_regressions.py : première cause capture malgré close en échec ;
exception pidfd sans perte de record ; vrai consommateur face à sidecar absent ;
expiration au vrai fsync après lien avec panne d'écriture refusal puis vrai
consommateur. Popen/pidfd sont FAUX pour le test OS, aucun enfant natif réel.
Les écritures/fsync/observer/consumer sont réels sur TemporaryDirectory.

Recette PROPOSÉE regression-red-recipe.json : quatre seuls contre-exemples,
attendu4FAIL/0ERROR/0skip/rc1, Common20/OS28+2 après vrai POSTE32G. Sept sources
pinnées dont quatre v1 figées, plus Common et outils identiques à l'entrée
nominale. Sources privées PAS encore copiées ni recette admise, aucun GO CPU
acquis ; R doit lire ces contre-exemples puis C7 décider. Les 19 autres témoins
ne font pas partie de cette entrée proposée et restent NON JOUÉS.

Le témoin SIGTERM des19 est complété par HANDLER_READY et returncode=-SIGKILL
pour prouver la causalité ; NON JOUÉ. Native exeSHA/env/pins module et portée
whole5 complète restent non qualifiés avant tout GO contact. Les quatre
contre-exemples ne ferment pas ces autres limites, ni l'ancienne réserve OS.


## Quatre rouges de relecture observés — GOd72aa02288

Entrée UNIQUE C7-PSQL-REGRESSIONS-RED-5DD6-17nov30, source5dd661bf70,
recette9e4b2154b92f3ef01e227a65ed10e840718ee6c91a356adbc3f53d378c0a68ce.
Common validation-3xm6hlp5 ROUGE/code1/erreur null, terminé/nettoyé/restes[].
4 tests/4 FAILURE précis/0 ERROR/0skip/rc1, elapsed0.2648362130858004s.
Deux ValueError non levés côté consumer, première cause absente après cleanup,
OSError pidfd sans record confirmé. Ce sont les quatre rouges attendus, pas
un vert.19autres NON JOUÉS ; Popen/pidfd faux,0enfantnatif/PG/SQL/service.

19 fichiers de preuve conservés et vérifiés taille/SHA sur les deux postes,
AVANT fix : PCpsql-host-runner-20261117-r1/regression-red-03 ; portable même
parent/regression-red-03-proof. Manifeste
d42e78e677012e4cee5e6abdbda20d600bff74f03c5c2b33b7c1cc41b50df86a ; archive
d70049acf3cf65a61d08cecbc63e830946e652fa358774104f2f104d87ff041c.
Les reçus Common transférés sont sans jeton, autorité brute PC privée.
Préparation fix des trois défauts maintenant autorisée C7 ; nouvelle publication
source/RO et GO CPU distinct nécessaires avant éventuel vert. Les limites
natives/exe/env/outerwhole5 restent non qualifiées et sans GO contact.


## Fix source des trois défauts après quatre rouges — NON EXÉCUTÉ

Conservation deux hôtes et rouge fb3d acquis AVANT ces changements.
Capture englobe Popen/selector/pidfd/loop/cleanup : première cause et erreurs
de cleanup séparées dans CaptureRefused.record, flags partiel/inconnu sans
blanchiment d'une race. Nettoyage ne remplace pas la cause initiale. Observe
archive capture.json et bruts sur refus comme sur succès ; sa fermeture garde
first_error et cleanup_error distincts dans le refusal.

Publication : sidecar et publication-complete.json après lien/fsync/gates ;
un dernier gate après fsync completion reste obligatoire. Ce marqueur INTERNE
ne suffit pas au lecteur : read_fixture_observation exige terminal_receipt +
terminal_sha256 EXTERNES, pièce bornée/pinnée attestant Common TERMINÉ/VERT/rc0/
nettoyé/restes[] et liant artifact path/SHA et completionSHA. Pièce, sidecar,
completion, timings et bruts vérifiés, marqueur refusal refusé avant/après.
Une interruption laissant un lien mais sans terminaison externe admise ne
produit donc pas de lecture admissible. Le producteur réel de cette admission
terminale n'est PAS livré/qualifié ici et doit venir du superviseur extérieur
après sortie et quiescence réelles. Un booléen du writer ne le remplace pas.
Les erreurs tardives sans refusal restent rouges, même si des fichiers subsistent.

Quatre témoins complémentaires SOURCE (8 dans test_review_regressions) :
lecture positive de pièce COMPLETE + terminal SYNTHÉTIQUE, scope terminal ROUGE
refusé, sidecar requis même avec terminal, completion requise même avec terminal.
Ils empêchent de confondre toujours-refuser et consommateur fonctionnel. Ces
pièces synthétiques ne sont pas une admission réelle. Les quatre rouges
préexistants sont conservés inchangés dans ce module ; tous huit NON JOUÉS sur
le nouveau source, 19autres NON JOUÉS aussi. Aucune importation/parse/CPU/natif.

Pins source dans fix-source-manifest.json. Anciennes recettes rouges et preuves
restent immuables et NE SERONT PAS REJOUÉES sur ces nouveaux pins. R doit relire
fix+nouveaux témoins avant recette CPU distincte (huit seuls fake/FS proposés,
pas RealCapture) et éventuel GO C7. Aucune qualification verte acquise.
Réserves native exe/env et outer whole5/import/fsync tardif maintenues ; aucun
GO contact/secret/PG/fixture, aucun service/original/lib/SDK changé.


## RO fix d787 favorable SOURCE, recette CPU huit seuls proposée

R a levé STATIQUEMENT les trois réserves initiales dans la frontière fix3 :
premièrecause/record/consumerterminal. Aucun de ces résultats n'est un vert
runtime. Deux anciens contre-exemples consumer sont désormais refusés aussi
pour manque de terminal, les quatre nouveaux vérifient non-vacuité et pièces.
Producteur terminal réel/non falsifiable, exécution/env natives, outer whole5
et limites EOF/arbre restent hors qualification.

fix-cpu-recipe.json sélectionne UNIQUEMENT test_review_regressions (8 tests),
attendu8/0échec/0ERROR/0skip/rc0, même Common20/OS28+2/POSTE32G. Popen/pidfd faux,
FS/fsync/observer/consumer réels privés et pièces terminales SYNTHÉTIQUES.
Les19 autres NON sélectionnés, PsqlSession non instanciée, aucun child natif,
client, secret, PG, Docker, provider ou service. Paquet source et outils à
matérialiser/rehash sous racine neuve fix-cpu-04, puis RO et GO C7 DISTINCT
avant entrée. Aucun GO CPU acquis ni exécution de ce correctif à ce stade.
