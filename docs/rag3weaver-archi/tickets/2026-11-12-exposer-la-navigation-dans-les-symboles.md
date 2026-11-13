# Exposer la navigation dans les symboles

- **État** : ouvert ; première tranche intégrée par 3edbb8475 et passe complète d00661e96 verte ; grep et couverture étendue restent à livrer.
- **Famille** : produit
- **Gravité** : capacité manquante ; l'agent doit pouvoir explorer des arbres de code sans lire tous les corps.
- **Atteignable en service** : oui sur master depuis 3edbb8475 via outil code et relais atelier ; outline reste disponible.
- **Touche rag3weaver** : oui, outils déclaratifs du code et de l'atelier, références de symboles, rendu et enrichissement du grep.
- **Pour** : ED et archi pour les contrats/outils ; codeparsers pour la couverture des membres et relations.
- **Signalé par** : Lucie, demandes list_methods puis list_members, parents/enfants bornés, list_symbols(file) en arbre et option symbol_hierarchy_depth du grep.

## Demande et conception

[Vision](../../orchestration/visions/2026-11-12-explorer-les-symboles-et-leur-hierarchie.md).
Composer outline/Plans/parcours existants, plutôt qu'un deuxième moteur par client.
Ne pas confondre contenance, appartenance déclarée et héritage ; une référence doit
être qualifiée dans sa vue. Décrire les limites de couverture et d'index au lieu
de rendre une absence trompeuse. Grep : profondeur supplémentaire0 par défaut,
enrichissement optionnel avec provenance, sans réindexation implicite.

## Fin attendue

Contrat commun et première tranche reviewables, puis autorisation et validation
du vrai parcours agent : lister l'arbre d'un fichier, choisir un membre, parcourir
ses parents/enfants et lire seulement ce qui est utile. Témoins de profondeur,
ambiguïté, membres partiels, fichier modifié/index périmé et troncature. La note
de conception ne ferme pas ce ticket et ne déclare aucune nouvelle API livrée.

## Première tranche autorisée pour la nuit du12au13novembre

Lucie demande des lots parallèles utiles depuis les visions. Archi prend la
première tranche de navigation réellement accessible, après qualification de
la couverture ; codeparsers prend l'inventaire IR/fixtures en soutien, sans
pointeur parent ni ancienne administration à modifier.
[Répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).

Réutiliser outline/Plans/parcours ; ne pas ajouter un alias sans capacité utile.
Arbre de fichier puis membres/parents/enfants qualifiés, profondeur et couverture
explicites, sans confondre héritage et contenance. Noms et découpage sont des
choix d'implémentation ; les vrais changements de conception sont numérotés sur
la page existante. Archi garde la navigation/gabarits ; ED garde le parcours
agent et annonce toute édition nécessaire d'un fichier partagé. Publier diff
et témoins avant lourd, puis validation du vrai accès commun aux agents.

## Revue de la première source c999774c5 — nuit du12novembre

Relecture statique de `src/symbol_navigation.rs` par orchestration ; la source
testée en cours reste figée. Aucun build ou ciblé supplémentaire lancé par cette
revue, aucune réussite du runtime encore revendiquée. Archi garde la correction
et les témoins dans son lot. Deux écarts sont à traiter avant avis favorable :

- Le plafond96Kio n'est appliqué qu'à la sortie finale (lignes287–296).
  `ambiguous_symbol` retourne déjà aux lignes196–208, avec jusqu'à200 candidats
  incluant signatures512caractères, noms et références. Ce chemin peut dépasser
  le plafond annoncé ; `ambiguous_source` et les autres retours anticipés ne
  passent pas non plus par la réduction commune. Qualifier les sorties d'erreur
  et d'ambiguïté avec le même contrat de budget, sans cacher leur statut ni
  inventer une liste exhaustive. Le budget vise le résultat effectivement rendu.
- Les égalités/plages ambiguës sont signalées dans `coverage.ambiguous_ranges`
  (ligne186), mais le rendu Markdown par défaut ne lit pas ce champ. Un parcours
  parents sur deux scopes de même plage rend `ready`, le seul symbole puis
  l'avertissement générique de couverture ; le demandeur ne voit pas que son
  parent est indéterminable. Rendre cette ambiguïté explicitement aussi en mode
  compact et conserver le choix non arbitraire. Même exigence pour les faits de
  membres malformés et les limites spécifiques, sans devoir deviner qu'il faut
  relancer en JSON pour découvrir une limite de ce résultat.

