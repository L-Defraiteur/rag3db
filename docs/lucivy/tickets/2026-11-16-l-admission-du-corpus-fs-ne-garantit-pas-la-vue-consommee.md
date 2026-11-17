# L’admission du corpus FS ne garantit pas la vue consommée

- **État** : ouvert (16 novembre 2026), découvert pendant la reprise native après les neuf juges mémoire.
- **Gravité** : moyenne ; une réussite de test peut masquer une vue vide, des lectures perdues ou un index réutilisé.
- **Famille** : validation
- **Pour** : lucivy
- **Atteignable en service** : non ; collecteurs et builders des tests de corpus.
- **Touche rag3weaver** : non directement.

## Parents et preuves

Parent qualification core : [profil core et diagnostics non isolés](2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md).
Parent lot et suivi : [indexed:false](2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md).
Les neuf juges mémoire de1ee5/6c701 sont verts et relus (preuves9d6451f721,
RO8040fe84a9) ; ce ticket ne les ferme pas et ne les rebaptise pas admission FS.
L’ancien correctif du corpus absent traitait l’absence de racine, pas ce nouveau
contrat de vue/erreurs/panel avant builder ; aucun ticket clos n’est rouvert.

Source1ee5, audit CODE+archives seulement, zéro contact corpus/ELF/Cargo :
- derived_in_ram lit le préfixe300 trié Linux2.6.0 fixé, historiquement sans
  mutex_lock ; son gate11 refuse avant builder. Jeu fixé NONadmissible pour
  parité, pas de nouveau run de refus prédit ni de sélection posthoc.
- v3_ground_truth_contains collecte au plus5000 fichiers dans read_dir nontrié,
  ignore erreurs d’énumération/métadonnées/lecture/UTF8 et peut retourner avec
  une vue vide. Arbre clone8507 empreinté mais vue ordonnée5000 nonobservée.
- Son builder est appelé AVANT les15requêtes du panel (distinct des11derived).
  run_panel peut compter0=0 commeOK. Ses oracles relisent le FS et utilisent
  skip/lossy/unwrap_or_default. Les octets comparés ne sont pas garantis être
  les mêmes que ceux indexés.
- V3_INDEX_DIR peut rouvrir un index avec une clé faible chemin/compte/options,
  ou supprimer/persister à un chemin fourni. Ce chemin est exclu du futur
  parcours de correction ; aucune suppression/reutilisation pour faire vert.

Voir recette/contrat16 et pins historiques sous
`../16-11-2026/preuves/native-preparation-1ee5/`. Aucune observation FS récente
ou vueeffective n’est inventée pour compléter ces pins.

## Contrat demandé par C7 avant correction substantielle

Vue ordonnée explicitement figée, identités/ordinals et empreintes de contenus
liés à une origine admise ; chaque document sélectionné lu UNE fois, erreur IO,
empreinte et UTF8 fatales. Un snapshot immuable sert au panel ET au builder ET
aux comparaisons ; aucun oracle de correction ne rouvre une racine mutable.
Refuser la vue vide et toute requête noncouverte AVANT scratch/index/report.
Le panel15 reste celui du cas natif, sans l’échanger contre11 ou modifier des
requêtes après résultat. Builder neuf sans capacité de reuse/persist/delete,
V3_INDEX_DIR et modes qui relâchent les assertions refusés avant effets.

Préparer quatre témoins red-first : vue vide, lecture échouée, UTF8 invalide,
panel noncouvert/0=0. Ils doivent atteindre le vrai raccord FS du consommateur,
avec backend stdfs pour le natif et spy déléguant les mêmes opérations pour les
petits fixtures/spies ; pas un helper isolé testé à la place du chemin réel.
Ordre figé, read-once, zéro builder avant admission et zéro appel reuse/delete
sont observables dans les mêmes témoins et dans un contrôle positif distinct.

## Travail et fin

Porteur lucivy ; RO Recherche sur contrat/raccord/spies avant implémentation
substantielle. Préparation seulement : aucun contact corpus, aucun ELF/Cargo,
service, Linux7.2 substitué, sourcefix substantiel, tag/main/PR par ce ticket.
Après RO, extraction de raccord préservant la baseline puis publication des
quatre témoins et vrai rouge avant correction, selon nouveau GO borné.
Fin : erreurs/absence nonconfondues au vrai consommateur, preuves des quatre
rouges puis verts, snapshot/panel admis avant builder et aucun effet persistant,
puis recette native distincte avec nouvelle source/ELF si le code a changé.
Les ELFs1ee compilés restent des pins historiques, pas une validation future.
Core complet,9HOLD/FS/ULP7.2/PR42 restent nonqualifiés jusqu’à preuve adaptée.

