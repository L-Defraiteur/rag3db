# Une scope e2e imbriquée peut échapper au plafond du lanceur

- **État** : ouvert le 12 novembre 2026 ; porteur recherche pour la primitive scripts/procédure, relecture ED ; faits ED et mémoire.
- **Pour** : recherche pour la recette commune des lanceurs ; orchestration coordonne les créneaux et ED relit son adoption. Aucun changement du runner produit dans le lot des accords.
- **Famille** : produit
- **Gravité** : risque de dépassement du budget mémoire et d'arrêt incomplet des enfants.
- **Atteignable en service** : non, lanceurs de validation.
- **Touche rag3weaver** : oui, montage de run_e2e et attestations de cgroup.

## Faits et limite de la preuve ancienne

Dans `extension/rag3weaver/run_e2e.sh`, l'absence de `RAG3WEAVER_VERROU_TENU` relance la suite
sous poste lourd (64–102). `confined` (181–190) crée aussi par défaut une scope systemd
`MemoryHigh=16G`. Une scope ainsi créée ne constitue pas une preuve d'appartenance à l'unité
privée de l'appelant : elle peut être sœur, hors de son MemoryMax et de son KillMode.
MemoryHigh est une limite souple, pas MemoryMax.

ED a lancé sa globale b42 sous unité privée Max32G et KillMode=control-group, vérifiés sur
le parent. Les deux suites e2e sont fonctionnellement vertes, mais leur PID/cgroup réel n'avait
pas été attesté. Le ciblé lectures24 avait une unité enveloppante au pic9,4M, alors que le
runner lançait sa propre scope. Ces mesures du parent ne prouvent pas le plafond des enfants ;
on ne réattribue pas rétrospectivement un cgroup non observé. Mémoire a identifié le même piège
et interrompu préventivement une tentative avant e2e ; ses preuves restent dans son ticket M8.

