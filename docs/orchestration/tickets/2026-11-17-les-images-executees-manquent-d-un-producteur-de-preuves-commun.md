# Les images exécutées manquent d’un producteur de preuves commun

- **État** : ouvert, 17 novembre 2026 ; frontière documentaire relue, faisabilité et mécanisme physique non établis.
- **Famille** : produit
- **Gravité** : P2, bloque la qualification globale TTY ; aucun défaut d’ELF effectivement exécuté attribué.
- **Atteignable en service** : non établi ; cette demande concerne les passes de qualification attribuées.
- **Touche rag3weaver** : oui, ses pilotes et preuves de qualification, pas une fonctionnalité utilisateur livrée.
- **Pour** : orchestration ; rag3weaver-archi pour l’étude de faisabilité source seulement ; TTY consommateur, Codeparsers propriétaire du relais de publication.
- **Signalé par** : TTY, Mémoire, Codeparsers et Archi ; arbitrage orchestration après leurs revues documentaires.

## Constat et preuves

Ticket parent TTY : docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md.
Contrat documentaire961c873bb9d74408aa892d7d6799737047882650, note11 etJSON inactif : trois RO documentaires favorables M/CP/Archi, pas une faisabilité physique. Vingt spécifications P01–P20, NON ÉCRITES/NON JOUÉES ; neuf positions racines proposées, pas neuf processus observés ni couverture exhaustive des descendants.

Validateur hermétique fe01 : quatorze témoins verts sous GO distinct, preuves4a6ccde9e281690a97fd3f151f77749985a2a160/RO M ; ELF_PROVENANCE resteVIDE, Q4 execution_ready=false. Les copies/ACK du relais, PID/start et brackets périodiques229b ne prouvent pas à eux seuls chaque image réellement exécutée. PID/start n’identifie pas une génération d’exec ; tentative échouée, image remplacée, fin d’image et wait du processus sont distincts. SHA de l’image ne qualifie pas ses bibliothèques SDK dynamiques.

Codeparsers conserve seulement relais/ACK/Window. C9constante36verts+3rouges et8rougesWindow sont acquis ; capacité4/Windowenpréparation, aucun producteur physique livré ou attribué par ces résultats. Aucun pilote ne doit fabriquer le reçu manquant depuis le catalogue.

## Prochaine action autorisée : étude source et décision, pas implémentation

Archi retrace les primitives réellement existantes de construction des commandes, confinement/Common, publication et observations runtime. Comparer précisément ce qui se prouve par construction du runner de confiance et ce qui exige observation d’une image au vrai exec. Ne pas affirmer une observation PID/image lorsque la preuve est seulement constructive. Le canon2150 reste inchangé ou HOLD explicite.

Produire une proposition unique de raccord minimal commun, ou une impossibilité/lacune explicite : contenu/identité/occurrence/générationexec, rattachement au plan préautorisé, phase/source/inventaire/RAW, perte d’événement, descendants/couverture, rehash avant/après et copie de la bonne image, clocks propres aux hôtes, annulation/END/cleanup attribué. Les capacités SDK dynamiques restent une attestation distincte au PID réel. Ne pas proposer un observateur ad hoc TTY ni dupliquer la boucle du relais.

L’étude doit annoncer les coûts et effets : conserver un débogueur pendant les tests peut changer leurs délais (ancien cycle rouge sous GDB, causalité non démontrée). Proposer si possible une frontière d’observation avant les tests puis libération ; ne pas tenir la sonde active dans une mesure en la déclarant neutre sans preuve. Aucun chiffre de coût inventé. Pas d’accès hors sous-arbre de processus de qualification explicitement attribué, ni privilège/sandbox modifiés.

Référencer les API/source primaires et la surface minimale conditionnelle, les limites de garantie et les témoins à prépublier au vrai raccord. Si le contrat ne peut être tenu avec canon/confinement/bornes demandés, STOP et arbitrage C7 sur garantie/coût/surface avant tout code. TTY fournit le contrat consommateur ; Codeparsers garde le noyau publication ; rôle producteur et adaptation Common futurs restent conditionnés à cette revue.

## Interdictions de cette préparation

Aucun code ou prototype de producteur, réservation de chemins, test/CPU/processus de qualification, traceur, proc d’autres sessions, signal, build, Cargo, native, DB, provider, service, slot, SDK/cache partagé ou target modifié. Aucun GO3600, globalArchipasse, API physique ou liveness n’en découle. Aucune reprise de C1, aucun ancien GO/END réactivé.

## Fin du ticket

Un raccord commun effectivement qualifié par témoins publiés avant correction, admission distincte du vrai montage, artefacts physiques source/exec/générations/bornes/cleanup relus, et intégration dans le consommateur autorisé ; ou abandon explicite documenté remplacé par une garantie plus limitée acceptée. Une étude documentaire ou un seul validateur virtuel ne ferme pas ce ticket.


