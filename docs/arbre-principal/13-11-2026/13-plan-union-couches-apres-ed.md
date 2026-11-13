# Union2a après ED : rebase neuf et recette réglementaire bornée

Plan actualisé après décision C7 F8+liblarge ; la première proposition
ciblée seule1200s est remplacée et n’a donné lieu à aucun run.
Ce document n’autorise pas d’exécution ni de push master.

## Base et frontières relues

LandingED confirmé4e2e2bfbb7c8ce670a49a06a36580d481d9abbc0, baseba031.
Source qualifiéeED461e8b868f40907bb0b2f731ff2fcaebc78923d2 : produit identique
au candidat finalED selon revue/owner. Contrôle Git indépendant du candidat
b6974800141c35ee0fa69360051c16785a91e74c : UNIQUE entrant Rust
extension/rag3weaver/src/daemon_host/atelier_fichiers.rs,99ajouts/1retrait :
propagation backend/usages contextualisée avant absence/unicité,4témoins.
Aucun publication/layers/physical_index/Cargo/build/template/gitlink modifié.
Le fetch de la base réellement atterrie et cet audit seront refaits avant
gel de l’union ; tout nouvel entrant est inspecté, pas de rebase silencieux.

Le résolveur2a ajoute un module publication::layers pur, jamais appelé par
read_scope/outils aujourd’hui. Son adaptateur sous feature native est une
référence QualifiedPhysicalState, sans lecture/writeDB. daemon_host est
compilé sous daemon+native, pas sous le minimum. Les écritures publiques,
L6/top-k/rendus/persistance/branches restent hors2a et inchangées.
Cette disjonction n’est PAS une dispense de passe : l’arrivée Rust impose
F8 canonique et lib entière jeu large sur union, selon arbitrageC7.

## Rebase et conservation

Paquet à conserver72db9b21f65d7bfa6430eb10740cf77d2a17e9d2 +ce plan.
Créer branche/WT NEUFS cx-arbre-couches-union-r1/cx-wt-couches-union depuis
ce paquet, rebase sur4e2e après fetch réel, publier sans force. App69 et
piloteabaf restent figés, leurs résultats CPU ne changent pas de SHA.
Range-diff des patches2a, mode+OID+gitlinks ; layers.rs/tests byte-identiques
à69 ; pubmod ajouté une fois ; entrantatelier_fichiers byte-identiqueED ;
tous apports master hors chevauchements et suppressions préservés.
Conflit produit : STOP et revue explicite, pas de choix automatique.

Target session cx-target-contenu-derive PRIVÉ conservé :1751191963octets,
qualification69 terminée214.675s/3000. Preuves67/ELF sont hors target et
rehasées deuxpostes. Avant réemploi : source de provenance69, reçu terminé,
quiescence et chemin réel nonlié vérifiés. Aucun partage targetED, aucun
effacement/reprise d’ancien reçu. Piloteunion en WT séparé, copies horsW,
preuves/scope/contrôle v2 NEUFS. SHAappunion/pilote exacts à publier avantGO.

## Coûts disponibles et borne proposée

69 minimal à froid : build118.125s, checknatif47.704s,4exactssous1s et
publication0.054s ; cible1.75Go. Ils ne mesurent pas le coût nouveau F8/large.
Le cache contient les dépendances minimum/native mais pas forcément Burn/
OCR/code/daemon/PG ni toutes variantes8jeux. Nouveau chemin de crate peut
réinvalider des artefacts : aucune promesse de fraîcheur ni perf.

Proposition RÉVISÉE à arbitrer C7 :4800s UNIQUES après vrai poste lourd
32GiB/HIGHmax/jobs8/quatreCPU. Caps : F8global2100 (chaque jeu900 sous
borne globale), buildliblarge900, buildCPUminimal480, liblarge600,
chaqueexact60, publicationCPU120. Tous MIN(cap,échéance−15), ledger/pause/
finaliseur inclus ; aucune prolongation/reset. Manque : NONQUALIFIÉ.
Ce budget est une proposition, pas GO ni durée déjà consommée.