Point de coût à qualifier avant d'appeler l'outil borné et rapide : la recherche
des parents minimaux (lignes158–170) contient trois boucles. Sur une chaîne de
plages imbriquées, chaque candidat recherche parmi les autres candidats ; le
nombre d'appels `contains` croît au cube, malgré le plafond2000scopes. Cette revue
ne constitue pas une mesure de latence. Prévoir un témoin de travail/complexité
borné ou réduire l'algorithme ; ne pas lancer un gros benchmark pour prouver ce
que le comptage d'opérations peut établir.

Codeparsers a aussi transmis directement à Archi la limite du mode members :
des méthodes Python/Rust présentes comme scopes peuvent manquer quand les seuls
champs `declarations`/`field_types` sont vides. Composer les faits disponibles ou
nommer cette limite précise ; ne pas annoncer une absence de méthode comme un
inventaire complet. Qualification IR livrée dans
[navigation-ir/qualification.md](../../codeparsers/12-11-2026/navigation-ir/qualification.md),
master a4bf86c865. Le ticket reste ouvert pour la façade et ses autres critères.

## Prise en charge archi — première tranche

WT isolé `~/.cache/rag3weaver-build/cx-wt-symboles`, branche `cx-archi-symboles-1`,
base a783013f1. Qualification statique : outline est plat, limité/totalisé, sans
comparaison du contenu ; HAS_PARENT comporte des appartenances hors fichier et
filtre les parents fonctions. Aucun changement de cet outil existant.

Façade commune proposée `navigate_symbols(path, mode, symbol, depth, limit, format)` :
file, parents, children et members. Un Plan déclaré lit les scopes d'un fichier
avec leur source/UUID/positions octets, déclarations et champs typés ; DEFINED_IN
porte la provenance/hash du fichier dans le même Plan. Limite d'entrée2000 et
compte total, sortie bornée ; un jeu d'entrée tronqué refuse d'inventer un arbre.
La contenance est nommée `range_containment` (plages octets strictes, même fichier
et source) : elle n'est ni appartenance à un type ni héritage ; les plages égales
ou ambiguës ne choisissent pas un parent arbitrairement. Les noms homonymes
rendent des candidats au lieu de choisir ; la référence UUID est qualifiée par
source/path et contexte backend existant. Profondeur : arêtes depuis racine,
0=racine,1=voisins. Les membres déclarés et champs typés disponibles sont rendus
avec leur provenance et sans identité de graphe inventée ; couverture partielle
nommée, champs non typés/constructeurs/getters potentiellement absents.

Un nœud de transformation commun manque : il unifie sélection, construction de
l'arbre borné et résultat structuré/compact. Il consomme les lignes/total du Plan,
compare le hash aux octets du FileSource correspondant sans livrer les corps ;
index périmé ou provenance non vérifiable restent explicites et empêchent
l'affirmation d'une contenance actuelle. Aucun accès hors FileSource, nouvel index
ou modèle. Codeparsers qualifie les fixtures/IR en parallèle, sans édition navigation.

Ajouts communs annoncés à ED avant code : lib.rs, dataflow/mod.rs,
node_factories.rs, manifestes code/atelier ; nouveaux modules/Plan/gabarits propres.
Source et recette publiées avant lourd, ciblés bornés puis passe/revue adaptées.
Cette tranche ne livre pas l'enrichissement grep ni la complétude des membres ;
le ticket reste ouvert tant que ces critères et le vrai parcours ne sont pas acquis.


## Source initiale et recette de qualification

Qualification codeparsers a42b41c73 relue : les offsets sont des plages de lignes,
non des bornes AST. Égalités signalées, lambdas repliées absentes, fonctions imbriquées
variables selon langage. Python peut donner la ligne du conteneur pour un champ ;
la propriété typée projetée n'a plus de position. Go/hors-classe C++ ne sont pas
reliés par inclusion ; Rust possède plusieurs Box (struct/impl), donc ambiguïté.
Les membres restent des faits séparés de la contenance dérivée, sans UUID inventé.

