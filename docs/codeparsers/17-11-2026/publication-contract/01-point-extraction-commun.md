# Proposition : une transaction de publication commune, plusieurs plans ordonnés

État : **contrat à relire**, pas une extraction implémentée. Branche de préparation
cx-codeparsers-publication-contract issue deD45 ; outillageD45/P05 conservé
figé. Aucun CPU,SSH,/proc,signal,Cargo,natif,autorité ou calibration dans cette
préparation. Les36témoinsD45 verts et leur corroboration Banc restent des
preuves CPU du seul couple actuel, pas une qualification de cette proposition.

Demande Archi sous coordinationC7 : quatre publications ordonnées global/D6,
`global` avec5ELF puis `agent-red1`,`agent-green1`,`review1` avec1ELF chacun.
Ticket existant consommateur :docs/rag3weaver-archi/tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md.
Ticket fournisseur :docs/codeparsers/tickets/2026-11-17-la-publication-du-ledger-de-sonde-attend-un-ack-sans-relais-autonome.md.
Aucun GO natif ni admission du couple APPf1e8/PILOTE229b n'est transféré ici.

## Frontière à extraire

Le point d'extension est le **plan de publications** et le **contrat du ledger**,
à l'intérieur du même moteur ; pas un nouveau relais avec sa propre boucle.
Emplacement proposé pour l'implantation ultérieure :
`extension/rag3weaver/scripts/publication_relay.py`, à côté du confineur existant,
autonome dans rag3weaver. Ce fichier n'est pas créé par la présente proposition.
Les scripts Qcopient ce module unique avec son SHA dans leur manifeste de
contrôleurs ; pas de fallback vers une ancienne copie ou un module du PATH.

| D45 existant | Extraction proposée | Ce que l'adaptateur garde |
|---|---|---|
| Relay.cap/cleanup_cap/refresh | PhaseBudget + fenêtre courante | limites approuvées dans prepared |
| Relay.publish | transaction de fichiers déclarés + ACK | validation du sens du ledger |
| Relay.run | séquence de publications, premier refus, finish/reap/final | commande de phase approuvée |
| SSHTransport | transport borné et logs communs | entrée Gate/phase/prepared épinglées |
| Gate bootstrap/launcher/active/cancel/final_roles | PhaseLifecycle commun, rôles réels et arrêt | sources/roles déclarés et épinglés |
| Gate.stage | fournisseur de publication, sans copie/ACK propre | extraction Cargo/SDK/inventaires ou ledgerglobal/D6 |
| Gate.export/ack | porte transactionnelle commune autour du fournisseur | aucune échappatoire aux contrôles de phase |

Le noyau commun reste propriétaire de la copie, du hash, de fsync/link, de la
séquence, des délais, de first-red, de l'arrêt et du constat de quiescence.
Le fournisseur ne peut ni déclarer phaseactive par un booléen, ni exporter,
ni copier, ni écrire un ACK, ni exécuter du natif : il lit les pièces locales
et vérifie leur contrat métier sous le garde actif commun. Les deux côtés
revérifient la phase aux mêmes frontières qu'en D45, avant/après opération et
immédiatement avant le lien d'un ACK.

## Séparer le plan figé de la publication observée

`PublicationPlan` est ordonné, sans label répété, et hashé avant le lancement.
Chaque `StageSpec` déclare son label, le nom du ledger, les fichiers/roles
attendus, les identités logiques des exécutables et le contrat de provenance.
Les SHA des futurs contenus ne sont pas inventés à cette étape. Le nombre et
les identités attendus sont déterminés par le contrat approuvé, pas par ce que
le producteur trouve opportun d'envoyer.

`PublicationEvidence` apparaît quand le propriétaire réel a scellé son ledger :
phaseID/numéro de séquence/planSHA/preparedSHA/sourceBindingSHA, ledger JSON et
SHA de ses octets originaux, files{name:SHA,size}, ELFmap des identités logiques
vers les fichiers ou références antérieures. La porte valide cette observation
contre le StageSpec, puis la fige pour ce label. Une modification de ledger,
liste ou SHA pendant les copies refuse ; elle ne constitue pas une révision
transparente du plan. Les données ne sont exportées que sous un nom final
complet, fsync puis lien exclusif, jamais depuis un fichier encore en écriture.

Les fichiers sont référencés par IDs sûrs, sans chemin libre venant du pair.
La porte résout exclusivement les chemins réguliers, sans symlinks et aux UID
attendus dans la zone de publication approuvée. Le ledger et tous les fichiers
attendus doivent être présents ; fichier manquant ou supplémentaire refuse.
Pas d'export d'autorité/jeton, .env, .vault ou environnement complet.

