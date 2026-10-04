# Brouillon de vision — montrer nos produits : démos publiques, graphes visibles

4 octobre 2026. **Brouillon** : rien n'est décidé ni codé ; Lucie termine par
« je ne sais pas », « on y réfléchira ». Ce document garde les idées, pas un
plan. Il suit le brouillon du dépôt de référence
(`../4-octobre-2026-16h17/01`).

## 1. Les idées de Lucie

> « Ça pourra servir pour présenter nos produits aussi, ce truc de référence :
> montrer des démos aux gens sur un site web. Ou bien l'agent pour Magic, pour
> construire des decks, en enlevant le filtre "ma collection", en se basant
> uniquement sur la base totale de cartes MTG Arena. Et peut-être une
> visualisation de DAG — ça c'est une vision à part entière : en faire un
> genre de n8n, avec des scripts rhai dans certains nœuds ; peut-être des
> Composio ou des trucs du genre à brancher dessus pour la démo. Le tout avec
> très peu d'usage max par visiteur par jour, genre deux decks Magic
> construits par jour. »

> « Ou une démo d'édition de code quand même, mais très limitée, on y
> réfléchira. Peut-être qu'un Lovable serait encore plus rentable que des
> présentations hasardeuses, si l'utilisateur voit les fiches produit
> s'afficher. Le problème c'est le GPU qu'il faut pour les embeddings : il
> faudrait petit à petit louer un Modal ou autre ; d'abord servir depuis mon
> gros PC aux deux R9700. »

En liste :

1. **Des démos publiques sur un site**, appuyées sur des dépôts de référence
   (en lecture seule).
2. **L'agent Magic** sur la base entière des cartes, sans le filtre « ma
   collection », pour construire des decks.
3. **La visualisation des graphes de traitement**, puis un éditeur à la n8n :
   des nœuds, des scripts rhai dans certains, des connecteurs externes (du
   genre Composio) à brancher. Vision à part entière.
4. **Un quota très bas par visiteur et par jour** (deux decks par jour).
5. **Une démo d'édition de code, très limitée.**
6. **Un produit à la Lovable** plutôt que des présentations : la personne
   construit quelque chose et voit les fiches s'afficher.
7. **Le GPU des embarquements** : servir d'abord depuis le poste aux deux
   R9700, louer ensuite (Modal ou autre), petit à petit.

## 2. Les idées ajoutées par l'orchestration

- **Montrer la trace, pas seulement la réponse** : les résultats trouvés,
  l'arbre des liens, le graphe qui a tourné. C'est le premier usage de la
  visualisation des graphes, en lecture seule, bien avant un éditeur.
- **Le comparatif côte à côte** : la même question à un agent qui n'a que
  grep et à un agent avec nos outils — appels, jetons, temps. À mesurer
  honnêtement, sur des questions qui ne sont pas choisies pour gagner.
- **Des rejeux enregistrés** pour la plupart des visiteurs : des questions
  choisies, jouées une fois, rediffusées avec leur trace. Coût nul par
  visite ; le direct reste pour le quota.
- **« Interroge un dépôt connu »** : une référence sur un projet que les gens
  connaissent, pour qu'ils jugent par eux-mêmes.
- **La mémoire qui se souvient du visiteur** d'un jour à l'autre.
- **Magic comme preuve de généricité** : le même moteur sur des cartes et sur
  du code, rien en dur ; le deck expliqué par les relations entre cartes.

## 3. Ce que chaque piste demande

| Piste | Ce qui existe | Ce qui manque |
|---|---|---|
| rejeux enregistrés avec trace | le journal de session, le rendu des outils | une page qui rejoue un journal ; la trace du graphe exécuté |
| démo en direct sur une référence | index, recherche, outils en lecture | la référence comme source (brouillon 16 h 17), un quota, un service exposé |
| agent Magic sans « ma collection » | la base MTG (à refaire, décision du 4 octobre), les outils de recherche | la base refaite, le gabarit de l'agent sans le filtre |
| graphes visibles | les graphes sont des fichiers `.mmd` | un rendu, puis l'exécution montrée nœud par nœud |
| éditeur à la n8n | les nœuds, `RhaiNode`, les manifestes vérifiés | l'édition, les connecteurs, la vérification en direct |
| édition de code limitée | le bac à sable Landlock, la garde des liens | un espace jetable par visiteur, des bornes de temps et de taille |
| produit à la Lovable | l'agent de code, les fiches de contexte (brouillon 14 h 58) | presque tout le produit autour : aperçu, hébergement, comptes |

## 4. Les points durs

