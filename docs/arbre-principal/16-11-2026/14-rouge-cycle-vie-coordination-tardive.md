# Cycle B5 : refus de coordination avant tout writer

GO C7-LIFECYCLE-B5-16nov900 exactb5e32aa171e0ee0e6a2cba249593f52d1625095c,
prepared630a5c58271666e83d2de24b66de9b899a6b154983551a1889c6902d79138e17,
après ROED/Recherche/C7. Scope validation-9etop4at, common830
END1196403.792607478, mémoire34359738368/HIGHmax/swap0/quota5CPU.
Code/préparation figés ; pas de réutilisation du GO après le premier rouge.

CloneFIN complet rc0, inventaire fidèle ; huit lecturesRO passent : dette0,
marquespendingabsentes/promessevide,5accordsFTS et digesttyped97247vecteurs.
READY nonce e261bb25a6d541139bf78788c3ec5ca1,
SHA39cc461eb969f39261eec6eb9126d5d4042fd67f80c3bbcc29a99e4cd7f1c483.
DigestavantSHA86ce25e2445f9dc412d83ea84a0a1fab37e9aa9a7db70603b047010635f7d9dc.

## Chronologie, acteur par acteur

READY constaté par Arbre à1195658.1637888, puis signalé à Embar. Le fichier
ne contenait ni observed ni pause_deadline ; l'âge annoncé « quelques
secondes » était sous-estimé. Le budget60s courait depuis sa publication,
pas depuis cette notification. Le timestamp exact de publication n'est
pas inventé après coup.

Le driver refuse à1195692.294115836 :
`fresh lifecycle provider coordination absent or post-RO reserve too short`.
Le grant était absent lors du refus, la réserve de500s n'est pas la cause
attribuée : il restait plus de700s à END. Retour1 ; scopeROUGE1/restes[]/
nettoyé/autoritéterminée ; busabsent/ownedvivants0 contrôlés après arrêt.

ACKEmbar observed1195697.090762079, **4.796646243s après le premier rouge**,
grantSHA1a1976689530310e46f2698cadaddd815a66a0611b731524f82d1432c30b70f0.
Il est lié au bon nonce/READY, mais n'a jamais été consommé par le driver.
Embar a reconnu avoir estimé la pause depuis le temps de notification ;
son GETsanté1195697 est POSTSTOP, pas une action de notre pilote avant refus.
Aucun zéroRPC global n'est déduit d'usage0/0.

L'ACK est révoqué, fichier consommable retiré par rename de conservation :
Q/PROVIDER_GO.LATE-INVALID-e261bb25.json, copie et invalidation conservées
chezEmbar. Aucun grant vivant/réservation/permission restante déduit ; aucune
relance ni nouveau slot/GO. provider-coordination/provider-before/
writer-process sont absents côtéclient. Donc **aucun Popenwriter, aucune
publication d'appel, aucun shutdown/ACK/rc0writer jugés**. Les contrôlesPOST
du cycle restentNONJOUÉS. Ce rouge ne dégrade pas le constatFIN_RO antérieur.

## Conservation après arrêt, sans DB supplémentaire

Contrôle fichiers seulement : quatre sources strictement inchangées, nouvelle
fixture octets/modes/dirs inchangés ; aucun ajoutreaders/WAL, tous pins exacts.
Scans source/fixture found[] ;11UNKNOWNglobaux non attribués consignés comme
inconnus. Aucune réouverture DB/provider/CLI pour cet audit après refus.

PC ~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-red-01 :65fichiers,
16680667octets, manifesteSHA
cddb26214c6c0176c96566a091f09102b8415926691ffbe8f01060556221ec03.
Copie locale lifecycle-red-pc-local-01 :65tailles/SHA réempreintés sans écart.
Manifeste joint lifecycle-red-proof-manifest.json. Scripts figés, préflights/digests/FTSIDs/profil, READY,
grantinvalidé et statutscope publics préservés, sans DB/corpsvecteur/jeton.
P/Q/S et fixture restent conservés. Les autorités900/600 précédentes et
leurs verdicts ne sont pas réécrits ; tickets restent ouverts.

## Préparation suivante à arbitrer, pas de nouvelle exécution

Avant tout diff correctif : consigner le manque de borne absolue du READY
dans le même ticket. Publier observed+pause_deadline et son END, partager
cette borne (horlogePC) plutôt qu'une estimation depuis le message ; acteur
doit vérifier firstredabsent/autoritéactive et marge avant son health, puis
à nouveau avant de poser le grant. Un reçu après refus reste invalide.

Réserve de code identifiée statiquement, distincte de l'événement observé :
la boucle teste la deadline seulement tant que le fichier est absent.
Un grant arrivant juste après la borne mais avant le prochain test exists
pourrait sortir de la boucle sans recontrôle de pause. Aucun tel grant n'a
été consommé ici. Préparer témoin de la VRAIE branche avec grantdéjàprésent/
clocktardive, rougeavantfix, puis recontrôler borne avant consommation.
Nouveau paquet/RO/GO distinct requis ; ni prolongation ni reprise du writer
par l'ancien ACK, et aucune modification de la limite à la volée.

Ticket commun publié par C7 AVANT correction : be3a4d260e,
docs/orchestration/tickets/2026-11-16-un-grant-est-publie-apres-la-pause-ready-faute-de-borne-explicite.md.
Préparation autorisée seulement : observation avant publication et borne
absolue partagées, cap120MAX/MINEND−500, acteurcheckétat/firstred/borne avant
linkatomique et consommateur après fichier puis avant action ; nouveaux
témoins rouges→verts et RO avant toute nouvelle admission. Cela ne remplace
ni le rouge présent ni la qualification native du cycle encore manquante.
