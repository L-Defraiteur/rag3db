# rc acquis et RAW malformé : témoin prépublié avant correction

Réserve TTY587785 consignée au ticket AVANT diff. Le runnercf3 est INCHANGÉ.
Le port wait_child renvoie exit/code7 valides avec stdout bytearray non admis ;
la copie récursive refuse actuellement ce RAW avant d'enregistrer le code7.
Le contrat conserve le rc authentiquement acquis, sans admettre le RAW malformé.

Un vrai appel run_fixture_invocation/ports MODELED : Refusalobservation-types
attendue, puis assertion invocation_returncode=7. Prévision sourcecf3 :1FAIL
«None != 7», NON OBSERVÉE ; pasERROR/import ni nouveau refus artificiel.
Le correctif devra isoler/capturer les métadonnées de wait avant snapshot des
RAW, sanscallback/checkclock entre acquisition/décodage/copie. Les handles ne
sont pas concernés, les snapshots des données et recheckposthash restent requis.

Pins runner-wait-rc-red-pins.json, UNE méthode-v-f58+2 ;30autresnonselectionnées,
31fonctions totales. RO puis GO1rouge distinct/bruts2hôtes AVANTfix. GO30HOLD ;
nouvelle source/pins/ROtotale/GO31distinct avantverts. Aucun test/import/parse,
OS/PQS/native/CP/Common/SDK, pur565b et toutes archives inchangés.
