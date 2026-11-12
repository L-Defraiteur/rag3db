# Runtime unique vert, capture complète, saturation non reproduite

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Paquet35ea figé, une tentative autorisée, aucun retry.

**Exactement un test demandé/joué, vert ; capture strictement valide. Aucun refus
capturé aux deux sites instrumentés pendant l'armement. Le rouge historique n'est
pas reproduit et sa cause reste indéterminée.** Le ticket demeure ouvert ; ce
résultat ne remplace ni les preuves ciblées de sûreté0511, ni le diagnostic distinct
de couverture de quarantaine pour les transactions déjà ouvertes.

## Résultat, arrêt et ressources

Poste mesure acquis le12nov à14:40:25 après attente derrière un lourd déjà engagé.
Budget démarré après acquisition. Enfant2013219, groupe2013219, contrôleur2013218 ;
un seul Popen, aucun retry. Natif0, driver0, unité Resultsuccess/inactive/MainPID0.
Attente wait de l'enfant terminée ; aucun signal TERM/KILL, stop_reason=null.

Service988,102s ; contrôle988,038223s ; test GTest987,526s. CPU unité4439,884s,
pic4,2G tel que rapporté par systemd, swap0 ; cgroup8G/swap0, scratch dur2G,
réseau isolé et source/packageRO attestés. Les trois fichiers bruts totalisent
128792octets, sous16Mio. [Résultat](preuves/runtime-saturation-01/result.json),
[limites](preuves/runtime-saturation-01/run-limits.json),
[journal unité](preuves/runtime-saturation-01/unit.log),
[état final](preuves/runtime-saturation-01/etat-final.txt).

Le poste mesure est libéré. Les lourds d'autres sessions en attente ont repris :
la lecture finale montrait3/4 occupés, **aucun à cette session**. Ne pas présenter
cela comme un poste globalement vide. Aucun service/GPU/PG ni magasin utilisateur.

## XML et capture : deux validations distinctes

[XML](preuves/runtime-saturation-01/result.xml) : racine tests=1/failures=0/errors=0/
disabled=0, un seul testcase TwentyWriters.DdlEveryMinute, status=run,
result=completed. Pas de test supplémentaire, skip ou découverte préalable.
L'horodatage XML13:40:25.754 est celui de l'environnement TZ=UTC explicite,
correspondant au lancement14:40:25 en Europe/Paris ; les événements du scénario
restent des durées monotones depuis son origine propre.

[CSV](preuves/runtime-saturation-01/capture.csv) :984samples, un schéma exact,
16uint64 par relevé, valeurs/domaines conformes, cumuls monotones, intervalles de
lecture ordonnés, footer complete=1/first_state=0/sample_count=984/overflow=0.
Aucun first_meta/first/first_saved : cette absence est **attendue pour state0**,
pas une première capture perdue déclarée réussie.

[Validation stricte](preuves/runtime-saturation-01/validation-stricte.json) par
[outils/cx-valider-capture-conservee.py](outils/cx-valider-capture-conservee.py),
appliquée après arrêt seulement : zéro erreur et exactement un test demandé joué.
Elle vérifie les uint64 et le protocole complet, au-delà du shape_ok initial du
driver. Son verdict d'intégrité n'est pas utilisé comme substitut au verdict GTest.

Les834fichiers du paquet sont rehashés après le passage sans écart :
[reçu du paquet](preuves/runtime-saturation-01/package-after.json).
Driver3445e990… et wrapper912454a5… sont restés ceux publiés avant lancement ;
source35ea et bibliothèques privées/distribuées non modifiées pendant ce passage.

## Chronologie observée et pression du pool

[Log intégral](preuves/runtime-saturation-01/stdout.log) et
[extraction numérique](preuves/runtime-saturation-01/analyse-compteurs.json).
Temps imprimés en millisecondes, arrondis ; les pics sont des maxima de fenêtres,
**jamais une occupation instantanée au début du DDL**.

| DDL | Début ms | Fin ms | Pic fenêtre avant / pendant, Mio (sampler100ms) |
|---|---:|---:|---:|
| 1 ADD note | 60000 | 60002 | 1079 / 0 |
| 2 ADD rang | 120000 | 120001 | 1100 / 0 |
| 3 CREATE REL TABLE | 180000 | 180000 | 1111 / 0 |
| 4 CREATE_VECTOR_INDEX | 240000 | 969144 | 1120 / 1045 |
| 5 CHECKPOINT | 969144 | 969620 | 0 / 126 |
| 6 ADD b | 969625 | 969627 | 0 / 0 |

Les zéros de fenêtres courtes peuvent signifier absence d'échantillon, comme dans
doc13. Le pic global100ms est1120Mio ; celui des984relevés supplémentaires1s est
1170551093octets, soit1116,3245Mio. Les cadences différentes expliquent que ces
maxima diffèrent, sans signaler une incohérence.

