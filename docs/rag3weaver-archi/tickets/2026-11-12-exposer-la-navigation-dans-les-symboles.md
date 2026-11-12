# Exposer la navigation dans les symboles

- **État** : ouvert ; première tranche codée et ciblé8/0/0 prouvé, passe globale/revue/intégration à terminer ; grep et couverture étendue restent à livrer.
- **Famille** : produit
- **Gravité** : capacité manquante ; l'agent doit pouvoir explorer des arbres de code sans lire tous les corps.
- **Atteignable en service** : oui dans la branche candidate via outil code et relais atelier, pas encore intégré à master ; outline reste disponible.
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
