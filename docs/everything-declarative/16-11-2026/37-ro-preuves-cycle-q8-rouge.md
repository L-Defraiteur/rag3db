# RO des preuves du cycle Qv8 rouge

Publication immutable **65eb82372e4dec638f0e9557e2e83f44d9d6c1f6**,
rapport06 et manifeste497baaf. Avis **favorable à la fidélité et au périmètre
du rapport ROUGE**, pas au cycle complet. GO C7-LIFECYCLE-HB6C6-Q8-17nov900
consommé ; source6c6b/prepareda5d8 restent les pins de cette exécution.
Aucune nouvelle DB/CLI, requête SQL, health, processus/proc ou reprise native
pendant cette RO ; aucune correction ni tolérance du juge proposée ici.

## Archive vérifiée deux hôtes

PC R/bascule-live-20261116/lifecycle-v8-red-01 et copie locale
lifecycle-v8-red-pc-local-01 : **103 fichiers,17346531 octets**,104 avec
manifeste. Chaque taille/SHA réempreintée indépendamment LOCAL+PC, aucun
fichier supplémentaire. ManifesteSHA
**497baaf330eaa519aa73c8e3488e812b8c8ba15da5de80227ecbb2020aea2d5d**,
égal au blob Git publié. Les22 copies contrôleur égalent les SHA du prepared
a5d8 précédemment recoupé contre Git6c6b. Autorité copiée explicitement
assainie, sans jeton privé.

## Acquis avant STOP recoupés aux RAW

ARMED1265984.765839082, READY1266042.871824671, observation grant
1266043.382686947 ; premier rouge1266055.091886152 avant END1266814.44527935.
Hashes config/armed/admission/READY/grant/ACK, prepared/source/allocation,
nonces et cgroup concordent. L'evidence owner dit honnêtement
OBSERVED_NOT_YET_GRANTED ; le grant et son ACK exclusifs distincts prouvent
la publication ensuite, sans transformer cette seule evidence en grant.

Writer PID913237/start126604879/CLI0e310. Profil capturé : actual==expected,
diff{}, start après identique, exeSHA exact, cgroup avant/après conforme à
l'autorité ; cinq lignes maps librag3db.so59 et cinq libvector59 capturées.
Le RAW contient réponse camelCase pendingCount0, partialfalse, signals[],
nonce6af04ae348ac484ea814685f222a657d et run graph-1a1e9941375-0, puis
closed:true. Hash response-publication lié à shutdown-request ; PID/start
communs aux pièces, writer-closed étatCLOSED/rc0/ACK. Acquisition limitée à
ce transport/publication/shutdown observé, pas à une durabilité démontrée.
Deux health persistées réelles granite768dense/facticefalse, usage baux et
en_cours entiers0. Usage0 n'est pas un décompte global de RPC ou calculs.

## Rouge stable_readers confirmé et limites de comparaison

Traceback driver : lifecycle.py231 appelle stable_readers, audit_result.py57
refuse `read-only copy changed existing bytes or modes`. Premier rouge de
cette cause, puis acteur REFUSED/WAIT_RELEASE1266055.134352207 sur premier
rouge existant. Le refus acteur est consécutif, pas la cause initiale.

Comparaison indépendante des DEUX inventaires persistés :2252→2250 entrées,
aucun ajout ; retraits code-principal.rag3db.ecrivain et .wal ; seul fichier
modifié parmi les communs : DB principale, même taille/mode, SHAeb3144…5a02c
vers3d1cad…599a. Les autres entrées sont identiques. Le juge à la ligne231
exclut cette DB et refuse les deux retraits, conformément au contrat courant.

before_writer utilisé par ce juge était en RAM, NON PERSISTÉ. Le tableau ne
prétend pas le reconstruire : il compare copy-before-db à copy-after-shutdown.
Le code est passé par son contrôle préalable stable_readers et le contrôle
des répertoires avant ce refus ; aucun snapshot supplémentaire n'en découle.
Ces écarts observés n'établissent ni mécanisme natif ni innocuité logique,
ni dette/corruption produit. Aucun juge affaibli ou rouge historique innocenté.

Absents de l'archive : vectors-bits-after, graph-publications, fts-after,
vectors-final, sources-after, driver-result. Ces étapes POST sont NON JOUÉES.
Aucun invariant final des sources FIN/originales ni scan indépendant postmortem
de PID n'est donc revendiqué. Les quiet capturés sont préwriter ; ils ne
deviennent pas des quiet après rouge. Scope : ROUGE/code1/erreurNone/restes[]/
nettoyé,29 identités enfant recensées, autoritéTERMINE ; bus privé indiqué par
le verdict toujours absent au contrôle metadata PC. Ces pièces ne sont pas
un scan /proc frais effectué par ED. ComposéRED est observé, pas une
finalisation positive du cycle ni un résultat VERT.

Suite exclusivement à C7 : éventuelle analyse/recette RO native distincte
après proposition et revue sous le ticket17 OUVERT. Cette RO n'autorise ni
replay, réouverture DB, recovery, health, assouplissement, retrait, LIVE ou
nouveau budget. Les preuves et les données de copie restent conservées.
