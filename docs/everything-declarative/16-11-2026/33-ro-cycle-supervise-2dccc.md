# RO totale cycle supervisé2dccc, paquet Qv5

RO statique et fichiers, source exacte
`2dccc070f1564d8ca9d22a59fcb2b73222f415e5`, prepared
`e3741c89cd9c5fbf8d0b5cc6422c204722c2d9c589323fd41ceed2098239837f`.
Nouveau couple ; aucun avis ou GO3c2 transféré. Même ticket C7 de l'armement.
Avis **réservé sur un P1 de finalisation**, après revue du nouveau contrôleur
et des copies PC. Aucun test/process/service/DB lancé par cette RO.

## P1 : la publication finale peut finir après sa deadline et rendre zéro

`lifecycle.py:239-242` contrôle l'heure après hashes/scans, construit
LIFECYCLE_FIXTURE_OBSERVED puis appelle q.save avec deadline=auth.echeance.
Mais `qualify.py:17-20` ne vérifie cette deadline qu'après open et AVANT
json.dump/flush/fsync. Aucun contrôle après fsync ni avant le retour0 du
compose. Le même objet record reste positif sans modification.

Contre-exemple statique, NON JOUÉ : dernier contrôle à END-epsilon,
publication commencée avant END, fsync achevé après END, retour de save puis
retour0 ; le composed conserve LIFECYCLE_FIXTURE_OBSERVED/deadline passée.
Le watchdog physique898+2 peut rester actif après la borne Common830 : il
n'impose donc pas le refus à cette frontière logique de finalisation.
Ce point préexistait au changement contrôleur ; RO totale ne l'exempte pas.

À faire par Arbre sous le même ticket avant nouveau GO : finaliseur avec
contrôle frais APRÈS flush/fsync et juste avant retour, verdict publié/retourné/
code de sortie cohérents, premier rouge conservé en cas de publication tardive.
Prépublier un témoin sur le vrai bloc de finalisation avec horloge franchissant
END pendant fsync, puis correction/source/paquet neufs et RO. Ne pas remettre
900 ou Common à zéro pour faire passer le témoin. Aucun correctif ni témoin
exécuté par ED ici.

## Nouveau raccord producteur relu

Le common exécute --controller avant --driver ; préchecks de contrôle/pins
ne lancent pas de DB. Parent possède Popen/poll de l'acteur, observe ARMED
process/argv/interpréteur/source/CG indépendamment, heartbeat et première
itération, puis admission exclusive et nouveau contrôle avant spawn driver.
ARM10 sous END-500, heartbeat10 et release2 sous même END ; aucun TTL900 acteur.

Le driver installe ACTION_GUARD avant copie/DB et publie READY seulement sous
admission exacte. Source/prepared/allocation/interpréteur/argv/ascendance et
deux nonces liés ; producer_watcher obligatoire au protocole, sans fallback.
Parent supervise aussi les lectures bloquantes. Disparition/rc0 inattendu,
heartbeat périmé ou premier rouge => refus ; la portée commune assure le
nettoyage attribué sur rouge, pas de kill arbitraire dans le mécanisme.
L'acteur garde WAIT_RELEASE après grant sans nouveau contact santé ; sortie
récoltée sous END et résultat RELEASED lié. Aucun autre blocage établi sur ce
nouveau raccord statique. Les tests124 sont rapportés par la publication ;
aucun rejeu, import de leurs modules ou qualification physique inférée ici.

## Vérification indépendante des copies PC

WT source exact/propre. Qv5 contient23 entrées : prepared +22 fichiers
réguliers UID utilisateur, SHA recoupés préparé ET blobs Git2dccc. Pas de
fichier supplémentaire. Acteur SHA
7d7db5a0c7d9ac762eed74ade2d0b9ee256b8499d0029ae970feac916de76612 ;
owner83139/transportedf45 strictement conservés. Python physique
/usr/bin/python3.14 SHAa38210…449d et timeout conformes, aucun exécuté.

P/S/allocation/grant absents, y compris lien pendant. Neuf références COPYA,
cinq AUDIT et cinq FIN physiquement réhashées contre prepared. Le profil
provider reste EXPECTED_PROVIDER_ONLY_NOT_FRESH_HEALTH ; son fichier de
provenance est hashé, sans observation de PID ni GET. Pas de santé fraîche
déduite de cette empreinte historique. Le dossier Q a tous les imports plats
nécessaires à sa CLI ; ne pas lancer la source W sans le raccord PYTHONPATH.

Lecture indépendante sans import du code relu :
R/cx-ed-ro-cycle-2dccc.py et cx-ed-ro-cycle-2dccc.json. Aucune lecture /proc,
pidfd/signal, CPU-fixture, Cargo/rustc, SDK chargé, service/cloud, SQL ou DB
runtime ; aucune création de runtime/autorité/allocation ni mutation target.
Le paquet concordant ne lève pas le P1 de son code. Nouveau GO900 correctif
et toute activité réelle restent exclusivement à C7, après correction/RO.