ELFmap n'est plus un scalaireelf_sha256. Chaque entrée vise soit un fichier
exécutable de la publication courante, soit un fichier d'un label antérieur
**déjà acquitté**. Cette seconde forme conserve le comportement Rust runtime :
son ELFbuild est rehashé avant l'ACKruntime sans produire un faux inventaire
pour le client global. Une référence vers une étape future, inconnue ou non
acquittée refuse. Même SHA entre deux occurrences ne les fusionne pas : label,
identité et provenance restent distincts.

## Plans des deux consommateurs

| Plan | Label | Fichiers courants | ELFmap |
|---|---|---|---|
| Rust par défaut | build | ledger,elf | probe→build/elf |
| Rust par défaut | runtime | ledger,all,ignored | probe→référence build/elf déjàACK |
| global/D6 proposé | global | ledger +5fichiers ELF déclarés |5identités→fichiers courants |
| global/D6 proposé | agent-red1 | ledger +1fichier ELF déclaré |1identité→fichier courant |
| global/D6 proposé | agent-green1 | ledger +1fichier ELF déclaré |1identité→fichier courant |
| global/D6 proposé | review1 | ledger +1fichier ELF déclaré |1identité→fichier courant |

Les noms exacts, sources complètes, features/profils/SDK, emplacements et contrats
de ces huit occurrences global/D6 restent à fournir par Archi et à épingler
avant une admission. Aucune archive tar, aucun all/ignored factice ou nouveau
fichier artificiel n'est demandé pour remplir une API prévue pour Rust.
Le verdict métier d'une étape volontairement rouge appartient à son contrat
et à ses preuves ; le noyau ne blanchit pas un exit nonzero sous un nom de label.

Compatibilité : le wrapper Rust `Relay(...)` peut garder son défaut strict
build/runtime en construisant le plan ci-dessus et en appelant le module commun.
Le moteur commun exige un plan explicite et ne porte pas un défaut Rust caché.
Les champs historiques du ledger Rust restent lus et validés dans son adaptateur.
Pas de deuxième implémentation de run/publish/cancel à laisser dans ce wrapper.

## Contrat du transport et de la porte

L'interface de transport garde ready/start_phase/running/snapshot/fetch/
publish_ack/finish/abort/stop. Les signatures proposées sont dans
publication_contract.pyi ; ce fichier est une spécification, pas un module
importable livré. Les noms publics n'autorisent pas d'appeler une phase expirée.

- ready : identité réelle PID/start/UID, entrée script et tous modulesSHA,
  preparedSHA/planSHA ; superviseur physique réellement tenu et logs avant spawn.
- start_phase : watchdog distant couvrant aussi l'attente ; bootstrap réel puis
  descriptor après exec, rôle vivant contrôlé avant le préflight. Un ready seul
  ou un Popen qui a déjà rendu1 n'est jamais un producteur armé.
- snapshot : état et durées restantes calculés sur PC, publication scellée si
  présente. Files/ledger/ELFmap appartiennent à la phase et au prochain label.
- fetch : demander label+ID, pas un chemin. Revalider la phase avant/après ;
  écrire dans un fichier provisoire, fsync/rehash puis lien exclusif local.
- publish_ack : seulement après tous les fichiers et exécutables courants ou
  hérités rehashés sur le pair. ACK lié au label/index/phase/plan/prepared/
  ledgerSHA/fileSHA/ELFmap/acteur et chemins locaux de copie. La porte revalide
  l'état vivant, les SHA et l'END juste avant link. Aucun ACK posthume.
- finish/abort/stop : même réserve et mêmes preuves de récolte. Un timeout ou
  problème de transport garde son premier défaut ; il n'est pas requalifié en
  attente normale. Absence non observée et UNKNOWN ne valent pas quiescence.

Les méthodes provider de la porte sont strictement de lecture/validation et
bornées par le même appel. `PhaseLifecycle` commun garde les vérifications
obligatoires d'autorité réelle, rôles PID/start/UID/exe/argv/cgroup/scriptsSHA,
END, first-red, scope final et observation physique. La politique de sources et
capacités du consommateur reste déclarée/hashée, pas copiée d'une allowlist Rust.

## Deux niveaux d'échéance, jamais quatre budgets globaux

Le budget de phase/global et son hard parent sont fixés une fois avant ready.
Les durées PC sont ancrées au début local de chaque RPC ; le budget global ne
peut que diminuer. Une transition de label n'autorise aucune réinitialisation.
Les réserves d'arrêt et les limites par appel restent à l'intérieur des END
approuvés. L'API ne transporte jamais une horloge absolue PC à comparer au local.

