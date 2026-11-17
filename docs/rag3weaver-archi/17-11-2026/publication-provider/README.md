# Provider de lecture en préparation, pas un acteur de publication

Sous accord C7 1cc2b726c3, neuf témoins prépubliés519839/a9d84 AVANT ce lecteur,
NONJOUÉS. provider_reader.Reader lit des JSON/fichiers privés réguliers,
phase/plan/binding/role/manifest/CargoJSON/features, octets ledger et lien à
l'admission uniqueBroker. Aucun transport/copieur/exporteur/ACK/horlogeadmission,
aucune commande Git/Cargo/native/service/SQL. Il ne remplace pas le noyau commun.
Un retour physical_admission=false est une pièce de LECTURE, jamais un grant,
ARM, capacité, hash ELF vérifié ou preuve que le producteur vit.

Le noyau commun SEUL doit vérifier l'autorité réelle et ses originauxEND,
les rôlesPID/start/UID/CG/exe/argv/scripts/modules, propriétaire vivant de la
fenêtre, ordre/exactitude du plan, rehash réel du contenu ELF sur deuxhôtes,
export/copie/fsync/link, ACK et fermeture. Le Reader conserve les attentes
sous bytes JSON canoniques privées ; une mutation ultérieure du mapping de
l'appelant ne choisit pas un nouveau contrat à partir de l'actual.

Global lit global-ledger.json et les cinq copies fixes <role>-elf. D6 attend
un événement de label fixe (agent-red1/agent-green1/review1) portant la seule
identityBroker sûre task-N-start. La règle broker_bound est préapprouvée :
<identity>-admission.json, <identity>-ledger.json et <identity>-elf, tous feuilles
régulières du root privé. Aucun glob/mtime/cheminpair ni fusion d'occurrences.
Ledger/sourcehead/source_state/patch doivent correspondre à l'admission. Le
contrat actif du juge rouge→vert et reviewer reste celui du Broker publié,
à revalider au raccord complet ; ce Reader n'exécute/parse pas du Rust.

Le dossier est une préparation de LECTURE DOMAINE, pas encore l'adaptateur
PublicationProvider.read→PublicationSource runtime : le noyau publication_relay
n'existe pas et le raccord propriétaire des fenêtres est en discussion avec
Codeparsers. StageHeader.window_event_sha ne suffit pas à lui seul : le
Lifecycle doit obtenir le chemin d'événement préapprouvé, le relire et valider
le vrai owner+CommonEND/ENDop. Le provider ne peut retourner active=True pour
contourner cette vérification. Ne pas écrire une fausse implémentation du noyau
pour faire passer les tests du bridge ; ceux-ci doivent viser le vrai noyau
quand Codeparsers l'aura publié. Pas d'importfallback vers D45 ou Q229b.

40-provider-reader-config-propose.json est NONEXÉCUTABLE/run_authorized=false,
champs runtime inconnus=null, non accepté par Reader. Les quatre stages, rôles,
bindings et règles de chemin y sont concrets ; un futur prepare doit fournir
un NOUVEAU pin source/contrôleur/commonClosure/plan/execution/autorité/pinsPC,
à soumettre à RO. Ne pas remplir ces champs en observant une ancienne phase.
Fenêtre90 appartient à l'owner et est bornée par ENDop ; Reader vérifie sa
cohérence ARITHMÉTIQUE seulement. Le noyau impose l'heure réelle et la présence
physique. Aucun temps absolu n'est comparé entre hôtes.

Bornes C7 :7440TERM+15KILL avantqueue/ARM, queueMAX120DANSwhole ;
CommonMIN(7200,restephysique−30), ENDop=CommonEND−15, réservearrêt15DANSCommon.
Chaque fenêtre90 commence chez owner AVANTwait, MINENDop ; aucun resetGLOBAL.
Le code229b n'a PAS été modifié pour ces nouvelles obligations : ses ROetQ
restent figés, aucun nouveau pin/admissionnative n'est revendiqué.

À qualifier après publication : les neuf témoins reader à TempDirs/JSON réels
sans transport/processus, puis les raccords vraiscommonGate/ownerEvent,
quatrepub8ELF/ordre/redgreenprovenance et fenêtre90 avantwait. Aucun import,
CPU, /proc, SSH, signal, poste, ELF, exportSDK ou accès service n'a été exécuté
pour cette préparation. L'absence de transportPG qualifié reste un HOLD avantARM,
pas une autorisation NoEmbedding ni une lease fabriquée.