Source initiale : module commun et adaptateur SymbolNavigationNode, Plan borné2000,
façade navigate_symbols sur code et relais contextuel atelier. Cinq témoins purs
(profondeurs, UUID/homonymes/sources, plages égales, fraîcheur, limites, membres)
et un témoin réel hôte/chat à modèle de fixture (outil déclaré→arbre→membres,
retours réels sans corps), puis relais atelier avec contexte et refus de vue.
Aucune réussite de ce ciblé n'est encore acquise.

Premier ciblé CPU sous poste lourd, unité privée RuntimeMax600/Max32G/Swap4G,
KillModecontrol-group, j8 et target archi unique32Go, lib29 complète épinglée.
`cargo test -j8 --lib --features rag3db-native,code,daemon,openai-llm -- symbol_navigation --nocapture`.
Variables de services/modèles/identifiants retirées, roots/cache/runs privés,
pas de GPU ni donnée de Lucie. Source et lanceur externe annoncés avant lancement.
Premier rouge inattendu : arrêt, diagnostic publié avant correction/reprise ;
pas de replay inchangé. Passe adaptée proposée après résultat, aucun master sans revue/tour.


Revue codeparsers a4bf86c865 : réserve concrète sur members de c999. Python/Rust
peuvent avoir des méthodes indexées sans entrées declarations ; ne rendre que ces
faits omet une capacité déjà disponible. Nouveau témoin exige le scope méthode
direct dans nodes ; correction prévue compose ces enfants de plages (qualifiés
comme tels) et les faits déclarés séparés, sans les confondre avec appartenance.
Le ciblé c999 reste figé pendant son exécution ; nouveau rouge attendu après.


## Diagnostic et corrections après revue

C999 : refus montage avant compilation (XDG_CACHE_HOME déplaçait le magasin),
corrigé uniquement par RAG3DB_LIBS_DIR explicite. Compilation ensuite rouge E0283
sur MockLlm.into et E0521 sur Cow du schéma ; typages corrigés d6bad0abd.
Sur cette source, témoin membres effectivement rouge0/1/0 : scope méthode direct
absent malgré sa présence dans les lignes qualifiées. Rouge conservé séparément.

Revue orchestration741d6b8d30 acceptée : plafond commun appliqué à TOUS les statuts
et au rendu markdown ; candidats ambigus retirés sans tronquer leurs identités.
Ambiguïtés de plages et faits invalides visibles par défaut en markdown. Calcul
du parent réduit à deux scans par scope (au plus2N² comparaisons comptées), sans
triple boucle ; témoin chaîne64 et compteur, pas de grand banc chronométrique.
Membres compose désormais scopes directs de plages dans nodes et déclarations/
propriétés dans members, gardant leur sens et leur provenance distincts.
Deux témoins supplémentaires couvrent budget sur symboles/sources ambigus et
limites visibles/coût quadratique. Le vrai chat doit aussi recevoir la méthode run.
Qualification codeparsers a4bf86c865 liée :
`docs/codeparsers/12-11-2026/navigation-ir/qualification.md` (7 fichiers/50 scopes,
preuves de parsing, projection relue ; aucune exhaustivité ou UUID inventé).
Reprise prévue même filtre symbol_navigation (8 témoins), source/recette publiées
avant exécution ; temps des tentatives déjà dépensé non effacé.


Ciblé879 observé :7 témoins purs verts/1 intégration rouge, avant l'ouverture du
backend : SymbolNavigationNode n'était pas qualifié nœud d'outil de backend.
Diagnostic backend_code::READ_NODES, ajout du seul type annoncé à ED avant code.
Le nœud lit FileSource pour le hash : policy.read_files existante est requise,
pas un nœud de base/reaction permissif. Témoin d'intégration exige maintenant le
refus au chargement si cette politique est retirée, puis rétablit sa fixture.
Pas de modification hôte/agent ni d'élargissement général. Source/recette de la
correction publiées avant nouveau ciblé ; rouge879 conservé.


Ciblé389 :7purs verts et vrai chat arbre/membres vérifié ; rouge au relais atelier
« invocation contextuelle indisponible hors d'un hôte ». Fixture construisait
DaemonHost seul sans son initialisation publique se_connaitre (appelée normalement
au démarrage). Correction test uniquement : Arc puis se_connaitre avant ouverture,
aucun runtime/hôte/permission modifié ; nouveau replay dans le reste600initial.


