# Le cycle démarre avant l’armement effectif du producteur de grant

- **État** : ouvert, 17 novembre 2026 ; cycle privé3c2 refusé avant health et writer, défaut de coordination à corriger avant toute reprise.
- **Famille** : produit
- **Gravité** : P1 de qualification et coordination ; aucune dette de donnée originale attribuée à cet incident.
- **Atteignable en service** : non, contrôleur de qualification sur copie privée et son producteur de grant.
- **Touche rag3weaver** : oui, indirectement par les pilotes et preuves ; aucun correctif Rust demandé ici.
- **Pour** : orchestration, Arbre et Embar.
- **Signalé par** : orchestration, retours concordants Arbre et Embar du cycle privé3c2.

## Faits, sur la même horloge PC

Allocation C7-LIFECYCLE-3C2-17nov900, source3c2a93ca38ba5729b4414a5d813467f916cbf905/Qv4prepared69c5480da4bc212f4e435a4419c0a56ebb3b26b041e3ca7ade47dd37336aa68f, nouveauPcopy-v4. Aucun ancien délai n’a été réactivé.

READYnoncea93c7255b4d945148b6d91c89be3fb51/SHA946e93c74f2750e5e4b6e6d9f84d01c205d6851e65767e5175be4f8a68b475b4 : observation1246927.830043659, grantdue1247047.830043659, END1247698.195278915. Marge owner30 : dernière entrée possible1247017.830043659.

WatcherPID2000831 armé seulement1247046.19743902, après cette entrée maximale. Il refuse avant publish_observed_grant et avant GET ; resultREFUSED_NO_RETRY/grant_presentfalse. Son annonce « veille active » était incorrecte : poll=1 était déjà observé. Embar la corrige explicitement et préserve ses traces.

Pilote : firstred1247047.917505952 fresh lifecycle provider coordination deadline expired. Grant, provider-before.transport et writer-process absents ; composéRED, scope arrêté/nettoyage à réattester. Seuls les précontrôles sur la nouvelle copie avaient tourné, aucun writer ni contact provider des deux acteurs. Aucun retard réseau ou transport n’est attribué à partir des messages.

L’orchestration avait reçu une disponibilité annoncée, puis lancé avant preuve de veille effectivement armée. Cette admission était insuffisante ; les rappels de message n’ont pas remplacé un processus vivant. Le contrôle de phase a correctement empêché le grant tardif et le writer.

## Conservation

Arbre conserve P/Q/Sv4, copie et sorties readonly, firstred immuable, scopes et quiescence/sourcehash avant toute correction. Embar conserve ~/.cache/embarquement-notes/lifecycle-3c2-watch-v4/{armed,result,expected-pins,expected-provenance}.json, watch.py et watch.log. Copies et manifestes sur deux postes avant tout fix. SourceFIN immuable et originaux restent hors écriture.

## Correction à préparer avant une nouvelle admission

Un contrôleur coordonné sur PC arme le watcher à la politique Embar préépinglée AVANT les précontrôles copie/DB et AVANT READY. La disponibilité d’une session ne compte pas comme armement. Un descripteur ARMED complet est publié après le vrai exec, puis lié au PID/start/UID/exe/argv/scriptSHA/cgroup et à l’allocation/source attendus. Le consommateur vérifie le processus encore vivant, pas uniquement un fichier ou Popen sans contrôle de poll.

La publication et consommation du grant n’ont plus besoin d’un ping humain. Avant READY puis chaque action, la liveness/phase est recontrôlée ; disparition/refus du watcher déclenche immédiatement un premierrouge, sans attendre120s. Le helper owner strict reste identique et s’exécute seulement après READY valide, sans health préalable. Le même watchdog physique borne attente, watcher, worker santé et contrôleur, avec cleanup attribué et borné. Pas de nouveau mode d’acceptation tardive ou de délai prolongé.

Témoins aux vrais raccords à publier avant code : watcher jamais armé, vivant annoncé mais exited, retard avant exec/descripteur avec mauvais argv, watcher terminé après ARMED, READY arrivé avant ou après refus ; aucune action provider/writer avant admission complète. Les contrôles deadline/nonce/pins/typedhealth/firstred et noembedding restent stricts. Aucun process réel, réseau, DB ou CLI dans ces témoins hermétiques.

Après source/tests publiés et RO totale du couple/pin, nouveau GO correctif distinct uniquement. L’ancien3c2 reste ROUGE ; aucune reprise n’est autorisée par ce ticket. Pas de migration LIVE, redémarrage de service ou actor de remplacement non attribué.

## Réserve finale sur la publication du composé — 17 novembre

