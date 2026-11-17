# Deux familles résiduelles : cinq contre-exemples avant correction

RO totales Mémoire/TTY du pin e15db5a4a87220e2de0b38c3721e4461844c7f36,
HOLD58 C7/3a4c24b3ca. Le ticket ledger contient les constats AVANT ce diff.
Moteur et loader restent STRICTEMENT e15, aucun correctif anticipé. Aucun
import/parser/test/processus/SSH/natif ou ancien jeu exécuté.

`test_publication_residual_counterexamples.py` sélectionne cinq fonctions,
aucune autre classe de tests n'est réexportée :

| Cas | Raccord réel | Motif attendu, NON OBSERVÉ |
|---|---|---|
|first-red journal refusé|Relay.run|OSError du journal remplace l'objet RuntimeError initial et saute abort|
|cleanup-refused journal refusé|Relay.run après échec abort|OSError du constat remplace l'objet initial|
|local-stop-refused journal refusé|Relay.run finally après échec stop|OSError du constat remplace l'objet initial|
|reap puis close refusé|SSHTransport.stop|OSError de close remplace l'objet d'erreur de reap déjà acquis|
|prepared second-read substitué|publication_loader.verify_entry|JSON B est retourné avec pin des bytes A ; données hors snapshot vérifié|

Les quatre premiers assertent l'IDENTITÉ de l'objet d'erreur, pas un message
ou un TimeoutError générique. Après cela ils exigent cleanup tenté quand possible
et un record de secours en mémoire : first_error=repr(original), secondary_errors
conservés et remote_quiet=UNKNOWN dans ces montages SANS attestation distante.
Ce contrat de record est proposé avant implantation ; ce n'est pas une archive
réussie, ni un reçu CP/LIVE. On n'efface pas un fait de quiescence authentiquement
acquis dans un autre contexte au seul motif qu'un disque a refusé son journal.

Les pannes d'atomic_json sont sélectives sur les trois vrais noms de journaux ;
le reste passe par la vraie primitive. Reap/close sont des frontières fictives,
jamais des processus/signaux OS. Aucune autre exception de montage n'est
convertie en rouge. Les registres de secours ne sont interrogés qu'APRÈS
l'assertion d'identité : sur e15, les quatre causes doivent d'abord échouer
précisément à cette assertion, pas sur un champ absent.

Le cinquième cas conserve réellement le fichier prepared A et son SHA ; seul
le second read_text fournit B avec execution_host altéré. La vraie fonction
verify_entry reste utilisée et parcourt les contrôleurs réels de la fixture.
Après fix, lire une seule fois bytes/hash/JSON doit retourner A. Aucun thread,
fichier partagé, mutation de l'hôte ou remplacement du loader n'est nécessaire.
La primitive de snapshot liée au pin devra servir les autres lecteurs prepared
relay_main/gate_main/pilot.main aussi : le préimport ne justifie pas une
relecture ultérieure non liée au même pin.

Prévision CINQ FAIL/0ERROR/skip, rc1, pas un résultat. Pins et commande future
unique dans publication-residual-pins.json ; revue ciblée M/TTY puis GO CPU
DISTINCT avant toute observation. Pas de GO58/8792/f3d réactivé. Les anciennes
preuves sont conservées sans modification ; les deux familles restent ouvertes
jusqu'à correction groupée, revue totale et futur GO des58+nouveaux cas.
