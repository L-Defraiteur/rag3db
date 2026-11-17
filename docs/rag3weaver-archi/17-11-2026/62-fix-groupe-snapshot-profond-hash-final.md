# Correctif groupé snapshot profond/hash final, avant GO28

Les réserves M/R5e3b ont leurs quatre rouges réellement observés, puis neuf
bruts réhashés LOCAL/PC AVANT correction : preuveb3ab49f8b7fe1b3ce5b41b727aaa8d2f988ae92c,
manifeste60e1b1e219030c49d5980f6643f133512a4d02bcf2b5379d1d547fa9a8fef866.
Une invocation4FAIL «RunnerRefusal not raised»/0ERRORskip/rc1 ;24autresnonjoués.
Le rouge de baselineb40 et les archives pures restent figés, aucun repeint.

Le seul correctif de logique du runner :

- snapshot récursif privé du request avant validation/ports : mappings figés,
  tuples/scalaires immuables, types étrangers refusés sans effet ;
- vue récursive DÉTACHÉE et mutable remise à prepare_owned_launch. Le port peut
  modifier sa copie sans TypeError ni modification du request original ou des
  attendus internes (y compris ENV imbriqué) ;
- construire le résultat complet et ses deux SHA, puis contrôler END frais
  APRÈS les hashes, juste avant return. Le rc enfant déjà acquis reste conservé
  en cas de refus tardif, avec première cause et cleanup secondaire.

Les28fonctions de témoins sont BYTEidentiques à la prépublicationdeee (243cf...),
NONJOUÉES sur ce correctif. Aucun GO24/25/3cas reporté ; les pins proposent UNE
suite-v-f/TERM58KILL2, source réelle/pur565b/test/Python/cwd/PYTHONPATH fermés,
executed=false/go=null. [Pins proposés](execution-entry-native-protocol/runner-effects-28-pins.json).
Demande RO totale source/pins AVANT GO28 distinct et toute exécution.

MODELED uniquement : flags cp_ack/global_admission restent false, LIVE/Common
réel/CLI natif restent unsupported. Les prédicats du pur565b réutilisés restent
BYTEinchangés ; ses51verts ne sont pas rejoués. Les callbacks sont contrôlés
avant/après, PAS interrompus ni nettoyés sous une borne OS démontrée ; ce port
et ses témoins devront être qualifiés avant natif. Aucun syscalls/proc/signal,
backendOS/PQS/SDK/appelant/Cargo/service/target/privilège/sandbox ajouté ou joué.
