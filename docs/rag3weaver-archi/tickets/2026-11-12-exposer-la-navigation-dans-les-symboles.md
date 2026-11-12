# Exposer la navigation dans les symboles

- **État** : ouvert ; conception autorisée par Lucie le 12 novembre, interfaces proposées et couverture actuelle à qualifier avant code.
- **Famille** : produit
- **Gravité** : capacité manquante ; l'agent doit pouvoir explorer des arbres de code sans lire tous les corps.
- **Atteignable en service** : non pour les nouvelles façades ; outline et les relations de parents existent déjà.
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