## Historique de l’étude avant arbitrage

## Étude source Archi après attribution, aucune implantation

Voir [étude source45](../../rag3weaver-archi/17-11-2026/45-etude-source-producteur-images-executees.md).
Proposition conditionnelle : un runner commun de confiance relierait l'objet
ELF/hash/copie stable au handoff réussi d'une racine déclarée, puis à son vrai
wait. Garantie limitée à l'image d'ENTRÉE, pas aux futurs exec/descendants ;
écart P17–P20 soumis explicitement à C7, aucune conformité20 déduite.
Surface proposée seulement : quatre sites e2e directs et mécanisme Cargo runner
vers le même point commun ; blob canon2150 inchangé ne signifie pas parcours
transitif inchangé. FD seul ne fige pas les bytes, handoff réussi reste à prouver.
Alternative ptrace event-only non exhaustive à ce stade : CLONE_UNTRACED/detach ;
aucune impossibilité universelle affirmée. SDK séparé, publication CP seule,
coûts/droits/compatibilité UNKNOWN ; STOP admission et code avant arbitrage.
Aucun prototype/test/trace/proc peer/signal/build/SDK/target/service/slot/réservation.
Ticket toujours ouvert : étude documentaire ≠ producteur qualifié ou abandon accepté.

## Arbitrage C7 après étude a5281ec4e4 et contrelectures

Rapport45 relu par C7 ; avis documentaires exacts TTY et Mémoire favorables à son bornage, pas à une capacité livrée. Le candidat ptrace événementiel persistant ne démontre pas sa couverture des descendants (`CLONE_UNTRACED`, détachement). Nous retenons pour le prochain lot une **invocation d’artefact déclaré avec confirmation de son image d’entrée**.

La preuve lie le catalogue/source/profil préautorisés, l’objet effectivement entré, une copie de son contenu, les arguments et le contexte, le PID/start de l’invocation, ses sinks RAW et son vrai wait. L’entrée doit être confirmée au vrai raccord : un FD, une intention d’appel ou un EOF de pipe ne constituent pas seuls cette confirmation. La stabilité des bytes jusqu’à cette confirmation reste à établir ; une barrière au premier exec réussi est une piste concrète, avant libération du programme.

Le résultat est celui de **l’invocation du programme lancé depuis cette entrée**. Son wait n’est pas présenté comme le code de retour d’une image A restée inchangée pendant toute sa vie ; les octets des sorties ne sont pas attribués individuellement à un PID écrivain. Les programmes et sources qualifiés sont déclarés et revus ; leurs sous-processus peuvent faire partie du comportement du test. Cette qualification n’est pas une preuve contre un programme arbitraire fabriquant ses propres sorties. Ressources et arrêt du sous-arbre restent portés par le confinement, indépendamment de l’exhaustivité d’une trace d’images.

### Changement explicite de contrat pour TTY

Les vingt spécifications de961 restent une proposition historique plus forte. **Le suivi exhaustif des futurs exec et descendants de P17–P20 ne sera pas une condition implicite de livraison du lot anglais.** Le consommateur doit publier une nouvelle version du contrat indiquant ce bornage. Aucun de ces quatre points n’est déclaré satisfait par la preuve d’entrée ; la couverture ultérieure reste UNKNOWN. L’observation positive d’une image d’entrée étrangère au plan refuse, et un B ne devient pas attendu depuis l’actual pour sauver la passe. Les garanties de P01–P16 applicables aux racines et aux copies restent à porter au vrai raccord, avec une table explicite de portée ; les spécifications non remplies ne sont pas comptées vertes.

L’observation exhaustive de toute génération, si un usage futur la requiert, devra faire l’objet d’un lot et de coûts explicites. Aucune implémentation ptrace permanente, seccomp, privilège, sandbox ou journal de tous les syscall n’est autorisée pour contourner le problème dans ce lot.

### Surface autorisée à préparer

Archi porte le contrat/API et les témoins du producteur partagé ; TTY reste consommateur et CP garde publication/ACK/Window. Préparer un module commun de validation sous scripts/, sans boucle de publication parallèle ni logique anglaise. Le mécanisme concret de confirmation doit venir avec son protocole précis avant implantation ; considérer les barrières de démarrage déjà utilisées (exec/main, libération avant tests, parent naturel et wait réel) comme sources réutilisables, jamais comme réactivation de leurs anciens pilotes, délais ou preuves.