LivrablePREPA16+pinsA/Bséparés+contratJSON4vacuïtés préparés : toutmetadata
historique,aucunRootcorpus/ELF/contactPC/proc/Cargo/testRust/service. Action
RORecherche duCONTRAT/raccordconsommateur/StdFsSpy/4vraisrougesAVANTimpl
substantielle. Les signatures du plan sont proposéesNONimplémentées, pasAPI
existantefactice. C7 tranchetransportmanifest/politiquecaptureavantcode ;
aucunrunArefusprédit,source1eequalifiée9mémoireseulement resteinchangée.

RORecherche CONTRATexact355ffadd01 favorablebornée reçue sansblocagestatique :
vraiStdFs/Spyentréeavantbuild/report,4rougesbaseline+positif/panel15/readonce
mêmetamponcohérents. Spieseffetsréelspourreuse/delete/persist, flagsV3refusés
avantlecture mêmevide/0. AucunRustdiff/test/Cargo/ELF/proc/corpus/service.
Choixcapture/transportmanifestC7resteAVANTimplémentation/GO ; proposition
captureuniquepolitiqueactuellepubliéeavantbuilder,aucunereselectionaprèsrefus,
manifestedereplayensuite ; pastri5000silencieux. TransmisC7,pasFSqualifiée.

ROsecondaireBanc ccd9ebcf86 contrat355ff favorableprépa/4SHAconformes,
baseline1eeconfirméeparGit, pasbloqueursupplémentaire. SouscasIOenum/meta/read
distinctssirevendiqués, preuveREFUSEDadmise≠reportindexavantgate. Aucunimpl/
témoinCPU/Cargo/ELF/proc/corpusmêmemetadata/service. Les2ROcontratfavorables,
choixcapture/transportC7avantcodeetGOtoujoursrequis ;source1ee/neufmémoire/
core9HOLD/7.2ULPPR42inchangés.

Arbitrage C7 reçu17nov : CAPTUREINITIALEUNIQUEStdFs, ordreEFFECTIF/politique
versionnée/cap5000/exclusionsavantcorpus ;pastri/recaptureaprèsREFUSED.
TransportCapturePlan explicite+snapshotbuffersRAM ; sinkmanifestexclusifhashé/
fsyncavantbuilder ADMITTEDouREFUSED(pasreportindex), replaymanifestmêmespaths/
SHA sansreselection. Prépa extractionBASELINEvraiconsommateurB+spies/4vacuïtés
(IOenum/meta/readdistincts)+positif AUTORISÉE, fautesconservéesjusqu'auxrouges.
SourceRED+recette300/32Gquota4/T3552/ELFJSONavanttests àpublier/RO/GOdistinct
AVANTCargo. 0corpuscontact/natif/implgatecorrectifavant4rouges ;A/BHOLD,
SHA2devdepversionexacte/lockaudit/offlinepascryptoDIY. Nouveau lot propre
cx-lucivy-fs-admission àpréparerdepuis1ee5, mémoires9historiquesconservées.

RO Recherche reçue17nov sur source exacte8b9435363dbd8ec0e3973f2368b641189f0726eb
+docs c987957426 : favorable BORNÉE baseline/recette proposée, aucun blocage
statique établi. VraiB/seamStdFsSpy/panel15 inchangé,6négatifs4familles+positif
statiquement discriminants ; compilation plausible mais NONvérifiée. Delta
lock offline à archiver strictement. Aucun Cargo/rustfmt/test/ELF/proc/corpus/
service par Recherche. PasGO300 ni qualificationFS/native ; pilote concret,
RO et GO C7 distinct restent requis. Ticket reste ouvert.

C7 poursuit autorisation PRÉPARATION du paquet/piloteRED17nov sous ce ticket,
aucun Cargo/testnatif/corpus. Proposition300 conservée (release/offline mêmeT),
END physique unique incluant préflight/CommonMIN/final ; ledger2ELF copié sur
portable avant toutELF via exporteur autonome armé AVANT lancement, ACK
source/nonce/SHA sans dépendance humaine. Bail lancement15s, échec si poste
nonacquis dans ce délai ; aucune attente sans borne ni GO implicite. Six
rouges par noms/causes/traces, infraSTOP ; contrôlepositif. ROcouple puis GO
C7 distinct avant exécution. Code source8b94353 inchangé.


### Banc — couple RED préparé6a16c7220f, 17 novembre

[RO source/paquet favorable bornée](../../banc/17-11-2026/03-ro-couple-fs-red.md) : troisAST, neuf copies PC/preflight79adf115 et exporteur portable conformes ; barrière ledger2ELF avant exécution, ARM15/physical300, causes sixrouges et positif vérifiés par lecture. Aucun pilote/test/proc/corpus/DB contacté/exécuté par Banc. GO C7 distinct requis ; ticket OUVERT et FS corrigée/global non qualifiés.