Ciblé0e310 :7purs verts, vrai chat et relais effectif prêts. Dernière assertion rouge
sur cas négatif : view="unavailable" est invalide au schéma (type object requis),
donc ne peut tester le refus de vue de l'hôte. Correction fixture : view={} valide
au schéma, refus attendu « vue contextuelle non disponible ». Aucun code produit
modifié depuis389. Budget initial600 arrivé à terme ; aucune relance ni remise à
zéro implicite. Étape corrective distincte120s proposée à orchestration, même
filtre8, source neuve et recette publiées avant exécution. Tous rouges conservés.


## Recette globale distincte, publiée avant entrée

Ciblé30ab48674 acquis8/0/0, ELF41bf0591…/223 gabarits/mapsnative29/quiescence[] ;
600initial et corrective120 terminés séparément, aucun budget réinitialisé.
Branche neuve cx-archi-symboles-2 après rebase master741d6b8d30 (docs/fixtures IR
uniquement) ; revue741 préservée, diff src/templates avant/après rebase vide.

Globale CPU6000s après acquisition poste lourd, sous scripts/confiner_validation.py
(mode lourd, preuves externes), j8, Max32G/Swap4G/Highmax, target archi unique32Go.
Source HEAD figée/poussée, pilote externe cx-symboles-passe.py publié aux deux postes
et empreinte transmise avant lancement ; arrêt au premier rouge inattendu.
Phases plafonnées dans le même6000, sans réserver leur durée ni remettre l'échéance :

- Base PostgreSQL privée cx_archi_symboles_20261112 créée et SELECT current_database
  vérifié (45s) ; aucun schéma/base partagé utilisé. Le plan/gabarit modifié justifie
  la suite postgres de la validation commune ; dump et empreinte avant nettoyage.
- F5 par check_features (2100s), puis vrai minimal --no-default-features (120s).
- Compilation des trois binaires large, navigateur, e2e : plafond partagé1500s,
  listes réelles/inventaire gelé et publié avant exécution ; compter les huit témoins
  symbol_navigation dans la lib, compter les gabarits réels (ne pas forcer220).
- Lib entière large (900s), étape navigateur (600s, paquets React existants).
- E2e lectures27, hôte13 et postgres (effectif gelé depuis inventaire),120s par suite,
  une suite par appel run_e2e. Environnement journaux à espaces conservé.

Jeu large : rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,
anthropic-llm ; aucun service dense ni modèle d'inférence. Burn forcé CPU.
Lectures seules demandent sparse BGE7879 existant : créneau coordonné après les
compilations, santé avant/après et restitution explicite ; pas de réservation
pendant F5. L'attente du créneau compte dans6000. Identifiants/modèles retirés.
PID/starttime/cgroup/mapsnative29 enregistrés pendant chaque binaire testé,
SHA ELF et table des gabarits réelle vérifiés. Résultat driver et superviseur
VERT, aucune suite sautée, quiescence après, copies de toutes preuves au portable.
Aucune publication master incluse : revue puis tour explicite restent requis.


## Relecture pendant compilation globale — ordre du rendu

Constat statique concret sur2fcc, pas encore exécuté : nodes est construit en
parcours en largeur, mais Markdown l'affiche avec indentation depth dans cet ordre.
Avec deux classes A/B et chacune run, la liste file,A,B,A.run,B.run affiche les
deux méthodes sous B. Les parent_uuid structurés restent corrects ; l'arbre compact
est trompeur. Ajouter témoin de préordre à deux conteneurs avant correction,
conserver les preuves2fcc et figer/arrêter la passe avant de changer sa source.
Correction bornée prévue : visite en profondeur stable ou rendu depuis adjacence,
sans modifier relation ni profondeur. Aucun vert global revendiqué pour2fcc.


Rouge23d272201 effectivement observé0/1/0 : ids [a,b,ar,br] au lieu de
[a,ar,b,br]. Aucun corps, modèle ou donnée réelle ; première entrée de montage
40G refusée avant test (poste préfixé à tort), preuve séparée, entrée directe
confiner32 corrigée. La globale2fcc a été arrêtée en compile-large après F5/minimal
verts ; superviseur vérifié PID/starttime/cgroup avant signal, verdict interruption
ROUGE/restes[]/nettoyétrues, quiescence[] ; deadline6000 originale conservée.

