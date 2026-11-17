# Rouge réel des réserves R1/R2 du pipeline

GO C7-TTY-PIPELINE-RED-C004-17nov60, source exacte
c004ad63560e7f883a6fd72139b7bb292fa8d2b1, testd998cea…4fb6f,
driver9a0b5032…352e inchangé, Qc003 conservé INACTIF.
Une seule invocation, sans failfast, depuis le worktree :
`timeout --signal=TERM --kill-after=2s 58s python3 -B docs/tty/17-11-2026/test_qualification_review.py`.

**8 tests : 2 réussis, 6 échecs, 0 erreur, 0 ignoré ; rc1**.
Les contrôles positifs filets2+TTY4 et should_panic passent.
Les cinq contre-exemples du parser échouent car l'assertion de refus n'est
pas levée : zérotest masqué, noms étrangers, faux filets, doublon et ordre.
Le vrai stage extrait par AST ne demande aucun arrêt pendant le fauxbackend
qui émet FAILED : témoin R1 rouge sur absence du stop avant continuation.
Ce sont les défauts visés, pas un échec d'import ni un ERROR de harnais.

start871646.767809808, finished871646.835258793 :0,067449s englobantes.
Le unittest rapporte0,018s ; ces durées ont des frontières différentes.
Aucun enfant OS dans les témoins, aucun Cargo/native/service/acteur/contactSQL,
aucun slot lourd ni targetmuté. Le wrapper timeout est celui du coupon autorisé.

[Preuves](preuves-pipeline-c004/manifest.json). RAW complets archivés puis
réhashés LOCAL et luciepc :
`~/.cache/rag3weaver-build/archives/cx-tty-pipeline-red-c004/evidence.tar`,
SHA2565b9f1363d2cf588312a84aa6bcb266a0c3ebb6eb3780febf11c7e6ea621ff153.

Même ticket anglais OPEN. Aucun correctif R1/R2 ni essai vert exécuté sous
ce GO rouge ; nouveau couple/pins/revue/GO distinct avant suite.
Les HOLDacteurPCLOCAL/politique/payload/SQL/release restent distincts.
