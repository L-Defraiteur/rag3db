# Producteur d'entrée : protocole, API et commandes proposés avant backend

Après suffisance C7d72aa02288 du pur892bc/51verts, préparation documentaire
concrète dans [execution-entry-native-protocol](execution-entry-native-protocol/README.md) :

- [API opérationnelle proposée .pyi](execution-entry-native-protocol/producer_api.pyi) ;
- [recette et argv INACTIFS, pins de préparation](execution-entry-native-protocol/native-recipe.inactive.json) ;
- [16 spécifications de témoins](execution-entry-native-protocol/native-witnesses.inactive.json), NON ÉCRITES EN TESTS/NON JOUÉES.

Déclaration avant lancement distincte de l'admission runtime du runner réel
avant fork. Parent naturel/TRACEME→EXEC réussi→FD de l'image réelle/copie stable/
seal→port CP unique/ACK lié→detach→vrai wait enfant. Le parent vivant porte la
fenêtre, pas l'enfant arrêté. Cas d'erreur/EOF/stop/absence/timeout explicités ;
images futures UNKNOWN, SDK séparé. Les sources qualifiées de capture/libération/
wait ont été relues et empreintées, pas rejouées ni transformées en preuve native
nouvelle. Le backend TRACEME reste ABSENT/non qualifié, pas GDB rc=programme.

Common existant possède le Popen dbus-run-session : racine réelle distincte du
runner en général. Son vrai wait/reap restent chez Common ; le coordinateur
extérieur lie les fichiers de clôture au scope/root admis. Lien absent/ambigu :
STOP, aucun waitpid d'un descendant libre. Ce raccord concret et l'adaptation
CP restent soumis à RO ; aucun descripteur Common magique n'est déclaré livré.

Première recette proposée : un printf ELF existant, SOURCE/HASH lus seulement,
argv exact/marker stdout/wait0, sous poste+Common ; soft120/KILL5 avant queue15,
Common floorMIN(90,reste−15), ENDop=CommonEND−15/réserve15. Budget et ressources
poste2G/swap0/CPU100% PROPOSÉS, pas réservés/accordés. Coûts/profil hôte UNKNOWN.
Le fichier montre les quatre argv (outer/poste/admission-Common/enfant) et le seul slot dynamique
ADMITTED_COMMON_BUDGET, calculé après vrai poste sans nouveau budget.

LIVE HOLD : gardes et adaptateur CP non qualifiés. Un éventuel éditeur de reçu
est FIXTURE_ONLY/cp_ack=false/global_admission=false, incompatible LIVE. Aucune
boucle CP/SSH parallèle. Le premier mécanisme natif fixture ne qualifierait donc
pas le vrai ACK CP, même si un futur GO le fait réussir.

Les composants futurs sont NULL/delivered=false et execution_ready=false :
execution_entry_runner.py, native_recipe.py et éditeurfixture N'EXISTENT PAS.
Recherche confirme aucun chevauchement d'écriture sur cette surface réservée.
Aucun P/Q/S runtime créé ; aucun code backend OS, appelant/Cargo runner/Common,
cible/service/SDK/native/proc/signal/test exécuté ou modifié.
Le validateur pur565b et les archives f74/2a/892bc restent intacts.

Demande RO du protocole/API/recette/pins et de la portée FIXTURE_ONLY avant
arbitrage C7. Si autorisation d'implémentation ensuite, témoins exécutables à
publier avant backend ; nouveaux sources/pins/RO puis GO distinct avant tout
CPU ou premier prototype natif. Aucune autorisation d'exécution dans ce lot.