Correction : parcours préordre stable, enfants insérés en tête en ordre inversé.
JSON/indentation conservent le même arbre ; le troncage garde ses ancêtres déjà
rendus. Markdown affiche aussi parent_range_uuid et le chemin de chaque symbole ;
parents affiche explicitement ↑ parent plutôt qu'une fausse descendance indentée.
Le compteur quadratique, les bornes et la sémantique range_containment restent.
Source corrigée publiée avant reprise ; inventaire comporte désormais9 témoins.
Reprise complète F5/minimal/3compiles/lib/NAV/lectures-hôte-PG dans le RESTE de
l'échéance6000 d'origine, aucunreset. PG privée réemployée seulement après preuve
création/identité/propriété de la première portée, jamais de createdb/drop aveugle.


## Arrêt de la lib complète — compteur de types

ab85 : F5/minimal/compilations/inventaire nouveaux acquis, lib arrêtée au premier
échec the_model_sees_tools_not_the_29_raw_nodes. Lecture : il compare la taille du
registre à BUILTIN_NODE_COUNT+SearchTool. Le nouveau SymbolNavigationNode est
enregistré, mais BUILTIN_NODE_COUNT garde19 types sous code au lieu de20.
Le test de comptage du registre lit la même constante ; ne pas affaiblir ces tests.
Diagnostic borné prévu sur l'ELF exact et ce seul test avec nocapture pour voir
les tailles, puis mise à jour de l'inventaire déclaré correspondant au type ajouté.
Nouvelle source/driver avant reprise entière dans le même reste6000, aucunreset,
aucun service modèle employé dans ce diagnostic. Les preuves ab85 restent rouges.


Diagnostic ELF exact ab85 observé0/1/0 : nœuds+SearchTool réel77, attendu76.
Correction : BUILTIN_NODE_COUNT code19→20 pour le type nouvellement enregistré,
sans changer les assertions ni les outils modèle. La lib complète ab85 n'est pas
verte ; le ciblé de diagnostic est volontairement rouge et conservé séparément.
Reprise intégrale source corrigée avec9 témoins navigation, même échéance globale,
ouvelle portée et dossier pour ne pas écraser les preuves précédentes.


Le diagnostic lib concurrente --nocapture a reproduit la première assertion de
la vidange (message prématuré avant vider). Ticket qualification ED lié ci-dessus :
la nouvelle fixture navigation n'isolait pas son registre background, contrairement
au helper hote_isole ; shutdown pouvait tuer la commande du test voisin via le
singleton. Correction uniquement dans cette fixture, with_background Registry neuf.
Production et assertions/délais RunScope inchangés ; reprise entière du même reste
6000 pour confirmer l'interaction, pas de nouveau budget ni modèle/service.

## Passe complète qualifiée — 13 novembre2026

Source d00661e969eb7d77aa7d2f25c6cb5fa4f93f3308 : F5 5/5, minimal réel
--no-default-features, lib2252/0/65 (neuf témoins navigation), NAV40/0/0,
lectures27/0/0, hôte13/0/0 et Postgres privé36/0/0. Les rouges intermédiaires
restent conservés : parcours compact corrigé en DFS stable, compteur des nœuds
code20, puis registre background privé de fixture. Ce dernier corrige
l'interférence de shutdown avec le témoin RunScope voisin, sans modifier son runtime.
Le ticket de qualification est clos dans docs/tickets/closed/
2026-11-12-la-vidange-de-portee-rougit-dans-la-lib-entiere.md.

Preuves aux deux postes : cx-symboles-d00661e96-reprise-20261112, hors target.
Inventaire b43e9a0892602d5415f335016092fa75a9b5743b74a8492da0a8694a72a5716a,
trois ELF réellement observés/native29/223 gabarits chacun ; primitive VERT0,
restes[], nettoyé=true, quiescence target[]. Durée cumulée4253,885/6000s
initiales ; aucune remise à zéro. Aucun master avant tour explicite et revue du
diff entrant. Ce ticket reste ouvert : grep enrichi et couverture étendue non livrés.

## Livraison et nettoyage — 13 novembre2026