Les quatre sites directs de run_e2e et le raccord Cargo runner sont acceptés comme **surface transitive proposée** du lot, à réserver avec Recherche avant de les modifier. Le SHA2150 conservé ne sera pas annoncé comme preuve d’un parcours inchangé. Chaque phase doit déclarer la propagation d’environnement (notamment LD modifié par Cargo), argv, PID du runner/enfant, sources/features, sorties et signaux. Les faits SDK chargés restent séparés au PID et au point réellement observés. Une racine non couverte n’est pas qualifiée par les autres ; auxiliaires et descendants non observés ne sont pas inventoriés fictivement.

Premier travail autorisé : publier un contrat exécutable/API minimale et des témoins hermétiques préalables au vrai futur raccord, avec tableau de portée et limites. Prévoir substitution de chemin/contenu, succès d’entrée à confirmer, échec/mort avant confirmation, copie/RAW/wait liés, erreur et disparition du producteur, END pendant copie/attente/finalisation, puis limite explicite A→B après libération. Des observations de processus sont des modèles dans cette première étape ; elles ne qualifient pas le noyau. Publier avant toute exécution, RO puis GO CPU distinct. Aucune implantation opérationnelle ou modification des quatre appelants avant cette revue de protocole/surface, aucun prototype OS/native/build/proc/signal/test par cette seule préparation. Un futur petit pilote réel et les passes des appelants auront leurs sources, pins, budget et GO propres.

Le ticket reste ouvert. Cet arbitrage choisit la sémantique d’invocation et ses limites ; il ne fournit encore ni API physique qualifiée ni GO global TTY/Archi, ni autorisation de master ou de retrait.

## Préparation Archi après arbitrage, avant implantation

[Étude45](../../rag3weaver-archi/17-11-2026/45-etude-source-producteur-images-executees.md)
publiée a5281ec4e4 ;
[rapport46 et contrat/API/témoins préalables](../../rag3weaver-archi/17-11-2026/46-contrat-entree-protocole-temoins-prealables.md).
Protocole proposé à RO : parent naturel, enfant de démarrage monothread, premier
exec réussi confirmé à une barrière, copie depuis l'objet réel et comparaison,
publication CP puis libération et vrai wait. Profil de stabilité des bytes à
qualifier ; pas de réception positive sur EOF/FD/intention seuls. Résultat de
l'invocation, pas image continue ou couverture future.

Trente et une fonctions de témoins de modèles écrites et prépubliées, NON JOUÉES ;
API .pyi seulement et module d'implémentation absent. Aucun rouge/vert revendiqué.
TTY44aev2 relu et aligné sur l'arbitrage ; réservation Recherche reçue sans
chevauchement déclaré pour le futur module commun et surface des appelants.
Aucune implantation/OS/proc/signal/native/test ni modification des appelants.
RO protocole puis décision C7 avant code et GO CPU distinct avant exécution.
Ticket reste ouvert ; aucune preuve physique ou autorisation de passe déduite.

## Réserves CP sur contrat377c94, consignées avant correction

RO source seule, aucun import/test/OS : le booléen ACK ne décrit pas une référence
CP liée aux faits exacts/phase/séquence/ledger ; les témoins doivent refuser les
ACK étrangers. La fin d'invocation produite par le runner ne peut pas attester
la clôture complète de Common, son ancêtre encore actif. Séparer enfant wait/RAW,
retour du runner, puis clôture Common attestée par le propriétaire extérieur.
Aucun défaut OS attribué. Corrections du contrat/témoins seulement, avant RO
neuve et GO CPU distinct ; pas d'implantation ou d'exécution par cette réserve.

## Réserve TTY377 sur borne inférieure, avant correction

Le plan ne porte pas encore start/issued Common ; ordre et END ne refusent pas
une séquence antérieure à l'allocation. Ajouter origine et émission de l'autorité
indépendante, dans le même domaine d'horloge, et témoins avant émission/fenêtre.
Réserve de contrat, pas défaut OS observé ; aucune exécution autorisée.

## Trois réserves Mémoire052, consignées ensemble avant correction

1. Le plan initial ne peut connaître SHA faits d'entrée/ACK futurs : séparer plan
immuable et attestation CP ultérieure indépendante, liée au plan et faits réels.
2. Le contrôleur extérieur n'est pas nécessairement parent naturel du runner
(Cargo ou shell le sont) : ne pas lui inventer waitpid(runner). Séparer wait
naturel de l'enfant d'invocation, wait réel de la racine Common et clôture externe.
3. Deux témoins de délais régressent dans le temps après ajout des faits finaux :
maintenir tous les horodatages suivants croissants pour isoler le refus délai.
Réserves documentaires/modèles ; aucun défaut OS ou test exécuté. Delta groupé
à publier et relire avant implantation/exécution distinctement autorisées.

Complément TTY/CP avant le même delta groupé : une erreur/signal du producteur
après enfant0 ne devient pas succès par quiet/reaped ; préserver le vrai rc
invocation dans les faits et la première cause d'infrastructure séparément.