- **Un visiteur n'a que des outils en lecture**, sauf dans un espace jetable
  et borné ; jamais `run_command` hors bac à sable.
- **Un quota par visiteur se contourne** s'il repose sur l'adresse seule : un
  compte, et un plafond global par jour.
- **Le coût par essai** : le modèle (l'agent se juge avec Gemini) et les
  embarquements. Une référence déjà indexée ne coûte que les embarquements
  des questions ; indexer le dépôt d'un visiteur coûte bien plus.
- **Le GPU** : le service d'embarquement distant existe déjà, par tunnel et
  sans authentification — à ne pas exposer tel quel ; l'authentification
  était « pour plus tard », une démo publique la rend nécessaire.
- **Les droits** : la licence des dépôts indexés en démo ; les conditions de
  Wizards of the Coast pour les données et les images de cartes.
- **Un produit à la Lovable est un marché encombré** : ce qui serait à nous,
  c'est le contexte tenu autrement (fiches vivantes, graphe du code,
  mémoire) — à montrer d'abord, avant de bâtir le reste du produit.

## 5. Un ordre possible, à débattre

1. Des rejeux enregistrés avec la trace, sur un dépôt de référence : le moins
   cher, le moins risqué, et déjà la visualisation en lecture seule.
2. Le direct avec quota, en lecture seule ; puis Magic.
3. L'édition limitée dans un espace jetable.
4. Les graphes éditables ; le produit à la Lovable — chacun une vision à
   écrire à part quand elle mûrit.

## 6. Le positionnement, dit par Lucie (4 octobre, 16 h 30)

> « On vend une interface de code locale, "no IDE needed" : le remplaçant d'un
> IDE moderne, mais orienté purement "tu parles à un agent, tu explores des
> fiches et un graphe de code". On vend des services d'embedding. On ne vend
> pour l'instant pas grand-chose d'autre. Et on met en valeur avec une démo à
> la Lovable accessible gratuitement, surtout pour démontrer la force des
> fiches de contexte. »

Donc trois pièces :

| Pièce | Rôle | Qui paie |
|---|---|---|
| l'interface de code locale, sans IDE | le produit | la personne qui développe |
| le service d'embarquement | le revenu récurrent, et ce qui rend le premier index rapide sans GPU chez soi | à l'usage |
| la démo à la Lovable, gratuite | la vitrine : faire voir les fiches de contexte | nous (quota bas) |

Points en débat :

- **« Local » et « service d'embarquement » tirent en sens inverse** : avec le
  service, des morceaux de code quittent le poste. À dire clairement, et à
  laisser au choix — le modèle se déclare déjà en service ou en local
  (`models.embed`), avec le petit modèle pour un GPU faible.
- **L'embarquement seul se vend peu cher partout** : ce qui se vend, c'est
  l'ensemble (premier index en minutes, sans réglage). Le produit est
  l'interface ; le service en est le confort.
- **« Sans IDE » est une promesse large** : relire un diff, lancer, déboguer,
  git. Les fiches de fichier avec leurs commits couvrent la relecture ; le
  reste est à lister avant de l'écrire sur une page. Variante prudente :
  « à côté de ton IDE » d'abord.
- **L'interface n'existe pas encore** : le moteur, les outils et le chat
  existent ; les fiches, le graphe et leur exploration à l'écran sont à
  bâtir. C'est le plus gros chantier de ce positionnement.
- **La démo doit montrer ce qui est à nous** : une tâche assez longue pour
  que les fiches fassent la différence (un contexte qui ne se perd pas), pas
  une page générée en un tour.
- **Le modèle de langage** : à la clé de la personne, ou revendu ? Aujourd'hui
  l'agent se juge avec Gemini ; rien n'est décidé sur qui le paie.

## 7. Ce que la personne voit dans l'interface

> « Tu lances tes propres recherches et tu vois de beaux graphes bien
> construits, se basant sur les arbres ASCII que verrait l'agent ; ou bien un
> agent construit la recherche et l'affichage pour toi en mermaid. Je ne sais
> pas. » — Lucie

Deux voies, qui ne s'excluent pas :

- **La personne cherche elle-même** : les mêmes outils que l'agent, rendus en
  graphe à l'écran.
- **L'agent compose une vue** pour une question (« montre-moi comment un
  instantané se finit ou s'annule ») et l'affiche.

Points en débat :

- **Une donnée, deux rendus.** L'arbre ASCII de l'agent et le graphe de la
  personne sortent du même résultat structuré ; on ne dessine pas en relisant
  l'ASCII. La personne voit alors exactement ce que l'agent a vu — c'est
  aussi l'outil de diagnostic demandé (« ce qu'il voit vraiment »).