Première tranche intégrée sur master3edbb8475ace557b87316da6a269938055d23f9d,
après revue favorable C7, tour explicite et rebase sur le formatage C++ f903fedba.
Diff source testée d006 → candidat publié : src/templates/Cargo/scripts inchangés.
Les trois contrôles après push sont verts (propre, aucun rebase, origin/master=HEAD).
Le ticket global navigation reste ouvert : grep enrichi et couverture étendue à faire.
Aucune vue supplémentaire ni complétude de membres/héritage n'est annoncée.

Nettoyage immédiat attesté aux deux postes : garde courante dca13abb… verte,
archives administratives fidèles dont sous-module privé PC, quiescence indépendante.
W cx-wt-symboles, target cx-archi-target, scratch actif cx-sn et branches locales
cx-archi-symboles-1..4 retirés ; branches distantes et preuves conservées.
Le scratch PC contenait62 dépôts de fixtures : inventaire des administrations et
worktrees sans dépendance extérieure, ensemble intégral archivé hors zone active.
Archives/preuves : cx-symboles-nettoyage-{pc,portable}-20261113 dans le cache de build.

Base privée cx_archi_symboles_20261112, OID1883108, propriétaire rag3weaver :
dumpSHA256 be6872187a1fa0d73cdafdf6e5a5d7f5c3b1845b186eaacc0b75dde472ce8114
vérifié aux deux postes, zéro session active, suppression et absence vérifiées.
Preuves finales : cx-symboles-d00661e96-reprise-20261112 ; aucun appel7879 à venir.
Ne pas recréer ce worktree/target/base : pour la suite, repartir du master courant.

## Suite autorisée — enrichissement du grep, 13 novembre

La première tranche et le parcours IDE sont intégrés ; Archi reprend la demande
de Lucie encore ouverte : `symbol_hierarchy_depth`, zéro par défaut, sur la
recherche textuelle spécialisée lorsqu'un index qualifié est disponible.
Nouveau worktree depuis master, cible et preuves privées ; aucun ancien lot
nettoyé n'est recréé. Publier le contrat, les fichiers partagés et la recette
avant le ciblé ; annoncer à Recherche toute modification de sa façade de grep.

Composer le contrat de navigation et les Plans communs avec les résultats de
recherche existants, sans refaire un moteur de hiérarchie par interface. Relier
chaque match à sa source, son fichier et ses positions ; rendre la contenance
et ses limites explicites. Profondeur bornée et budget global de sortie,
ambiguïtés, couverture partielle et index périmé restent observables. L'option
ne doit ni réindexer ni lire tous les corps pour fabriquer une hiérarchie.
Les correspondances textuelles restent utilisables en l'absence de contexte
qualifié ; le résultat enrichi indique précisément cette absence.

Critères : défaut zéro inchangé, profondeur demandée réellement accessible à
l'agent, homonymes de sources/fichiers isolés, plages ambiguës non arbitrées,
index absent/périmé et troncature explicites ; vrai parcours commun de l'outil.
La couverture IR reste celle qui a été qualifiée : ni héritage ni exhaustivité
des membres ne sont inventés. Passe adaptée, revue et tour d'intégration avant
master ; aucun déploiement du daemon personnel. Ticket global toujours ouvert.


## Contrat de la suite grep — 13 novembre, avant code

WT neuf cx-wt-grep-symboles, branche cx-archi-grep-symboles-1, base43e4f185b ;
target PC unique cx-archi-target (ancien target bien retiré), scratch privé court
cx-gs, preuves neuves hors target. Aucun fichier d'activation/résidence touché.
Fichiers partagés annoncés à Recherche : code_tools.rs, dataflow/code_nodes.rs,
templates/tools/{grep,atelier_grep_files}.mmd ; navigation commune dans
symbol_navigation.rs, tests et README propres. Pas de second moteur de parents.

symbol_hierarchy_depth=0 conserve les sorties JSON/Markdown et les lectures actuelles,
sans Plan supplémentaire. Valeur positive≤16 : pour chaque ligne retenue, situer le
premier match regex avec ses offsets UTF-8 exacts (la recherche reste ligne par ligne,
une correspondance rendue par ligne), source/path/ligne et intervalle qualifiés.
Sélectionner le contenant minimal uniquement si les plages le permettent sans choix
arbitraire ; égalités/croisements donnent des candidats explicites. Ajouter ce scope
puis au plus depth arêtes de parents par range_containment à granularité ligne.
Aucun héritage/AST ou rattachement de méthode hors classe inventé.

