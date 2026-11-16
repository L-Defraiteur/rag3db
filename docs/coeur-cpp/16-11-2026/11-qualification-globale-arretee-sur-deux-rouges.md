# Qualification globale arrêtée sur deux rouges de concurrence

GO exact C7, après avis RO favorable Hotfixs : source/moteur `888c81825e4cd6556261233f0e238af3b95375c9`, livraison `15ab2f4815511400ac806314a3e964f2d7e723e0`, base `68590270e21d00285b29800bd9cdecf0cb921461`, pilote inchangé `0de1ed3fefff91a84dd1487ee2a819e1b713cef8`. **Verdict réel ROUGE ; aucune intégration ni lib posée.**

## Exécution et arrêt

Staging neuf hors worktree, quatre scripts et bundle vérifiés avant entrée. Vrai poste lourd : parent `run-p673637-i147432510.scope`, cpu.max `500000 100000`, POSTE_FILS=5, donc j5/CPUQuota500%, MemoryMax34359738368/MemorySwapMax0. Unité `cx-coeur-dictionary-global-01.service`, PID673654, invocation `0aa820c2837f40e9a945191c010f80c3`. Budget global10800s après acquisition ; aucun allongement ni réessai.

- Compilation : 1136 étapes terminées, code0.
- Offsets : 1 cas passé, BUILD_ID888 émis ; contrôle des identités accepté avant suite.
- Transactions : 393 passés sur395, seuls deux auxiliaires nominaux sautés et un désactivé historique non joué ; durée1112,946s. Pas de nouveau skip ni échec, identité acceptée avant concurrence.
- Comparateur known_red : code8, verdict complet après1273,718s, **pas de timeout**. Banc : 16 rouges,315 verts, deux rouges nouveaux ci-dessous.
- Chaînes dédiées, inventaire E2E, positifs E2E et DROP exact : **NON JOUÉS**, arrêt au premier verdict de phase rouge. Les45 exclusions de concurrence restent non jugées.

Le refus natif est conservé dans raw/parsing/first-failure.json ; le superviseur conserve ensuite le retour1 de l'unité. Fin2652,631s après acquisition : ActiveState=failed, SubState=failed, MainPID=0, ControlGroup vide, quiescent=true, aucun PID attribué restant. Poste rendu. Le résumé émis se termine ROUGE. Aucun service, SDK59, base de Lucie, GPU ou master modifié.

## Deux constats, pas deux causes démontrées

`StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops` : wrongRows=228 au lieu de0 aux trois contrôles after-automatic-checkpoints, after-checkpoint et after-reopen ; premières clés signalées c-896 à c-900. Le compteur mêle balayage et sondages par clé : **228 n'est pas un nombre démontré de lignes distinctes perdues**. [Ticket propre](../tickets/2026-11-16-le-controle-des-chaines-apres-des-add-et-drop-releve-228-anomalies.md). Aucun SIGSEGV ni attribution au correctif append ou au crash C1.

`CheckpointOpenReader.OpenReaderOnRelsIsNotWaitedFor` : échec [plan], checkpointEndedFirst=true alors qu'attendu false. Le checkpoint a fini avant la lecture censée l'observer. Cela ne prouve ni attente du lecteur ni perte d'instantané. [Ticket du plan du témoin](../tickets/2026-11-16-le-point-de-reprise-finit-avant-la-lecture-censee-l-observer.md).

## Preuves conservées

[Lecture et empreinte d'archive](preuves/global-01/lecture.json), archive recus-et-resultats.tar.gz dans le même dossier : recettes, inventaire brut, résumé réellement émis, admission/refus/quiescence, XML offsets/transactions, JSON et log natifs de concurrence, manifeste et header. Le paquet complet avec ELF et archive moteur reste aux deux postes, chemins dans lecture.json. **53 fichiers bruts et10 artefacts rehashés localement après rapatriement**, conformes aux empreintes du reçu distant ; aucun journal réécrit.

Le ciblé précédent5/5 ne devient pas une qualification globale. Ticket append et parent C1 restent ouverts. Pas de relance, correction d'oracle, reclassement probabiliste ni modification de known_red sans diagnostic et nouvelle recette coordonnée.

## Comparaison RO avec la base exacte

[Comparaison et empreintes des artefacts](preuves/global-01/comparaison-base.json) : known_red.txt, probabilistic.txt, long.txt et les deux fichiers des nouveaux rouges sont **identiques octet pour octet** entre base685 et source888. Aucun changement de noms, listes ou oracles dans ce lot. Cela n'est pas une exécution native de référence : aucune passe baseline n'a été lancée.

BUILD_ID conservé Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF ; LTOOFF. Environnement tests MAX_NUM_THREADS2, OMP/OPENBLAS5, quota global5cœurs. CheckpointOpenReader impose lui-même pool2Gio ; la valeur effective du pool hérité de StringRewrite n'est pas journalisée séparément. Le vert ASAN historique14nov ne porte donc pas la même configuration. Le log natif du rouge [plan] mesure CHECKPOINT29,5782ms, avec sleep30ms dans le témoin : compatible avec la fenêtre manquée, sans démontrer un défaut moteur. Le cas lent NoWriterWaitsLongDuringTheCreate a finalement terminé en695,757s ; ce n'était pas le motif d'arrêt.