ED612c778aca relève sur source2dccc070f1564d8ca9d22a59fcb2b73222f415e5/Qv5e3741c89 : lifecycle.compose contrôle END puis appelle qualify.save, qui ne contrôle END qu'avant JSON/flush/fsync. C7 confirme en lecture ces deux vrais raccords. Une publication franchissant END peut donc laisser un composé favorable et rc0 après la borne Common830 ; l'enveloppe OS900 ne prouve pas le respect de cette borne interne. Aucun franchissement runtime n'est déclaré : le couple n'a pas été lancé.

HOLD de ce couple avant tout GO. Même ticket et ancien paquet conservés ; publier un témoin aux vraies instructions finales avant correction, puis rendre cohérents publication, horloge finale et code de retour. Nouveau source/paquet et revue totale avant nouvelle admission. Les contrôles d'armement, liveness et action sont favorables en lecture mais ne lèvent pas cette réserve finale. Aucune DB/provider/CLI/autorité supplémentaire admise.


Revue des cinq témoins33ae49f0cc/2d948 avant CPU : C7 et ED1320e637c8 constatent le même ERROR prévisible pour END déjà passé (q.save ouvre puis lève ValueError hors bloc extrait), et late-link ne traverse aucun link dans le code ancien. Pas de GO des cinq sous une qualification fictive de contre-exemples équivalents. Correction HARNAIS seule à prépublier pour capter l'exception réelle et juger explicitement rc/état/fichier ; late-link reste exigence de la future branche atomique, pas observation d'un lien ancien. Vrai fsynclate et premierrefus restent les cas actuels discriminants. Produit2d/Q5 figés, aucun test admis avant nouveau pin et GO distinct.


Harnais seul d31d3e57496945fdd13637bd6b933270240dc823 publie la capture de ValueError/rc/octets/JSON et annote late-link comme contrat futur ; ED20099 favorable, C7 relit le diff. **GO C7-CYCLE-FINAL-RED-D31-17nov60** se limite au SEUL cas actuel test_actual_fsync_crossing_common_end_cannot_publish_positive_and_return_zero, une invocation sous OS59TERM+KILL1, attendu1FAIL sur rc0 après vrai fsync à horloge100.1>=END100,0ERROR ; PASSinattendu/ERRORSTOP0retry. Les quatreautrescas ne sont pas exécutés sous ce GO. Exécutable2d/Q5 inchangés, aucun correctif avant conservation du vrai rouge, pas de DB/CLI/provider/proc/signalréelparfixture/native/autorité/slot. Bruts/source/rc/temps horsWT et copiesdeuxhôtes avantfix.


Correctif LOCAL6715c17ed98ca708edeaecfe068d9eb881b6f722 prépublié après vrai rougeD31 ; EDc5d05 favorable statique, C7 lit le diff entier. Finaliseur réutilise write_frame, contrôle END/firstred avant et après fsync/link/cleanup, reclassifie seulement son propre cadre positif complet en RED/rc1 ; q.save et primitifs communs inchangés. **GO C7-CYCLE-FINAL-GREEN-6715-17nov60** : une invocation des cinq méthodes test_lifecycle_finalization sous timeout59TERM+KILL1, python3-B-m unittest-v-f, même cwd et deuxPYTHONPATHcoord. Attendu5PASS/0FAIL/0ERROR/0skip, premierécartSTOP0retry. Bruts/source/rc/temps horsWT/deuxhôtes ; aucune DB/provider/CLI/processus ou signalréelparfixture/autorité/slot. Un vert permettra seulement nouveaupaquetQv6 metadata+ROtotale, jamais GO900 implicite. AncienQv5/2d et rougeD31 intacts.


ROtotale6715/Qv6 : Recherche trouve une seconde P1 statique d'entrée, indépendante du finaliseur vert. compose exécute python -B confiner_validation.py lancer ; Common6439 exige argv[1]=script/argv[2]=lancer, donc ce -B refuse l'autorité du contrôleur avantARM/DB. Les ROtotales favorables ED765 et précédenteC7 ne lèvent pas ce défaut désormais identifié. Qv6figé/pasGO900. Même ticket consigné avantfix, témoin b2bd689bf5 prépublié : vraie assignargvcompose contre vraie signatureCommonAST, fichiersprocprivésfictifs et contrôlepositifsans-B. RORecherche harnais favorable, C7lit les26lignes. **GO C7-CYCLE-COMMON-RED-B2BD-17nov60** : une invocation test_common_entry_signature sous OS59TERM+KILL1/mêmesPYTHONPATH/cwd, attendu1FAILargvactuel aprèspositif/0ERROR. ERRORouPASSinattenduSTOP0retry, bruts/source/temps/rcdeuxhôtes avantfix ; aucunproc/processréelparfixture/DB/provider/CLI/slot. Ensuite seulement préparer suppression-B devantCommon+envbytecode, conserver -B des rôles contrôleur/driver/actor, nouveaupin/RO/GOdistinct avant verts ou natif.