Réutiliser le Plan déclaré symbol_navigation.json via ReadCatalog et la préparation
commune des plages/parents. Un Plan par fichier enrichi, même lecteur que le grep ;
calcul des parents une fois par fichier et non par match. Réutiliser le contenu déjà
observé par grep pour fraîcheur/positions, aucune seconde lecture des corps ni
réindexation. Les erreurs/absences du contexte restent dans l'enrichissement, sans
faire disparaître les correspondances textuelles. L'ancien scope indicatif du grep
ne sera pas promu en contexte qualifié.

Budget global des sorties enrichies96Kio (JSON et Markdown), nœuds200 et scopes2000
par fichier comme navigation, bornes de fichiers enrichis/appels déclarées ; au-delà,
conserver les matches sous le budget et annoncer troncature/raison, total connu et
absence de contexte pour les lignes non qualifiées. Le défaut zéro n'est pas soumis
à un nouveau budget ni à une nouvelle forme de réponse.

Recette ciblée à publier après le code : montageCPU/lib29/j8/Max32G/target32Go et
primitive commune, tests des invariants purs + vrai outil grep code et relais atelier
sur fixture privée indexée. Témoins défaut0 identique, profondeur1/2, UTF-8, sources
homonymes/fichiers, plages égales/croisées, index absent/périmé, budget global et
absence de lectures supplémentaires. Premier rouge inattendu : stop/diagnostic,
pas de retry inchangé. Revue C7 avant lourde globale et avant master.

## Source candidate avant ciblé

Préparation commune PreparedNavigation : hash/positions/plages et parents calculés
une fois, puis sélecteur UUID/nom ou position UTF-8 dans le même moteur. Le grep
charge le Plan déclaré existant une fois par fichier et fournit le contenu observé.
Plafonds du contexte optionnel :8fichiers interrogés,200nœuds/candidats cumulés,
96Kio pour les deux représentations. Au dépassement, les comptes restent exacts et
une troncature explicite réduit le préfixe des matches rendus ; aucune référence
partielle n'est présentée comme une identité exacte. Les erreurs de Plan sont des
statuts index_context_error, sans faire échouer le texte cherché.