Si Archi garde un délai90s par published, il doit être **déclaré** dans StageSpec,
activé par un événement du propriétaire réel et borné par l'ENDglobal restant.
La porte calcule sur PC la durée de cette fenêtre courante ; les deux extrémités
figent son END pour ce label. Après ACK, seule une nouvelle fenêtre de label
peut débuter, jamais un nouveau globalEND. À défaut de fenêtre spécifique,
le plan Rust garde le seul END opératoire existant. Une notification humaine
ou l'apparition tardive d'un ledger ne peut pas démarrer un nouveau90s.

Exemple : globalEND1000, fenêtre du premier label100→190, puis second label
350→440. Conserver190 comme deadline globale casserait la composition ; remonter
le globalEND lors du second label casserait la garantie. Il faut donc garder
séparés budgetglobal immuable et fenêtre courante, avec cap=MINdesdeux. Ce sont
des horaires d'une seule machine pour expliquer le modèle, pas une conversion
interhost ni une proposition d'allocation réelle.

Les valeurs Rust300/2098+2/5 ne sont pas imposées au pilote global7455. Les
limites exactes du consommateur, les rôles et leurs watchdogs doivent être relus
et épinglés ensemble. Cette proposition ne choisit aucun nouveau budget.

## Revue et migration avant code exécutable

Archi peut préparer son manifeste et son fournisseur sur ces types, sans boucle
de copie ou d'arrêt locale. Reste à valider ensemble : huit identités ELF et
leur provenance ; activation des fenêtres90s ; limites exactes du global ;
source/emplacement du noyau commun ; protocole de compatibilité Rust et pins.

L'extraction ultérieure devra déplacer effectivement la boucle et les gardes,
faire importer le même blob par les deux adaptateurs et conserver les36assertions
sémantiques D45 sur ce vrai noyau (seuls chemins de patch/import adaptés).
Nouveaux témoins avant code :4publications5+1+1+1, ordre/uniquité/missing/extra,
SHA mouvant, référence ELF héritée, absence d'ACK avant tous les rehash,
fenêtres90s sans reset global, ARM après vrai exec, expiration/UNKNOWN/stop.
Publication avant essais, budgetsCPU distincts, RO totale du noyau ET raccords,
nouveaux pins ; puis seulement allocations de calibration/native séparées.
Une spécification relue n'est pas cette implémentation ni ses résultats.

## Sérialisation et portée des types proposés

Le stub.pyi précise les types, pas leurs implémentations. Les instances doivent
être validées et gelées profondément : frozen=True ne suffit pas pour figer
un Mapping fourni par l'appelant. IDs uniques/sûrs, SHAhex64, tailles entières
non négatives, durées finies, références antérieures et exactitude des ensembles
sont des préconditions du futur validate_plan/validate_evidence. Zéro copie ou
ACK avant leur validation ; aucune valeurplaceholder n'est exécutable.

Le hash du plan porte sur sa représentation JSONcanonique explicitement versionnée
(UTF8, clés triées, séparateurs virgule/deuxpoints sans espaces, ensure_ascii=False,
allow_nan=False), pas sur le futur ledger. Le SHAledger porte sur ses octets
originaux ; sa valeur JSON décodée ne remplace jamais cette identité de contenu.
component_hashes inclut le noyau commun, les entrées d'adaptateur et tous modules
chargés : scriptSHA d'un wrapper n'est pas preuve du code commun qu'il importe.
L'identité ready lie l'entrée réellement exécutée et cette fermeture de modules.

Le contrat de phase et le fournisseur doivent être relus ensemble. Une interface
Python ne confine pas du code hostile aux mêmes droits Unix ; les scripts sont
ceux publiés et épinglés, les capacités/lectures sont celles du contrat approuvé.
Les contrôles effectifs restent obligatoires dans le noyau et ne se délèguent
pas à une simple déclaration du consommateur. Aucun schéma ici ne remplace
l'admission C7, les contrôles source/lib ni les preuves du vrai propriétaire.

PreparedContext référence explicitement prepared_path et son SHA : le consommateur
n'a pas à fabriquer un faux prepared.json Rust si son manifeste approuvé a un
autre nom. Les paramètres clock/sleep existent pour les témoins du vrai moteur ;
le vrai CLI impose les horloges monotones et watchdogs réellement observés,
sans flag permettant de choisir un modefake lors d'une admission.