Le DDL d'index dure environ729,144s. Il réussit ; les DDL suivants démarrent ensuite
sans respecter leur échéance initiale déjà dépassée. ADD b réussit et les défauts
sont corrects. L'ADD a répété de la migration est refusé comme prévu ; son COMMIT
confirme l'annulation, sans annulation inachevée observée.

Dans la fenêtre après DDL2/avant DDL3, le pic100ms est1111Mio, contre2051Mio dans
l'ancien isolé rouge. Le passage n'atteint donc pas la même pression **observée**.
Le premier refus des sites reserve/limite de groupes n'existe pas dans la fenêtre
armée ; le log ne contient ni pool full, ni limite de groupes, ni rollback incomplet.
Cela n'est pas une preuve exhaustive de toutes les erreurs/allocation possibles,
ni un verdict sur le chargement initial ou les opérations après désarmement.

## Compteurs PK et volumes : ce qui est effectivement mesuré

Tous les984relevés donnent pk_calls=0, pk_completed_ns=0, pk_active=0.
Dernier relevé cumulatif : t=984,063677238s, read_end=984,063677252s.
**Aucun appel au retrait PK instrumenté jusqu'à ce relevé.** Le footer ne contient
pas de relevé terminal des compteurs : l'intervalle final après ce dernier sample
et les opérations hors armement ne permettent pas d'affirmer zéro appel sur toute
la durée du test. Pas de nouveau run pour combler ce défaut de couverture.

En général pk_completed_ns exclut intégralement les appels encore actifs ;
pk_active>0 aurait interdit d'écarter le coût PK sur le cumul seul. Ici le compteur
d'appels lui-même reste zéro dans le préfixe relevé. Les144erreurs d'acteurs sont
1doublon et143conflits de sérialisation, sans faute d'annulation enregistrée. Les
écritures locales refusées n'impliquent pas un INSERT_INFO persistant à retirer.

Au dernier sample : transferred_rows825144, published_commits1743208,
writer_ok1743201/writer_error144, actor_ok2642367/actor_error144.
Ce sont les métriques définies dans doc14, pas des cardinalités durables. Active26/
writers22 à ce sample n'indique pas des transactions retenues après arrêt : les
acteurs tournent encore à cet instant et il n'y a pas de relevé de jauge terminal.

Les contrôles finaux du scénario comparent les comptes avant checkpoint et après
réouverture : Doc60000/60000, Link1078120/1078120, Note821445/821445 ; défauts corrects.
Ces volumes diffèrent fortement de la base historique (Doc59927,Link447312,
Note59683). Le rouge isolé ancien n'avait pas fourni ses comptes finaux. Le débit
réalisé et l'environnement historique insuffisamment attesté interdisent donc
une attribution causale base/correctif à partir de ces seuls passages temporels.

## Départage obtenu et inconnues restantes

| Question | Conclusion de ce passage |
|---|---|
| Coût du retrait PK | Chemin non entré jusqu'au dernier cumul observé ; aucun coût PK à attribuer à ce préfixe. Ne mesure pas celui du rouge ancien. |
| Pression avant DDL | Fenêtre avant DDL3 nettement sous celle de l'ancien rouge ; saturation recherchée non reproduite. |
| Autre faute d'annulation | Aucune observée dans les messages/sites instrumentés ; ni preuve générale de quarantaine ni raison d'instrumenter tous les préfixes. |
| Causalité du rouge historique | Indéterminée : pas d'événement cible, pas de charge réalisée appariée. |

Aucun regroupement A ou optimisation COPY justifié par ce résultat. Il ne ferme
pas le ticket moteur, n'autorise ni intégration de lib ni autre mesure. La suite
utile est d'abord une décision de revue sur une condition de reproduction
contrôlable et les différences d'environnement/charge conservées, plutôt qu'un
nouveau long passage implicite. Aucun second runtime préparé ou lancé ici.

## Export et conservation

[Archive de preuves](preuves/runtime-saturation-01/cx-runtime-saturation-12nov-01-preuves.tar.gz),
36276octets, SHA256
`95eaf8e4b3611a4c9f97be52fb59b294842dfc0a46bd55d4a90e7728ffe1d663`.
23fichiers du manifeste final rehashés après export local, sans écart ;
[reçu](preuves/runtime-saturation-01/recu-export.json). Driver,env/argv,publication,
préflight,PID,limites,logs/XML/CSV,résultat,validation stricte,hashes et état final
conservés. Analyse numérique locale liée ci-dessus, produite après arrêt.

Copie stable hors worktree à luciepc :
`~/.cache/rag3db-moteur-notes/annulation/cx-runtime-saturation-12nov-01/` ; archive
voisine `cx-runtime-saturation-12nov-01-preuves.tar.gz`.
Le scratch et sa base de test privée ont fini avec leur namespace ; aucune base
utilisateur touchée. Aucun processus, créneau CPU ou poste mesure conservé par
cette session après le passage.
