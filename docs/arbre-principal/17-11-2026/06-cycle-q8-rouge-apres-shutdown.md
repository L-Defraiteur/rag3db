# Cycle Qv8 : rouge après shutdown, preuves conservées

GO **C7-LIFECYCLE-HB6C6-Q8-17nov900** consommé, clos ROUGE sans retry.
Source6c6b968832d53d76f50ae5244454fe076a8e29c2, preparedQv8
a5d86165be320eccfe0f2455a16ecb121abe0cdbe501c3206ea7e23bbfe8b445.
Une entrée `python3 -B Q/lifecycle.py --launch SOURCE --allocation GO`,
après garde fraîche source/pins/propreté/absences. Aucune DB originale,
service7970, branche master ou ressource Embar modifiée par cette entrée.

## Chronologie brute et arrêt

Common validation-fashfd7y, END1266814.44527935 ; enveloppe outer898+2,
Common830. ARMED puis admission indépendante avant copie/DB. READY et grant
autonome effectivement publiés ; grant_deadline1266162.871824671.
Ces passages ne reposent pas sur un accusé humain.

Writer PID913237/start126604879, CLI SHA
0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288.
writer-runtime-profile contient diff={}, maps réels, start/cgroup/exe
avant/après. Réponse publique camelCase pendingCount0, nonce
6af04ae348ac484ea814685f222a657d, run graph-1a1e9941375-0 ; publication
locale puis requête shutdown. Raw writer.stdout contient closed:true et
writer-closed.json atteste ACK/même identité/rc0. Health avant/après persistés,
usage baux/en_cours entiers0 ; aucune requête embedding autorisée.

Premier rouge1266055.091886152 :
`read-only copy changed existing bytes or modes`, audit_result.stable_readers
appelé lifecycle.driver ligne231 après inventaire shutdown. Watcher ensuite
REFUSED en WAIT_RELEASE1266055.134352207 sur premier rouge existant.
Composé RED, session rc1 ; scope ROUGE1/restes[]/nettoyé, autorité terminée,
répertoire bus privé absent observé après fin. Aucun retry/reset/recovery.

## Diagnostic borné aux JSON déjà capturés

copy-before-db.json :2252 entrées ; copy-after-shutdown.json :2250.

| Chemin | Différence capturée |
| --- | --- |
| code-principal.rag3db.ecrivain | retrait ; avant254octets, mode420, SHA30bfa672b741560b87a0713eeab03f5e64c5cff2a93517e65f9bce292f1243c9 |
| code-principal.rag3db.wal | retrait ; avant1589305octets, mode420, SHAdf6b1c34db87b6f69c312365fdd37123c665b77337738b4dec6f606b9fff53bf |
| code-principal.rag3db | taille1842655232/mode420 conservés ; SHAeb3144d6f3e98f5e9a4f1ab148ce7cc931b8d25c395d596bf653de63b065a02c →3d1cad2b2d7c528820a9c2b4fa8ad6e8a3168d7aa8bcefa43017798052bf599a |

Aucun ajout, aucune autre entrée modifiée dans cette comparaison. Le juge
exclut la DB principale et refuse les retraits des fichiers existants.
L'inventaire before_writer en mémoire n'a pas été persisté séparément ; son
contrôle préalable stable_readers(initial,before_writer) est passé, permettant
seulement l'ajout éventuel d'un .readers vide. Le tableau compare donc les
deux inventaires réellement persistés, sans fabriquer un troisième témoin.
Les répertoires après shutdown ont été capturés et leur égalité a passé avant
ce rouge. Aucun nouveau scan/lecture DB après rouge pour innocenter ces écarts.

Ces faits ne qualifient ni leur cause native ni leur innocuité logique.
Pas d'assouplissement du juge : analyse/proposition/revue avant toute suite.
Les vecteurs bits/graph/FTS après shutdown et finalisation du cycle n'ont pas
été jugés : NON JOUÉS après le premier rouge. Aucun VERT cycle/shutdown durable/LIVE.

## Conservation avant diagnostic

PC ~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-v8-red-01 ; copie
locale même racine lifecycle-v8-red-pc-local-01.
103 fichiers référencés,17346531octets,104 avec manifeste ; réempreinte
locale complète taille/SHA zéro écart, aucun fichier supplémentaire.
SHA256 manifeste **497baaf330eaa519aa73c8e3488e812b8c8ba15da5de80227ecbb2020aea2d5d**,
copie Git lifecycle-v8-red-proof-manifest.json identique.
P/Q fichiers, scope/bruts et autorité explicitement assainie : jeton privé
non copié. DB/FTS copie et répertoires d'état restent sur PC hors archive,
sans nettoyage ; sources FIN/canonical conservées, pas de nouveau verdict
de leur intégrité après rouge faute d'inventaire après source capturé.

Ticket17 reste OUVERT ; C7 seul arbitre une éventuelle recette RO native
distincte après proposition et revue. Ce rapport ne modifie aucun exécutable.