Douze témoins symbol_navigation attendus (dix purs, deux parcours natifs) et deux
témoins grep_symbols (défaut0, absence d'index, UTF-8/CRLF sans relecture, budget).
Le nouveau parcours natif demande l'option par le vrai chat et son outil déclaré,
puis relais atelier ; fixture sans montre pour éprouver la péremption sans réparation
implicite. Hôtes avec registres background privés. Ces tests ne sont pas encore joués.

## Premier ciblé — compilation rouge, aucun test exécuté

Source dea3570ab02d01ef6a4b332be65e06c6090aa684, recette a163d5e7… :
Cargo101, E0277 concaténation String+&String dans fixture budget, E0283 conversion
.into ambiguë dans l'appel MockLlm de la nouvelle fixture grep. Diagnostic complet
dans cx-grep-symboles-dea3570ab-cible-20261113/diagnostic-compilation.txt ;
primitive validation-4xejkeug rouge, quiescence target[]. Aucun verdict de test vert.
Correction bornée à ces expressions de fixture : format! et String explicite.
Reprise source/pilote neufs annoncés avant exécution dans le RESTE de l'échéance
1800s initiale du ciblé, sans remise à zéro ni changement d'assertion.

Second arrêt d752 : compilation verte, dix témoins purs navigation observés ok ;
le nouveau natif refuse le manifeste avant grep, champ catch_up_on_open placé
par erreur sous workspace. Le contrat BackendEmbeddings et la fixture ED existante
le déclarent sous embeddings. Correction de la fixture à cet emplacement uniquement ;
montres toujours explicitement désactivées pour le cas périmé. Preuve du refus dans
cx-grep-symboles-d7525d9cb-cible-20261113/cible.log (ligne1096 du panic),
primitive validation-j3lg1vlf rouge et quiescence target[]. Aucun vert de suite
native revendiqué. Nouvelle source/pilote dans le même RESTE1800 initial.

## Ciblé de la suite grep qualifié

Source ea7dcbb0b4bb15f89c6f8d7c3a7a2f542afee428 : navigation12/0/0 en1,14s,
grep2/0/0 en0,07s. Vrai chat du framework à modèle de fixture CallbackLlm,
outillage et index réels ; relais atelier, puis stale_index sans réindexation
implicite. Aucun fournisseur de modèle extérieur appelé. Preuves aux deux postes :
cx-grep-symboles-ea7dcbb0b-cible-20261113, ELF d5b8dd4ca105a0588315f2916869921de0f6abb57627233d3a28b38e2b509fc9,
223gabarits/aucun absent, PID2370583/start87775220/native29/Max32.
Primitive validation-1_rx2ina VERT0/restes[]/nettoyé=true, quiescence target[].
Les deux rouges précédents restent distincts et conservés. Pas de globale ni
d'intégration de cette suite avant revue/tour explicite C7.

## Recette globale autorisée — avant exécution

GO C7 pour une portée6000 distincte du ciblé1800 terminé. Landing garde Recherche
e16768186 relu, SHA6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c ;
rebase branche neuve3, diff runtime ea7→HEAD vide. ED features : contrat seulement
19cd9383d, aucune correction Rust disponible, combinaisons daemon seul/native+daemon
sans code non couvertes par cette recette et non revendiquées.

F5≤2100, vrai minimal0≤120, trois compiles liblarge/NAV/e2e sous borne partagée1500,
inventaires figés publiés avant lib≤900 (dont navigation12+grep2), NAV40≤600,
lectures27/hôte13/PG36 et filets2 chacun≤120. Ensemble sous une primitive/j8/
Max32G/target32Go, native29 fixe, CPU, aucune API ni nouveau modèle GPU.
Attente/coordinations et nettoyage inclus dans6000, pas de reset après un rouge.
PG neuve privée cx_archi_grep_symboles_20261113 : absence avant création, identité/
propriétaire/OID enregistrés ; dump+SHA puis zéro session avant suppression finale.
7879 seulement pour lectures après coordination, santé avant/après, rendu immédiat.
Trois ELF/223gabarits réels/native29 prouvés par PID/starttime/maps ; premier rouge
stop/diagnostic, aucun master avant revue et tour explicite C7.

## Arrêt global avant e2e — âge natif, 13 novembre

Source6786 : F5/minimal0/3compiles/inventaires, lib2278/0/66 en38,36s et
NAV40/0/0 en131,66s verts. Le lanceur e2e_lectures sort1 AVANT filets/tests :
native29 et vector sont plus anciens que le commit207701bb2 du formatage C++18.
Lib29 reste celle autorisée et fixée ; ni rebâti C++ ni substitution. Primitive
validation-3l2_x1i5 rouge/code1/restes[]/nettoyé=true, quiescence target[].
Les trois e2e ne sont pas verts ni exécutés ; globale rouge conservée dans
cx-grep-symboles-6786c03b5-globale-20261113. Santé BGE avant/après arrêt identique,
7879 rendu, aucun appel sparse de test n'a eu lieu.

La procédure push ligne91 prescrit RAG3WEAVER_MOTEUR_ANCIEN=1 dans ce cas.
Relecture avant application : tests/e2e_lectures.rs832 utilise AUSSI ce drapeau
pour forcer « non jugé » du témoin la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel,
indépendamment de la capacité réellement exposée par SHOW_INDEXES. Ne pas masquer
cette absence par le seul total27 marqué ok. La passe d006 conservée, sous la même
native29 sans ce drapeau avant le formatage, avait réellement joué ce témoin sans
ligne « non jugé ». Cela ne dispense pas de sa qualification sur la source actuelle.

Recette corrigée à faire relire : drapeau ancien explicite pour le lanceur, puis
même témoin exact de l'ELF e2e FIGÉ exécuté sans ce drapeau, sous la même portée et
les mêmes sources/paramètres, 1/0/0 attendu et refus de toute ligne « non jugé ».
Cela conserve la couverture de la suite sans modifier le lanceur ou les assertions
possédées par Recherche. Les filets2 et le27/0/0 restent prouvés par run_e2e ; le
non-jugé du premier appel et sa compensation sont conservés distinctement dans
les preuves. Composant natif supplémentaire≤120s, santé/rendu7879 après les deux.
Source et pilote neufs publiés avant reprise complète dans le RESTE de6000 initial
(deadline884429.442394705), pas de reset ; PG réemployée uniquement après identité/
propriétaire/OID identiques à la création. Pas de reprise avant revue de ce point.