Preuves ED : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/`
(`passe.sh`, `cgroup-passe`, `memorymax-passe`, journaux et verdict global b42-r2).
Source produit figée : `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`.

## Montage correct à composer

Après acquisition réelle de poste lourd, dans une unité privée effectivement Max32G avec
KillMode=control-group et budget après acquisition :

- poser `RAG3WEAVER_VERROU_TENU` pour ne pas reprendre un second verrou ; jamais avant un vrai verrou ;
- poser `RAG3WEAVER_BUILD_MEMORY_HIGH=0` pour éviter la scope intérieure quand le plafond extérieur
  est déjà garanti ; cela ne retire pas le MemoryMax extérieur ;
- relever le **PID du vrai binaire e2e**, son `/proc/PID/cgroup`, le memory.max de ce cgroup,
  son identité de suite et le KillMode de l'unité ; échouer si aucun vrai enfant n'est observé ;
- préserver les anciens passages, puis refaire seulement les étapes dont le confinement restait
  non prouvé, avec verdict composé explicite et source inchangée.

ED prépare cette vérification bornée pour lectures24 et démon13, autorisée par orchestration,
sans rejouer features/lib/nav déjà validés. Aucune désactivation de bac à sable, hausse du plafond,
nouveau service ou test de charge n'est nécessaire.

## Pour fermer

Consigner les preuves enfants ED/M8, prévenir les autres drivers et intégrer la recette dans
la procédure commune de passe. Vérifier que budget, arrêt et comptage des lourds couvrent les
vrais descendants. Toute évolution du runner lui-même est un lot séparé, relisible et testé ;
ce ticket n'en annonce aucune comme livrée.

## Preuve enfant ED acquise

Sur b42 inchangé, après autorisation et créneau7879 : lectures24/0/0 et démon13/0/0,
deux suites sur deux sans saut, unité600s après acquisition. PID2697195 et2702388
réellement observés dans cx-ed-accord-e2e-confines-20261112.service, avec
memory.max34359738368, RuntimeMaxUSec10min et KillModecontrol-group.
`I-luciepc/accord-20261112/{e2e_lectures,e2e_demon_hote}-enfant.json`,
`verdict-compose.json` et `quiescence-e2e-confines.json` ([]). Le verdict composé garde
features/lib/nav précédents et remplace seulement la preuve de confinement des e2e.
M8, prévention des autres drivers et procédure commune restent à orchestration : ticket ouvert.

## Extension du diagnostic : Chromium et le bus de session

Recherche487 a arrêté NAV40 après déplacement d'un PID Chromium dans une scope
app-org.chromium sœur. Le préflight natif distinct a observé le même PID/exe
d'abord sous Max32 puis dans cette scope avec memory.max=max ; arrêt des seuls
processus privés, aucun reste. Ni NAV40 complète ni lectures27 de cette source
ne sont validées par ces essais. Les anciens contrôles des PID e2e ne prouvent
pas rétroactivement le confinement de tous leurs descendants Chromium.

Source primaire relue à la version du paquet :
[Chromium156 systemd.cc](https://github.com/chromium/chromium/blob/156.0.8078.79/components/dbus/xdg/systemd.cc)
et [son contrat](https://github.com/chromium/chromium/blob/156.0.8078.79/components/dbus/xdg/systemd.h).
L'initialisation du portail peut demander au gestionnaire systemd du bus de
session une scope app avec le PID courant. Le chemin observé est cohérent avec
ce mécanisme ; la lecture ne remplace pas une trace locale de l'appel D-Bus.
Changer seulement le lanceur /usr/bin en binaire /usr/lib n'isole pas ce bus.

Recette candidate à préparer avant un nouveau préflight borné : bus de session
strictement privé au job via dbus-run-session, configuration explicite sans
répertoires d'activation de services, puis Chrome réel avec les mêmes arguments
et un profil neuf. Les outils et leur manuel1.16.2 existent déjà sur luciepc.
Le helper IPC doit rester dans le même Max32 et terminer avec le programme ;
aucune modification du bus utilisateur, de KDE, du système ou du bac à sable.
Ne pas simuler un environnement Flatpak/Snap pour éviter une branche de code.

Critères du seul préflight candidat : configuration/source/version/hash publiés,
30s et budget global initial inchangés, CDP réel disponible, tous les processus
privés observés dans le groupe borné, aucun détenteur restant après arrêt.
Au premier écart, arrêter les seuls processus attribués et soumettre le résultat.
Une conformité prouvée permettrait ensuite les seules phases manquantes, sans
rejouer compilation/lib/features déjà valides ni annoncer un navigateur simulé.
La recette n'est pas déclarée validée à l'écriture de cette note.

## Extension observée : navigateur sortant du scope de validation

Recherche, intégration des fixtures journaux, source figée `487c68af39575582784cc26fc67dddad9bbc4c3a` après rebase surmasterb726d907b. Vrai poste lourd/MemoryMax32G, BUILD_MEMORY_HIGH=0, contrôle des descendants activé. Features5/5 et liblarge2201/0/65 verts ; au démarrage séparé des40 témoins navigateur, le garde observe PID2997235 dans `/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.chromium.Chromium-2997235.scope`, extérieur à la portée du poste. Le pilote arrête immédiatement la session2994167 et ses descendants possédés ; aucune poursuite vers lectures ni retry.

Il ne s’agit pas de la scope créée par `run_e2e.sh` : l’étape en cours est `scripts/temoins_navigateur.sh`. Le code de test lance le navigateur par Command::new, le candidat disponible est /usr/bin/chromium, petit lanceur ELF vers /usr/lib/chromium/chromium. Le composant exact qui demande le déplacement et le plafond effectif de la scope sœur n’ont pas été mesurés avant l’arrêt : aucune attribution rétrospective de limite ni cause interne affirmée. Une recette qui ne vérifie que le cgroup du binaire de tests ne prouve donc pas celui de Chromium.

Preuves deux postes : `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/integration-487c68/` (source.json, argv.json, cgroups-enfants.json, navigateur.log, lancement.log, verdict-final.log, arret-session-2994167.json). Le PID2997235 et le wrapper2947827 sont ensuite absents ; quiescence-apres-arret.json porte active=[] et chromes=[] par exe/cwd/profil privé. Aucun service7879 utilisé, aucun navigateur utilisateur arrêté par nom.

Suite à orchestration/propriétaires des drivers : établir une recette navigateur restant sous le plafond et l’arrêt de la passe, l’attester sur vrais PID enfants avant de reprendre l’étape. Aucun changement produit ni désactivation de limite livré par ce constat ; pas de relance inchangée. Le ticket des fixtures reste distinct et ouvert.

## Résultat de l’unique préflight native, sans reprise des tests

Recette publiée avant exécution dans5e4d91596 puis4ff5d71b9 (ajout du traitement de SIGTERM pour le nettoyage). Une seule préflight sous poste lourd réel/Max32, timeout30s, deadlineglobale17:27:44 inchangée. Binaire direct `/usr/lib/chromium/chromium` identifié statiquement par paquet/version/hash ci-dessus, mêmes arguments que le témoin et profil neuf ; aucun changement système/KDE/DBus, option de sécurité ou service.

**ROUGE en1,246s.** PID3028835, starttime84506851, observé successivement dans `run-p3028833-i82714792.scope` avec memory.max34359738368 puis dans `app-org.chromium.Chromium-3028835.scope` avec memory.max=`max`, exécutable /usr/lib/chromium/chromium inchangé. L’accès direct au natif ne suffit donc pas à préserver le confinement. Le parent garde le même PID/starttime : il ne s’agit pas d’une confusion avec un navigateur déjà ouvert.

Onze PID privés attribués par filiation ; le garde a arrêté l’inventaire à la première sortie, donc le relevé cgroup publié porte ce parent aux deux états, pas une mesure complète rétrospective de tous les enfants. Version CDP non obtenue avant arrêt (null) : seule la version de paquet est revendiquée, aucun succès de protocole inventé. Tous les PID privés attribués sont arrêtés via pidfd/starttime, restes=[], profil supprimé après vérification. Preuves deux postes : integration-487c68/preflight-native-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log} et lancement-preflight-native.log.

Conformément à l’autorisation, aucun autre essai ni reprise de NAV40/lectures27/vrai checklib minimal. Les verts features5 et liblarge2201/0/65 restent distincts ; aucun verdict composé complet n’est affirmé. Blocage soumis à orchestration : il faut une recette de confinement du vrai navigateur avant de compléter la validation ; le lanceur /usr/bin n’est pas une explication suffisante. Ticket commun des scopes et ticket journaux restent ouverts, ressources de build et preuves conservées, aucun master.

## Préflight bus privé verte puis verdict composé d’intégration

Recette explicitement autorisée publiée268460373 avant son unique lancement ; sourceRust487c68af39575582784cc26fc67dddad9bbc4c3a inchangée. Préflight bus privé **VERTE en4,533s** : CDP réel protocol1.3, productChrome/156.0.8078.79, revisiona21ce95c24ce3964416f09ae1862f53f319eb4d0. Treize PID attribués, dont dbus-daemon et le navigateur natif, observés dans la portée Max32 ; restes=[], profil et socket privés supprimés. Preuves preflight-dbus-unique/{recette.json,pid-cgroups.json,verdict.json,stderr.log}. Aucun bus utilisateur, KDE ou système modifié, aucun service permanent activé.

La conformité de cette préflight permet la seule reprise explicitement autorisée des phases manquantes, sous le même budget initial2400 et deadline17:27:44 ; aucune relance des features5 ou de la liblarge déjà vertes. Vrai minimal : **cargo check --locked --lib --no-default-features rc0**, sans feature native et sans cargo test. NAV40 séparée, même ELF et mêmes arguments/profils, sélection native via RAG3WEAVER_NAVIGATEUR sous dbus-run-session/config privée sans activation : **40/0/0 en131,52s**. Puis /sante7879 réelle BGE-M3/sparse/facticefalse, lectures sous racine privée avec espace sans segment commandes : **27/0/0 en9,65s et2filets verts**. Santé après OK, aucun autre usage7879 recherche.

Les532 PID observés durant les phases finales, dont bus éphémère et vrais processus Chrome/e2e, restent dans un seul cgroup `memory.max=34359738368` ; HIGH0 et marker sous poste réel conservés. Source/manifestes propres et identiques. Trois ELF attestés source/native29 complète/provenance du bon worktree/**220 gabarits présents chacun** : lib SHA2566782a8e2c5a5be3f2a297530c97be9bfc10eb3437a0c4bb2eefdf1b995625691 ; NAV2567af2a82fc73ae3a6b3b0bd039af2b4eeb46d6697eeabf867474e48021ac2c ; e2e4a4266874ae7ed026e18d55481a4ad83af79c4e42c7a2e92120e524d718e9621. Chrome natif et configuration D-Bus ont toujours les hashes publiés. Scan /proc exeTarget/cwdWT/profilprivé=[] et socketbus absent : quiescence-finale.json.

**Verdict composé VERT**, sans transformer les anciennes tentatives en une passe sans incident : features5/5 et lib2201/0/65/0filtré proviennent du premier passage487 ; vrai minimal, NAV40, lectures27 et attestations des phases finales ci-dessus. Les rouges du garde et de la préflight native restent conservés, diagnostiqués séparément. Le checklib natif redondant est étiqueté comme tel ; il ne remplace pas le minimal. Le global2400 initial est respecté, aucune réinitialisation de budget ni prolongation, fin avant17:27:44. Aucun test/runtime/feature produit modifié pour le confinement ; aucun e2e entier, service/GPU/modèle nouveau ou rerunCI.

Preuves brutes deux postes : integration-487c68/{verdict-compose.json,verdict-compose.log,preflight-dbus-config.xml} et finalisation-dbus/{source.json,argv.json,cgroups-enfants.json,vrai-minchecklib.log,navigateur.log,lectures.log,e2e-complet.log,sante-7879-avant/apres.json,attestations.json,quiescence-finale.json,verdict-final.log,provenances.d}. Ticket commun des scopes reste ouvert pour la procédure transversale. Correction de fixture et preuves soumises à revue ; ticket journaux ouvert et aucun master avant àtoi. Ressources propres conservées, aucun lourd ou utilisateur7879 recherche restant.

## Reprise parallèle autorisée le 12 novembre : rendre la recette commune

Lucie demande de reprendre des chantiers utiles en parallèle des couches.
Recherche prend la suite d'outillage de ce ticket : intégrer la recette prouvée
dans une primitive de lanceur partagée et sa procédure, au lieu de recopier les
scripts privés à chaque passe. Le lot journaux et le lot BGE sont déjà intégrés
(0d7bf10eb et 6dd116e8f) ; aucun rejeu de leurs tests ou ancien plan FTS n'est demandé.

Composer les scripts existants : bus de session privé sans activation pour les
témoins navigateur sous Linux, acquisition réelle du poste avant marqueur de
verrou, absence de scope sœur échappant au plafond extérieur, arrêt et nettoyage
des seuls processus et chemins attribués au lot. Pas de modification du bus de
Lucie, de KDE, des services permanents, de Chromium ni de ses options de sandbox.
Conserver les limites Max32, les délais et les refus explicites ; une dépendance
manquante ne devient pas un succès ou un saut silencieux.

Proposer un diff borné des lanceurs et témoins d'outillage, publié avant essais,
avec adoption explicite par les chemins existants concernés. Première validation
CPU des contrats de montage et de nettoyage ; une préflight réelle bornée ne
sera lancée qu'après recette et coordination, sans reconstruire ou rejouer les
lots verts. ED relit la réutilisation ; Recherche garde l'édition des scripts.
Le ticket restera ouvert tant que la garantie transversale n'est pas réellement
adoptée et éprouvée. Les sources des passes Arbre figées restent inchangées ;
aucun push de ces scripts pendant sa passe sans coordination de l'intégration.

## Diff de primitive commune préparé sur43db9634a

Worktree/branche neuf cx-recherche-confinement-lanceurs, aucun ancien lot rouvert ni source Arbre modifiée. Propriété scripts recherche, ED relecture. Proposition : `scripts/confiner_validation.py`, primitive unique de portée de validation Linux, adoptée réellement aux entrées run_e2e.sh et temoins_navigateur.sh. La procédure donne aussi l’entrée composée pour posséder un budget/bus/plafond uniques pour toute une passe.

Contrats : vraie ascendance poste/fiche active avant marqueur ; plafond effectif hérité <=32Gio ; reçu privé valide seulement dans l’ascendance du superviseur vivant (PID/starttime/cgroup/token), sans bypass par variable ; HIGH0 et suppression de la création de scopes sœurs dans confined ; bus éphémère privé EXTERNAL/UID courant sans aucun répertoire d’activation ; suivi des descendants/subreaper et cgroups, arrêt pidfd des seuls attribués, code/rouge conservé sans retry, résidu rouge et nettoyage des seuls chemins privés après quiescence. Aucune option de test permettant de désactiver le confinement en production ; dépendances/limites manquantes refusées. Aucun Rust produit, réglage système/KDE, modèle/service permanent ou option de sandbox modifié.

Témoins CPU préparés : XML sans activation/échappement, plafond hérité et refus sans limite, marqueur seul/reçu périmé refusés, scope sœur rouge avec preuve, PID réutilisé jamais signalé, dépendance absente avant lancement, adoption des DEUX vraies entrées avec frontière factice et sans Cargo, commande normale/nettoyage, code rouge sans retry, deadline avec enfant et enfant détaché résiduel rouge/nettoyé. Les simulations n’écrivent ni dans les verrous réels ni dans les cgroups système, ne lancent aucun bus/service/navigateur ; seuls petits processus CPU privés.

Contrôles statiques préparatoires : parsing Python, bash -n des deux entrées et diff --check verts. Source et recette à publier avant exécution réelle ; CPU à jouer en premier, premier rouge inattendu stop/diagnostic. Unique essai réel <=30s à coordonner après NAV Arbre, preuve CDP vraie + helper/PID/cgroups/Max32/lifecycle via la primitive publiée, sans Cargo/lib/FTS/journaux à rejouer. Aucun push code master pendant A sans coordination ; ticket reste ouvert jusqu’à adoption prouvée et revue/intégration.

## Relecture ED/C7 avant préflight : P1/P2 corrigés et borne de nettoyage

Premier passage CPU e2bcdd6cd :11/11 verts en1,650s ; ajout897291e78 :2/2 verts (reçu vivant/expiré et conservation d’un plafond explicitement plus bas,8G au lieu d’imposer32G). Aucun bus/navigateur réel lancé, preuves `~/.cache/rag3weaver-build/journaux-cx-recherche-confinement-12nov/temoins-cpu-{e2bcdd,ajouts}.log`.

P1 ED confirmé par inspection : possedes[777]=ancien ne permet pas d’adopter888 si table777 est désormais un tiers. La filiation exige maintenant que le parent actuel ait encore le même starttime ; test adversarial stable vérifie888 jamais attribué ni signalé. P2 : un reçu cohérent/token choisi nommant un ancêtre poste ne prouve pas une supervision. La vérification exige maintenant aussi l’exécutable Python réel, l’entrée `lancer` et l’identité/hash du script superviseur en cours dans cette ascendance. Test avec ancêtre réellement vivant et poste/plafond simulés valides, fichier0600/token/starttime/cgroup/délai cohérents : refus s’il n’exécute pas le superviseur. Cette garantie porte sur l’absence de bypass par environnement/reçu seuls ; aucune isolation absolue entre programmes hostiles de même UID n’est revendiquée.

Retour Arbre/mémoire intégré : pidfd est ouvert AVANT la dernière relecture du starttime et le signal passe par ce FD ; nouveau test de réutilisation entre open et recheck sans signal. Un échec de nettoyage après commande rc0 capture maintenant l’erreur dans le verdict ROUGE et conserve le temporaire, au lieu de perdre le verdict dans une exception ; témoin négatif préparé. Sources à republier avec ces corrections et CPU à rejouer car les frontières de suivi/autorité/nettoyage changent. Premier rouge inattendu stop/diagnostic ; aucun essai réel avant finNAV Arbre et revue de cette source.