## Ordre des vérifications à figer dans le pilote

1. Ping managerbus avant poste/sanitize, puis confiner commun/busprivé
   unique. Env listeblanche, SDK29+pool1G pour F8/large seulement ; CPU sans
   SDK. Exactitude des deux WTs/sha/copies, cacheprovenance/du32, cgroup et
   ledgerinitial des scripts/source avant les vérifications F8.
2. scripts/check_features.sh CANONIQUE inchangé :8/8 +ses contrôles tickets/
   état, CHECK_FEATURES_LOGS privé, exportmagasin exactement UNE fois par
   le script (pas secondEXPORT du driver). Premier ROUGE/rc stop.
   Jeu8 rag3db-native utilise cargo check --tests --no-default-features ;
   il couvre déjà l’adaptateur minimal/lib, donc aucun cargo check --lib
   natif supplémentaire sans nouvelle preuve distincte à demander.
3. cargo test --lib --no-default-features --features JEU_LARGE --no-run
   --message-format=json-render-diagnostics, puis même commande sans
   features pour ELFCPU. Deux artefactsJSON réels/lib/profile.test, chacun
   sourceunion.find, features exactes, copies/SHA hors target.
   JEU_LARGE=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,
   openai-llm,anthropic-llm. SDK29/pool1G export une fois pour buildlarge.
4. Inventaires AVANTruntime : liblarge entière +ignorés (4ED et4layers
   exigés ainsi que juges physiques/régression existants), CPU4layers et
   publicationCPU10/0ignored. Nouveau ledger2ELF+source/features/inventaires/
   SDK/copierSHA/deadline publié horssource/deuxpostes avant marqueur v2.
5. Lib LARGE ENTIÈRE,8threads et --show-output : statut FAILED complet
   surveillé en activité. Ne pas appliquer à cette lib la règle panic brute
   du sous-module pur : des should_panic/catch légitimes peuvent y exister.
   Pas de filtre qui escamote4ED/4layers ni helpers/ignorés comptés juges.
   Les fixtures natives temporaires propres de ses unitaires sont admises
   comme partie de cette passe, aucune base existante/PGservice/activation.
6. Sur ELFCPU union features[] :4exacts mêmesnoms, premier gdbmain capture
   réelle puiscontinue MÊMEappel ; modulepublication10 qui réjuge ces4.
   Ciblé :10distincts/14bruts ; pas8nouveaux juges ni somme avec large sans
   distinguer les recouvrements. first_red3formes/nocapture pour ce périmètre
   sansshould_panic/catch. Aucun runtimeSDK exigé pourCPU.
7. Sources/ELFs/SHA/profils/maps réels natifsSDK29 pour large, memory/peak/
   events/du32/filiationPIDstart/UNKNOWN borné lot, quiescence et finaliseur
   après scan/hashes ET juste avant publication recontrôle END/elapsed.

## Portée des résultats futurs

F8 juge cohérence de compilation des8jeux et checkadaptateur ; liblarge
juge nonrégression commune et les nouvelles frontièresED+2a dans l’union ;
4exact+publicationCPU jugent le résolveur et la publication sans soutien
natif implicite. Ce ne sont pas des preuves de filtres avanttop-k/L6/outils
ou couches persistées. La lib entière rejoue ses unitaires natifs existants
sous nouvelle source ; aucun ancien scriptP14/rouge/global physique lancé.

Selon décisionC7 : pas de NAV, lectures/BGE, CLIpreflight, e2ePG ou4TX, ni
ancienne globale physique, faute de changement de ces chemins de ce lot.
Leur non-rejeu sera explicite, pas des verts inventés. Aucune basePG à créer,
pas de service/model/GPU demandé (FDdevice≠calcul et UNKNOWN≠absenceGPU).

Exécution attend RO+GOexact appunion/piloteunion et créneau réel. Fin : copies
preuves/superviseur/pilote SHA DEUXpostes, revue puis éventuel tourC7/dernier
fetch/postepush/ifVERT/3contrôles. Tickets2a+parent restent ouverts avant
intégration ; aucun retrait du cache/proofs encore autorisé.