## Réserve causale TTY0bae, avant correction

L'attestation CP acquise après entrée/copie ne peut être obligatoire lors d'une
mort avant publication. Autoriser l'absence explicite jusqu'à published ; garder
la première cause prépublication sans ACK inventé, et refuser published sans
attestation indépendante. Témoins à écrire avant implémentation, aucun CPU.

## Préparer la première implantation pure sans faux rouge d’import

Le contrat5cae3c31731dea3348be2c16fe96ed256fe567ff et ses47témoins ont troisRO exactes DOC/APIpure CP/M/TTY favorables ; aucun module runtime n’existe. Archi est autorisé à publier d’abord un module partagé **purement déclaratif/devalidation**, avec Refusal typée et une version initiale sûre de verify_transcript qui refuse explicitement «capacité non implémentée». Aucun lancement/OS ou appelant n’y est câblé.

Pour voir le nominal échouer comme comportement, le témoin nominal peut convertir SEULEMENT cette Refusal fonctionnelle en self.fail (en gardant toutes ses assertions positives). ImportError, erreurs de fixture et exceptions non contractuelles restent ERROR : jamais comptées rouges. Publier cette base et l’adaptation avant toute exécution ; pin du vrai module/import, RO puis GO du seul nominal distinct. Ce sera le rouge d’une APIpure nouvelle qui refuse encore, pas un faux défaut d’un observateur déjà installé.

Après ce rouge réel et conservé, implantation pure du contrat/conservation premièrecause/typesstricts/bornes/attestationCP séparée/fermetureCommon réelle permise en préparation. Publier et relire avant GO47verts distinct ; aucun kernel, fichier, clock, réseau, processus, signal ou publication réelle dans verify_transcript. Le backend, ses adaptateurs CP/Common, preuveshôte/profilstockage/handoff/SDK et les quatreappelants restent NONLIVRÉS et sans GO. Les bool/int ne doivent pas se confondre dans les champs typés. Aucun47vertsanticipé ni compétencephysique conférée par des faitsmodélisés.


## Préparation baseline pure après autorisation09726, avant tout CPU

Module partagé et nominal à publier : Refusal typée, verify_transcript refusant
explicitement capability-not-implemented, sans effet externe. Le seul nominal
convertit cette Refusal précise en self.fail ; import/fixture/exception autre
reste ERROR. Les assertions positives restent présentes. Pins et RO avant
GO nominal distinct ; aucun rouge acquis par cette préparation.

## Rouge nominal baseline2b04 acquis, preuves conservées avant implantation

[Rapport48](../../rag3weaver-archi/17-11-2026/48-preuve-rouge-nominal-baseline-2b04.md).
GO C7-ENTRY-PURE-BASELINE-2B04-17nov60/5e2dfd8bbe consommé une fois :1FAIL,
0ERROR/skip,rc1 sur refus explicite, enveloppe0.048796813s ;46autres NONJOUÉS.
Neuf bruts hors WT copiés et réhashés LOCAL/PC0écart, manifeste
d977e605e15ad599ccf9f1869e06305c171e013f85d615bc7ae60bceb94ebadb.
Aucune implantation pure complète avant conservation. Sa préparation seule peut
suivre09726 ; nouvelle source/pins/RO et GO47 distinct avant tests. Aucun backend
physique, adaptateur CP/Common, SDK/native/appelant autorisé par ce rouge.

## Implantation pure préparée après suffisance du rouge, non exécutée

[Rapport49](../../rag3weaver-archi/17-11-2026/49-implantation-pure-preparation-avant-47.md).
09726/f104 autorisent cette préparation après preuves f74 conservées2hôtes.
Le module partagé valide désormais les faits de MODÈLE et leurs types/bornes/
ordre/attestation distincte/firstcause/rc acquis, sans I/O ni clock/OS/publication.
47fonctions prépubliées inchangées depuis2b04, NONJOUÉES ; nouveaux pins/commande
proposée sansGO. RO source totale puis GO47 distinct avant tout test ; aucun
backend/adaptateur CP-Common/SDK/capture/appelant livré ou admission déduite.

## Réserve topologie Recherchecb290, avant témoins et correction

Contre-exemples statiques, NON JOUÉS : spawned peut réutiliser le PID du Common
owner ou common_root en transportant la même identité jusqu'au wait enfant ;
le plan peut aussi donner au même PID root/runner deux start différents.
Le module pur doit refuser ces topologies simultanées incohérentes, tout en
permettant root=runner avec identité exacte si le runner est enfant direct.
Prépublier quatre témoins avant correction ; aucun GO47 ne s'étend aux nouveaux
cas ni au futur pin. Cette réserve ne décrit pas un défaut OS observé.