- **L'agent choisit quoi montrer, pas quelles arêtes existent.** Un mermaid
  écrit librement par un modèle peut inventer un lien. L'agent appelle un
  outil (une requête sur le graphe) ; le rendu mermaid en est tiré de façon
  déterministe.
- **Une vue composée est une requête enregistrée** : elle se garde, se rejoue
  et se lie à une fiche — c'est l'adresse `query:` de la vision des mémoires
  (`../4-octobre-2026-00h09/01`).
- **Un même afficheur** pour les graphes de code et pour les graphes de
  traitement, qui sont déjà des fichiers mermaid (`.mmd`).
- **La taille** : mermaid tient quelques dizaines de nœuds ; au-delà il faut
  replier, ou un autre afficheur.

**Les références sont des liens** (Lucie) : « dans les fiches de code, les
références sont, autant que possible, des liens pour voir des choses réelles,
au clic. »

- Toute référence affichée est une **adresse** (`row:`, plage, `path:`,
  `query:`, `url:`, `schema:`, `graph:`) : le clic la résout — le scope dans
  son fichier, le diff d'un commit de fiche, la requête rejouée, le graphe
  affiché. C'est le même système d'adresses que la mémoire ; l'agent lit
  l'adresse en texte, la personne clique dessus.
- Le clic montre **l'état du jour**, et dit si la cible a changé depuis que la
  fiche en parle ; une cible disparue se dit, elle ne mène pas à une page
  vide.
- Un nom qui n'a pas d'adresse (cité par l'agent sans être résolu) ne se
  déguise pas en lien : c'est la différence visible entre « cité » et
  « vérifié » (`add_ref` rend déjà `verifiee: false`).

## 8. Autre sujet, à garder : un moteur de graphes pour le front aussi

> « Je ne sais pas comment, mais on a un moteur de DAG pour le back et le
> front — genre un DAG qui enveloppe React, je n'en sais rien. Je kifferais
> que les gens voient principalement des graphes. Mais ça c'est un autre
> sujet. » — Lucie

Gardé tel quel, sans réponse : l'interface elle-même décrite par un graphe
(comme les traitements le sont par des `.mmd`), et des graphes comme première
chose que la personne voit. À reprendre quand l'interface du §6 aura une
première forme ; rien à décider maintenant.

### Suite : un backend tout en graphes, un contrôleur est un graphe

> « Faire un genre de Next.js depuis rag3weaver ? Peut-être que le backend est
> tout en DAG : une route est un début de DAG, ensuite on référence une vue
> dedans, et une interface de données pour communiquer avec. Je n'en sais
> rien. » — Lucie

Ce que cela donne, mis en forme (proposition de l'orchestration, en débat) :

- **Un contrôleur est un graphe ; une route n'est que ce qui y mène** —
  précision de Lucie : « un contrôleur est un graphe, plutôt qu'une route un
  graphe. » La route est une adresse liée à un contrôleur, comme le nom d'un
  outil l'est pour un agent et l'abonnement pour un événement (réacteur).
  Trois façons d'entrer dans un même graphe ; plusieurs routes peuvent mener
  au même contrôleur.
- **La fin du graphe nomme une vue** : un afficheur générique (arbre, tableau,
  graphe, diff, fiche) qui reçoit le résultat typé. « Une vue est un outil
  dont le résultat va à l'écran. »
- **L'interface de données est déjà là** : les ports des nœuds sont typés et
  déclarés par schéma ; le contrat entre le front et le back en sort, sans
  l'écrire deux fois.
- **Vivant** : le réacteur pousse ce qui change vers la vue ouverte.
- **Les écritures** (un formulaire, une note posée sur une fiche) passent par
  un contrôleur, avec une politique par contrôleur comme il y a une
  politique par outil.

Ce qui ne serait pas un graphe : l'interface elle-même (les boucles clic →
état → écran sont le métier des bibliothèques d'interface), les sessions,
l'authentification, les fichiers statiques.

Deux façons de bâtir le front, la déclaration des vues étant la même :

1. sans React — rag3weaver rend du HTML par ses gabarits jinja comme il rend
   déjà du texte, mises à jour partielles poussées par le serveur, du
   JavaScript seulement pour dessiner les graphes ; un seul binaire ;
2. un front React mince — quelques afficheurs compilés une fois et servis par
   le binaire.

Premier pas possible : une seule vue, l'arbre des « Liens », servie en page
par rag3weaver. Ce n'est pas un cadre web général (routage, empaquetage,
hydratation) : ce n'est pas là qu'est notre valeur.
