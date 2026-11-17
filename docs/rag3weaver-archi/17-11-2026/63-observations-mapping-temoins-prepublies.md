# Observations acquises : deux témoins prépubliés avant correction

Réserve Mémoire10f753 consignée au ticket AVANT ce diff. Le runnerb925 reste
INCHANGÉ. Les attendus privés et recheckposthash déjà corrigés sont conservés ;
ce point porte sur données observées renvoyées par les ports, pas les handles.

Deux vrais appels run_fixture_invocation/ports MODELED :

- Capturer le dictF renvoyé par seal_entry (SHA D), accepter le permisD, puis
  finish_fixture réemploie ce mapping et change son SHA enE. Le résultat doit
  conserver le SHA D ACQUIS, pas suivre un buffer externe désormais réutilisé.
- Capturer le dictW renvoyé par wait_child, puis finish_fixture modifie stdout
  en bytes différents. Le résultat doit garder le hash des bytes ACQUIS au wait.

Attendus sur b925 courant :2FAIL de comparaison de valeurs, pas des refus
artificiels ni erreurs de fixture. Aucun test exécuté. Les snapshots futurs
concernent les données (mappings/tuples/scalaires/bytes immuables), JAMAIS les
handles OwnedLaunch/HeldEntry/Seal/Permit : identité et ownership restent celles
du port. Capturer avant tout callback/check susceptible de réutiliser les vues.

Pins runner-observations-red-pins.json proposent UNE commande2noms sansfailfast/
TERM58KILL2 ;28autres nonselectionnées,30fonctions au total. Source/pins/RO puis
GO2rouges distinct/bruts2hôtes AVANTfix ; aucunGO28 reporté. Le mode resteMODELED,
pas de flux OS/CP/Common/SDK qualifié, pascleanupOSborné. Pur565b/archivesintacts.
