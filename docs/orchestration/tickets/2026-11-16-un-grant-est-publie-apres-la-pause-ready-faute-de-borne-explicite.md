# Un grant est publié après la pause READY faute de borne explicite

- **État** : ouvert, 16 novembre 2026 ; incident observé sur le cycle privé B5, aucune action writer autorisée par le grant tardif.
- **Famille** : outillage
- **Gravité** : P1 de coordination de validation, borne de phase non attestée côté producteur.
- **Atteignable en service** : non, pilote de qualification sur copie privée et coordination des services.
- **Touche rag3weaver** : oui, indirectement par ses pilotes et preuves ; aucun correctif produit demandé.
- **Pour** : orchestration, Arbre (publication et consommateur READY), Embar (producteur grant).
- **Signalé par** : Arbre et Embar, cycle B5 sous allocation C7-LIFECYCLE-B5-16nov900 ; contrôle RO des fichiers par C7.

## Faits conservés

Source/pilote b5e32aa171e0ee0e6a2cba249593f52d1625095c, prepared630a5c58271666e83d2de24b66de9b899a6b154983551a1889c6902d79138e17. READY nonce e261bb25a6d541139bf78788c3ec5ca1, SHA39cc461eb969f39261eec6eb9126d5d4042fd67f80c3bbcc29a99e4cd7f1c483. Le READY ne portait ni instant observé de publication ni cutoff de la pause ; le constat communiqué PC1195658.16 était ultérieur à sa création, pas une preuve de publication à cet instant. Ne pas reconstituer un timestamp manquant.

Premier refus pilote1195692.294115836, attente de grant absente ou réserve RO insuffisante. ACK Embar observé1195697.090762079, soit4.796646243s après le refus. Producteur reconnaît avoir utilisé l'estimation du constat communiqué plutôt qu'une borne publiée. Le relais C7 ne disposait pas non plus d'une borne attestée ; ne pas attribuer un dépassement de SLA de transport à partir de ces seuls temps.

GrantSHA1a1976689530310e46f2698cadaddd815a66a0611b731524f82d1432c30b70f0 non consommé ; Embar le préserve sous Q/PROVIDER_GO.LATE-INVALID-e261bb25.json, preuve ~/.cache/embarquement-notes/lifecycle-b5/late-grant-invalidated.json. Health Embar postSTOP seulement. Les fichiers provider-before, provider-coordination et writer-process du pilote sont absents : aucune santé par ce pilote ni Popen CLI writer/publication/shutdown. ScopeROUGE1/terminé/restes[]/nettoyé/busabsent/ownedalive[]. Prégatesclone/debt0/pending-promessevides/FTS5/digest97247 passées, pas de qualification du cycle ni de durabilité.

## Travail à préparer avant un nouveau couple

- Arbre publie dans le READY immuable l'observation et la borne réelle de remise du grant, calculées selon le budget restant et la réserve aprèswriter, avec provenance de phase/nonce. Le constat de lecture par un autre agent ne remplace jamais cette borne.
- Embar lit et vérifie le READY exact et ses identités/nonce/SHA/autorité encore active, puis recontrôle la borne fraîche avant publication atomique. Aucun grant d'un job rouge ou expiré ne doit être consommable ; toute preuve tardive reste conservée.
- Le consommateur recontrôle la borne après apparition/ouverture du grant et avant action, pour fermer aussi le cas où le fichier arrive pendant son sommeil puis évite le test de timeout de la boucle. Pas de lecture provider ni writer si refus, et premier rouge immuable.
- Témoins aux vrais raccords : grant dans les temps, grant tardif, changement de nonce/READY, autorité terminée et publication retardée ; aucun PID/provider/DB réel dans ces contrôles purs. Publication du source avant tests, rouge avant correctif.
- Une pause proposée120MAX reste MIN(endcommun−500, now+120) ; la réserve RO500 et les caps call/validité restent stricts. Aucune attente infinie ou expiration recalculée depuis un constat tardif.

Cette étape est une PRÉPARATION seulement : ancien GO B5 terminé et grant révoqué, aucun rattrapage, extension, nouveau créneau ou nouvelle exécution. Nouveau pilote/paquet, RO puis décision C7 distincte avant toute suite.

## Critères de fin

Préparationcouple réalisée, pas de RUN : [recetteArbre15](../../arbre-principal/16-11-2026/15-contrat-ready-borne-producteur-consommateur.md).
Contrat partagé98ab/SHAc67f072b...9ec0, bornesPCobservationavantpub/
grantdeadlineMINobs+120/END−500, autorité/PIDstartCGUIDscriptSHA/firstred
relus ; READYcompletfsync+linkexclusif, checksconsumeraprèsopenetavantaction.
Producteur9052→3aa importéstrict, rechecksaprèshealth/fsyncavantlink ;
callbackownerrestechargédes9identitésprovideretcontactsbornés. Deuxrouges
vraisraccordsConsumerdf610 puis29CPUcouplevertsaprèspublication316d,
producteur11rouges(5fail)→15vertsowner. Aucunservice/DB/provider réel,
ancienB5inchangé. NouveauP/Q/Sv2/paquet/source/RO/GOdistinct nécessaires ;
checks+linknonatomiquesfaceàfinphase et aucune durabilitécrash revendiquée.

Contrat de borne partagé effectivement exercé par producteur et consommateur avec témoins discriminants rouges puis verts ; aucune action après refus ni preuve perdue. Une éventuelle qualification native du cycle restera distincte de la durabilité générale et du LIVE. L'incident ne se ferme pas sur l'envoi d'un message ou le seul renommage du grant.
