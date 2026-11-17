# Réserves groupées : trois contre-exemples avant correction

Réserve request mutable Recherche5e3b consignée au ticket avant ces deux nouveaux
témoins ; réserve hashfinal Mémoire/Recherche déjà prépubliée2b937. Le module
runner8dd368ca6dc56a08d797998d15763a135b4ecd619409f68c524d70d893177c6f
reste INCHANGÉ, aucun fix avant rouge.

Trois cas causaux sur le vrai run_fixture_invocation, ports/horloge MODELED :

- Un port modifie operation_end/end dans la vue reçue, puis passe l'horloge
  hors borne originale. Le refus attendu est operation-deadline avant fork.
- Le port modifie artifact_sha256 et l'environnement imbriqué, puis rend B
  conforme à ces attendus mutés. Refus entry-content attendu avant release.
- Les vrais hashes de retour avancent l'horloge à END+1 : refus final-deadline,
  rc enfant authentique0 conservé. Aucun helper présumant refus prérelease.

Attendu source inchangé :3FAIL «RunnerRefusal not raised»/0ERRORskip/rc1,
NON OBSERVÉ. Une invocation proposée SANSfailfast sur ces trois noms seulement,
TERM58/KILL2. Les24autres méthodes ne sont pas sélectionnées ; total27fonctions
prépubliées/NONJOUÉES sur ce source. Import/fixtureERROR restent ERROR.

Pins groupés runner-snapshot-hash-red-pins.json ; l'ancien seul pin hash25 est
supersédé pour une éventuelle admission, jamais un GO consommé. GO24 demeure
HOLD. RO puis GOrouge distinct/bruts2hôtes avant correction : snapshot profond
privé/nonexposé, vue détachée du port, résultat puis recheck après les deux hashes.
Callbacks noninterruptibles restent MODELED/postvalidation, pascleanupOSborné.
Pur565b/archivesintacts ; aucunimport/test/OS/PQS/backendnatif supplémentaire.
