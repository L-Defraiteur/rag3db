# Brouillon de vision — montrer nos produits : démos publiques, graphes visibles

4 octobre 2026. **Brouillon** : rien n'est décidé ni codé ; Lucie termine par
« je ne sais pas », « on y réfléchira ». Ce document garde les idées, pas un
plan. Il suit le brouillon du dépôt de référence
(`extension/rag3weaver/visions/2026-10-04-16h17-depot-de-reference.md`).

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
  (`extension/rag3weaver/visions/2026-10-04-00h09-memoires-par-gabarit-et-fiches-qui-pointent-vers-tout.md`).
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

- **Un service est un petit graphe partagé** (Lucie) : un sous-graphe nommé,
  que plusieurs contrôleurs appellent. À vérifier : ce que le moteur de
  graphes sait déjà de la composition (un graphe appelé comme un nœud).
- **L'authentification est fournie, pas écrite** (Lucie : « des builtins,
  peut-être, avec OAuth Google ou OpenID, webhooks HMAC ») : des gardes
  intégrées, déclarées au manifeste sur un contrôleur (qui peut entrer, par
  quelle preuve), jamais un graphe écrit par l'auteur du backend — le code de
  sécurité se vérifie une fois, au même endroit. Un webhook signé est une
  quatrième façon d'entrer dans un contrôleur, après la route, l'outil et
  l'événement.

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

## 9. Les secrets et les intégrations (brouillon, notre conception)

L'idée de Lucie :

> « Tu parles à un agent, tu lui remplis des variables dont il ne voit que le
> nom, et il les pose dans le registre sans jamais pouvoir les lire. »

### Le registre des secrets

- **L'agent connaît des noms, jamais des valeurs.** Il peut demander qu'une
  variable existe (« j'ai besoin de `STRIPE_KEY` »), savoir si elle est
  posée, la référencer dans un graphe. Il ne peut pas la lire.
- **La valeur ne passe pas par la conversation.** La personne la saisit dans
  un champ à part de l'interface ; l'agent ne reçoit que « posée ». Une
  valeur tapée dans le fil entrerait dans le contexte et dans le journal.
- **Un seul exécuteur détient les secrets.** Le harnais résout la référence à
  l'exécution du nœud, juste avant l'appel ; la valeur n'est ni dans le
  contexte, ni dans le journal, ni dans un résultat d'outil, ni dans une
  fiche.
- **Un secret est lié à sa destination.** Ne pas pouvoir le lire ne suffit
  pas : il ne doit pas pouvoir sortir. Chaque secret déclare où il a le droit
  d'aller (ce domaine, cette méthode d'authentification) ; un nœud qui
  l'enverrait ailleurs est refusé, à la déclaration et à l'exécution.
- **Ce qui revient au modèle est filtré.** La réponse d'un appel peut porter
  un jeton (une session, un rafraîchissement) : ce que le graphe en garde
  pour lui se déclare, et ne revient pas dans le contexte.
- **Stockage** : chiffré, des clés séparées par genre de secret, une portée
  dite (la personne, le projet, l'organisation). L'interface ne montre que
  « configuré » ou non.

### Les intégrations

- **Une intégration est déclarée, pas codée** : un manifeste dit ses méthodes
  d'authentification, ses actions (des contrôleurs ou des nœuds), ce qu'elle
  demande à la personne. C'est la règle de généricité appliquée aux services
  extérieurs.
- **Des méthodes d'authentification nommées**, fournies par le moteur et
  référencées par nom : clé d'API, jeton porteur, OAuth2, OpenID avec
  vérification de la signature, webhook signé. Jamais un graphe écrit par
  l'auteur du backend (§ précédent : l'authentification est fournie).
- **OAuth qui tient la concurrence** : rafraîchissement au moment de l'appel,
  cache, un seul rafraîchissement à la fois.
- **Les webhooks entrants sont signés et horodatés dès la première version**,
  avec une fenêtre contre le rejeu et une clé d'idempotence ; un webhook est
  une entrée de contrôleur comme une autre.
- **Les appels sortants** passent par une garde d'adresse (pas d'adresse
  interne, pas de redirection hors de la destination déclarée), des délais
  bornés, et un journal sans secrets.

### Ce qui reste à débattre

- la portée d'un secret quand plusieurs agents travaillent sur le même
  projet ;
- qui peut déclarer une intégration nouvelle : la personne seulement, ou un
  agent avec confirmation (comme pour un gabarit de mémoire) ;
- ce qu'on montre à la démo publique : aucune intégration à secrets pour un
  visiteur, ou seulement des intégrations fournies par nous.

## 10. Un agent ajoute une capacité, et la personne nous l'envoie

> « Comme chez nous tout vit dans un genre de gloubi-boulga incluant la base,
> on aurait moins de mal pour qu'un agent, à la volée, rajoute des capacités à
> notre version de l'integration kit, et nous envoie, sur accord de
> l'utilisateur, le module pour contribuer. Un genre de PR, mais version
> rag3weaver. » — Lucie

Pourquoi c'est plus facile chez nous : une capacité est **déclarée** — un
manifeste, des graphes, parfois un script rhai — et non du code compilé. Elle
se crée à chaud, se vérifie comme un manifeste l'est déjà, et se transporte
comme des données.

Ce que serait un module envoyé :

- la déclaration (manifeste, graphes, scripts), ses exemples d'appel rejouables
  sur des réponses enregistrées, et la fiche qui dit pourquoi il existe (la
  boucle étrange : l'agent qui conçoit laisse sa trace) ;
- **jamais un secret** : le registre est à part, donc par construction rien
  n'en sort ; les réponses enregistrées sont à nettoyer avant l'envoi ;
- les destinations réseau qu'il demande, lisibles en tête.

Le parcours :

1. l'agent conçoit la capacité pour la personne ; elle sert tout de suite,
   chez elle ;
2. l'agent propose de la partager ; la personne voit **exactement** ce qui
   partirait, et décide — jamais d'envoi sans son accord ;
3. chez nous : vérification automatique (manifeste, liste blanche des nœuds,
   destinations déclarées, exemples rejoués), puis relecture humaine avant
   toute mise au catalogue.

Points en débat :

- **C'est une chaîne d'approvisionnement** : un module malveillant déclarerait
  une destination à lui. Les gardes du §9 valent pour un module reçu comme
  pour un module écrit ici ; le catalogue est relu, signé, et un module non
  relu se dit comme tel.
- **Le même mécanisme vaut pour tout ce qui se déclare** : un gabarit de
  mémoire, une vue, un contrôleur, un genre de référence — « partager une
  chose déclarée », pas seulement une intégration.
- **La licence** : sous quelles conditions une contribution entre dans un
  produit sous LRSL ; à écrire avant d'ouvrir le canal.
- **Ce qui ne se partage pas ainsi** : un nœud nouveau en Rust reste une
  contribution de code, par la voie ordinaire.

## 11. Jamais de compilation pour la personne, et le rechargement à chaud

> « Est-ce que le code ne saurait pas se recharger, sans avoir besoin de
> relancer une compilation ? Si on fait tout ce qui est codable en rhai, les
> logiques de backend, l'utilisateur n'a potentiellement jamais besoin de
> compiler, non ? » — Lucie

Oui, c'est la conséquence de « tout est déclaré » :

- **Ce que la personne écrit n'est jamais compilé** : manifestes, graphes
  (`.mmd`), gabarits de rendu, scripts rhai. Le binaire est livré tout fait ;
  rhai est interprété, lu à l'exécution.
- **Le rechargement à chaud** : un fichier déclaré change, le système le
  revérifie comme au démarrage et remplace l'ancien d'un coup. S'il est
  invalide, l'ancien reste en service et l'erreur dit quoi corriger. Une
  exécution en cours finit sur l'ancienne version.
- C'est la condition de « un agent ajoute une capacité à la volée » (§10).

État : aujourd'hui un manifeste se charge au démarrage ; le rechargement est
à faire. La vérification des manifestes existe.

Limites à garder en tête :

- **rhai est lent devant du Rust** : bon pour la logique de liaison (décider,
  transformer un enregistrement, valider), pas pour une boucle lourde — le
  lourd reste dans des nœuds fournis.
- **Un changement de schéma n'est pas un simple rechargement** : une entité
  qui change de forme demande une migration des données ; c'est la partie
  difficile, à traiter à part.
- **Un nœud nouveau en Rust se compile** — par nous, pas par la personne.
- **rhai n'a accès qu'à ce qu'on lui donne** (ni fichiers ni réseau par
  défaut) : c'est aussi ce qui rend un module reçu acceptable.

### Et du code compilé chargé à la volée ?

> « Il n'y a pas une logique de DLL en Rust, des bibliothèques dynamiques
> chargées à la volée ? » — Lucie

Deux voies existent ; avis de l'orchestration, à vérifier avant de s'engager :

| Voie | Ce que c'est | Pour | Contre |
|---|---|---|---|
| bibliothèque native (`.so` / `.dll`) | du Rust compilé à part, chargé par le binaire | vitesse native, accès à tout (GPU compris) ; le moteur charge déjà ses extensions ainsi | Rust n'a pas d'interface binaire stable : il faut une frontière en C ou le même compilateur ; aucun bac à sable — un module fait ce qu'il veut et son plantage tue le processus ; décharger est délicat ; un fichier par système |
| module WebAssembly | du code compilé (Rust ou autre) exécuté dans un bac à sable embarqué | chargé et remplacé à chaud ; isolé — n'a accès qu'à ce qu'on lui donne ; un même fichier pour tous les systèmes ; proche de la vitesse native | une dépendance de plus ; les données traversent une frontière ; pas de GPU |

Trois étages possibles, du plus simple au plus puissant :

1. **rhai** — la logique de liaison, sans rien compiler ;
2. **WebAssembly** — une logique lourde ou écrite dans un autre langage,
   compilée par son auteur (ou par un agent), chargée à chaud, isolée :
   acceptable pour un module reçu ;
3. **bibliothèque native** — réservée à ce qui vient de nous ou que la
   personne compile pour elle-même ; jamais pour un module reçu.

Dans les trois cas la personne ne recompile jamais **notre** binaire.

### Un nœud sur mesure par assemblage, à la manière d'Unreal

> « Le problème, c'est si quelqu'un veut faire un nœud custom. Mais a priori
> on pourrait donner toutes les briques pour que ce soit faisable en
> assemblant des nœuds existants ? On ferait des nœuds if, for, etc., si ce
> n'est pas déjà fait — un peu à la Unreal Engine. » — Lucie

Ce qui existe (relevé du 4 octobre dans `src/`, par les noms seulement) : une
soixantaine de nœuds, et **`GraphNode`, un sous-graphe utilisé comme un
nœud** — ses ports libres deviennent ses entrées et ses sorties. Un « nœud
sur mesure » est donc déjà possible : un graphe enregistré sous un nom. C'est
aussi le « service = petit graphe partagé » du §« backend tout en graphes ».
Aucun nœud de contrôle (si, pour chaque) n'apparaît par son nom : à vérifier,
et sans doute à faire.

Les briques de contrôle, qui restent toutes un graphe sans cycle :

| Brique | Ce qu'elle fait |
|---|---|
| si / selon | envoie chaque enregistrement vers une sortie ou une autre, d'après une condition |
| pour chaque | applique un sous-graphe à chaque élément d'une liste |
| filtrer, regrouper, réduire | les opérations d'ensemble (certaines existent pour la recherche) |
| joindre | réunit deux flux sur une clé |
| essayer / sinon | une branche de repli quand un nœud échoue — et le repli se compte, il ne se tait pas |
| répéter, borné | un sous-graphe rejoué jusqu'à une condition, avec un nombre maximal de tours |

Points en débat :

- **Les nœuds pour le flux, rhai pour l'expression.** La leçon des graphes
  visuels : une addition ou une comparaison en nœuds devient vite illisible.
  La condition d'un « si », la transformation d'un champ sont une ligne de
  rhai dans le nœud — c'est « des scripts rhai dans certains nœuds ».
- **Ce qu'un nœud reçoit doit pouvoir venir d'un port**, pas seulement de sa
  configuration : aujourd'hui `RhaiNode` prend son script en configuration et
  `EntityBatchNode` ses enregistrements aussi (relevé par la session mémoire,
  qui a dû écrire deux nœuds pour cela). C'est la première chose à lever pour
  que l'assemblage suffise.
- **Pas de variables partagées** entre nœuds : les données passent par les
  ports. C'est ce qui garde un graphe lisible, vérifiable, rejouable.
- **Quand l'assemblage ne suffit pas** : WebAssembly (section précédente),
  pas un nœud Rust.

### Quel langage de script dans les nœuds ?

> « Il n'y a pas un autre langage de script plus performant que rhai pour
> décrire des nœuds ? » — Lucie

Avis de l'orchestration, de connaissance générale — **rien n'est mesuré chez
nous**, à vérifier avant de choisir :

| Langage | Vitesse | Isolation | Intégration à Rust | Connu des modèles |
|---|---|---|---|---|
| rhai (l'actuel) | la plus faible : il interprète l'arbre du script | sûre par défaut | native, aucune dépendance | peu |
| Lua / Luau | bien plus rapide ; très rapide avec un compilateur à la volée | bonne si l'on restreint la bibliothèque ; Luau est fait pour cela | une dépendance en C | très bien |
| Rune | plus rapide que rhai (machine à code intermédiaire) | bonne | native | très peu |
| JavaScript embarqué | de modéré (petit moteur) à très rapide (gros moteur, lourd) | bonne | dépendance moyenne à lourde | très bien |
| Starlark | modérée | déterministe et fermé par conception | native | assez bien (proche de Python) |
| WebAssembly | proche du natif | la meilleure | une dépendance ; il faut compiler | sans objet |

Ce qui pèse dans le choix :

- **la vitesse compte peu pour une expression** (la condition d'un « si ») ;
  elle compte pour un script appelé sur chaque enregistrement d'un gros flux ;
- **qui écrit les scripts** : si ce sont des agents, un langage que les
  modèles connaissent bien fait moins de fautes — c'est peut-être l'argument
  le plus fort contre rhai, avant la vitesse ;
- **le langage peut être un point de branchement**, comme le moteur de plein
  texte : un nœud de script déclare son langage, rhai reste le défaut tant
  qu'une mesure ne dit pas autre chose.

### Un moteur de script générique, et le nœud entièrement scripté

> « Donc WebAssembly et TypeScript ? Compiler, ça reste OK si on charge à la
> volée dès que c'est disponible ? Ça reste compiler un petit module à chaque
> fois. Complètement OK pour que le "moteur de scripting" reste générique ;
> le seul truc, c'est que je veux qu'on puisse déclarer un nœud 100 %
> scripté. » — Lucie

**Retenu par Lucie** : le moteur de script est générique (le langage est un
point de branchement), et un nœud peut être déclaré entièrement en script.

**Le nœud entièrement scripté** — ce que ce serait :

- une déclaration : un nom, des ports d'entrée et de sortie avec leur schéma,
  une configuration, un langage, et le corps du script ;
- une seule fonction à écrire : elle reçoit les entrées et la configuration,
  elle rend les sorties ;
- vérifié comme tout nœud (ports, schémas) avant d'entrer dans un graphe, et
  rechargé à chaud ;
- il n'appelle le système que par ce qu'on lui donne : requêter, journaliser,
  nommer un secret (jamais le lire), appeler l'extérieur par l'exécuteur
  gardé.

`RhaiNode` existe ; il manque la déclaration du nœud sous un nom, avec ses
ports, pour qu'il se réutilise comme un nœud fourni.

**Le moteur générique** : une seule interface — préparer un script, l'appeler
avec des entrées, recevoir des sorties — que chaque langage remplit. Le nœud
scripté ne sait pas lequel tourne dessous.

**TypeScript et WebAssembly** (avis de l'orchestration, à vérifier) :

- **TypeScript ne se compile pas directement en WebAssembly.** Deux chemins :
  un langage voisin fait pour cela (un sous-ensemble, pas du vrai
  TypeScript) ; ou embarquer un moteur JavaScript dans notre binaire et
  retirer les types à la volée — un outil écrit en Rust le fait en quelques
  millisecondes. Par le second chemin, la personne écrit du TypeScript et ne
  compile rien.
- **WebAssembly demande de compiler**, mais un petit module : quelques
  secondes, puis chargé à chaud. Ce que cela coûte vraiment : la personne (ou
  l'agent) doit avoir une chaîne de compilation sur son poste. C'est donc
  l'étage pour une logique lourde, pas le chemin ordinaire.

Ce qui donnerait : rhai aujourd'hui ; TypeScript par un moteur embarqué quand
on veut un langage que les modèles écrivent bien ; WebAssembly pour le lourd.

## 12. Les migrations de schéma, en verbes

> « Peut-être en verbes aussi, comme quand un agent cherche, non ? » — Lucie,
> 5 octobre, à propos des migrations du schéma de l'utilisateur.

Ce qui existe : le moteur valide et journalise un changement de schéma comme
toute écriture ; rag3weaver garde une version de schéma et ajoute à
l'ouverture ses colonnes internes manquantes. Ce qui manque : des migrations
du schéma de l'utilisateur — ordonnées, rejouables d'une base à l'autre, avec
la transformation des données.

L'idée : une migration n'est pas un script libre, c'est une suite de **verbes**
d'un petit vocabulaire, comme les outils de recherche le sont pour chercher.

| Verbe | Ce qu'il fait | Son inverse |
|---|---|---|
| ajouter un champ | avec une valeur par défaut ou calculée | retirer le champ |
| renommer un champ, une entité, une relation | sans toucher aux données | renommer dans l'autre sens |
| retirer un champ | — | aucun : perte, à dire |
| changer un type | par une expression (un script d'une ligne) | l'expression inverse, si elle existe |
| scinder ou fondre une entité | les lignes et leurs relations suivent | l'autre verbe |
| ajouter ou retirer une relation | — | l'autre verbe |

Ce que cela donne (proposition de l'orchestration, en débat) :

- **Chaque verbe sait s'il perd quelque chose et s'il se défait** : une
  migration dit d'avance « sans perte, réversible » ou « retire 3 champs,
  irréversible ».
- **Un essai à blanc avant d'appliquer** : combien de lignes touchées, ce qui
  serait perdu, ce qui devra être rebâti (plein texte, vecteurs, découpe) —
  un nœud d'essai à blanc existe déjà dans le moteur de graphes.
- **Une confirmation, pas un refus** : l'agent propose la migration par un
  outil, la personne voit l'essai à blanc et décide.
- **Appliquée dans une transaction**, puis le schéma déclaré se recharge à
  chaud ; ce qui est dérivé se rebâtit par l'état d'index (« mots : en
  cours »), jamais un vide muet.
- **Les migrations appliquées sont des lignes** : un historique adressable,
  qu'une fiche de mémoire peut citer (« pourquoi ce champ a été renommé »),
  et qu'une autre base peut rejouer dans l'ordre.
- **Un module partagé (§10) porte ses migrations** : mettre à jour une
  capacité reçue, c'est rejouer ses verbes.

**Des migrations indépendantes de la base** (Lucie, 5 octobre) : « ce qui est
fou en plus, c'est qu'on aurait des migrations indépendantes du provider de
BDD — rag3db, Postgres ou d'autres. »

- Un verbe dit **quoi** (« renommer ce champ »), pas **comment** : chaque
  base le traduit dans son langage. rag3weaver a déjà cette couche pour ses
  requêtes (le dialecte) ; les verbes de migration passeraient par elle.
- Le même historique de migrations se rejouerait donc sur une autre base, et
  un module partagé (§10) ne dépendrait pas de la base de celui qui le
  reçoit.
- **Ce qui est portable, c'est le vocabulaire, pas les garanties** : chaque
  base déclare ce qu'elle sait faire d'un verbe — en une transaction ou non,
  réversible ou non, avec ou sans réécriture de la table. L'essai à blanc le
  dit avant d'appliquer ; un verbe qu'une base ne sait pas faire sûrement est
  refusé en le disant, pas approché.
- Ce qui est propre à nous, au-delà d'un outil de migration ordinaire : le
  schéma n'est pas que des tables — les relations, le plein texte, les
  vecteurs et la découpe suivent le verbe, et se rebâtissent par l'état
  d'index.

## 13. Voir une exécution comme un graphe

> « Si tout fonctionne en DAG, on doit pouvoir visualiser le pas d'exécution
> en général en graphe, vu que tout est un workflow au final. » — Lucie,
> 5 octobre

- **Une exécution est un graphe parcouru** : le même dessin que le graphe
  déclaré, avec pour chaque nœud ce qui s'est passé — joué, sauté, échoué,
  sa durée, ce qui est entré et sorti (en compte, puis en détail au clic).
- **La matière existe** : le moteur de graphes tient déjà ses exécutions et
  l'état de chaque nœud dans des tables de la base (`_DataflowExecution`,
  `_DataflowNodeState`), et il a un nœud de trace. Il manque le rendu.
- **Un pas d'exécution devient une adresse** (un genre de plus, à côté de
  `row:`, `query:`, `graph:`…) : cliquer sur un nœud ou une arête d'une
  exécution ouvre ce que l'agent ou le traitement avait en main à ce moment —
  idée venue d'un échange de Lucie avec Gemini, gardée parce qu'elle rend la
  trace explorable au lieu d'être un journal qu'on lit.
- **Valable partout** : un outil appelé par un agent, un contrôleur appelé
  par une route, une ingestion, une migration — c'est le même afficheur.
- Du même échange, une piste de navigation : depuis n'importe quel nœud,
  « ce qui ressemble à ça » (ses voisins par le sens), à côté des relations
  et des mots. Chaque morceau a déjà son vecteur.

## 14. Éditer son site déjà déployé, sur place, avec un agent

> « Le mec pourrait éditer son site web sur connexion admin : il vient, il
> clique sur un bouton que lui seul voit, et hop l'interface d'édition se
> lance, il édite son site déjà déployé en temps réel avec un agent. » —
> Lucie, 5 octobre (vision née d'un échange avec ChatGPT)

Pourquoi c'est à notre portée, si le backend est déclaré (§8 à §12) : le site
est fait de contrôleurs, de vues, de gabarits et de scripts — des choses
déclarées, rechargées à chaud, sans compilation. L'agent qui édite le site
n'écrit pas dans un dépôt à redéployer : il change des déclarations, et le
site change.

Ce que cela demande (proposition de l'orchestration, en débat) :

- **Le bouton que seule l'administratrice voit** est une garde fournie
  (§9) : l'interface d'édition n'existe pas pour un visiteur.
- **On n'édite pas ce que les visiteurs voient** : l'administratrice et
  l'agent travaillent sur un brouillon du site, qu'elle seule voit en place ;
  les visiteurs gardent la version publiée. Publier est un geste, avec son
  résumé de ce qui change.
- **Chaque changement est un commit** d'une chose déclarée, avec sa raison —
  les fiches de fichier de la vision des fiches de contexte
  (`2026-10-04-14h58-…`) ; revenir en arrière est un geste aussi.
- **Un changement de données passe par les migrations en verbes** (§12),
  avec leur essai à blanc ; jamais par un script libre sur la base en
  service.
- **L'agent ne voit aucun secret** du site (§9), et ce qu'il peut déclarer
  est borné par ce que le site lui permet.
- **Ce qui casse se voit avant de publier** : la vérification des
  déclarations, et les exemples rejoués des contrôleurs touchés.

C'est le produit « à la Lovable » du §1, retourné : au lieu de bâtir un site
dans un outil puis de le déployer, le site déployé porte son propre outil.

### Une page de démo que tout le monde édite en même temps

> « On laisse une deuxième page collaborative où les gens contribuent tous à
> l'édition d'une même démo initiale, qui se recharge tous les jours, peut-être
> à zéro ; mais au moins dans une journée tu vois les éditions des autres, ce
> qu'ils ont dit, leurs sessions. Édition en réseau d'un site. Imagine une
> équipe de demain qui parlent tous à un site déjà déployé. » — Lucie,
> 5 octobre

Ce que ce serait : une page publique, remise à son état de départ chaque
jour, que chaque visiteur modifie en parlant à un agent ; chacun voit les
changements des autres arriver, avec ce qui a été demandé.

Pourquoi c'est une bonne vitrine :

- **elle se montre toute seule** : une page qui change sous les yeux, et le
  fil de ce que les gens ont demandé, se partage sans explication ;
- **elle prouve le plus dur** : plusieurs agents qui écrivent en même temps
  dans le même système, sans se marcher dessus — c'est la quatrième condition
  de la stèle du moteur (les écritures parallèles) rendue visible ;
- **elle montre l'équipe de demain** : plusieurs personnes, chacune avec son
  agent, sur un même site en service.

Ce qu'elle demande :

- **les écritures parallèles** du moteur, et la présence entre agents de la
  vision des fiches (qui a un curseur sur quoi, un verrou d'intention qui
  annonce sans bloquer) ;
- **une règle quand deux demandes se contredisent** : la dernière gagne, ou
  l'agent le dit et propose — à débattre ;
- **un bac très fermé** : aucun secret, aucun appel vers l'extérieur, des
  quotas par visiteur, une page isolée du reste ;
- **de la modération** : un espace que tout le monde édite attire les abus ;
  la remise à zéro quotidienne limite les dégâts, elle ne suffit pas ;
- **dire aux gens que leurs demandes sont visibles** de tous avant qu'ils
  écrivent.

### Parler à son site, à voix haute

> « Ça ferait vraiment futuriste. On mettrait du TTS/STT local, avec Whisper
> ou quoi ; peut-être le builtin du navigateur pour le TTS, et un STT valable
> multilingue pour la voix. » — Lucie, 5 octobre. Ses images : l'ordinateur
> de Star Trek — on lui parle, et la visualisation arrive sous les yeux.

- **Entendre (la voix vers le texte) est la moitié qui compte.** Un modèle
  multilingue du genre Whisper, servi par nous ou tourné en local : c'est une
  capacité de plus à déclarer, comme les autres (`models.<capacité>` : en
  service ou en local). La reconnaissance intégrée aux navigateurs existe,
  mais elle n'est pas dans tous, et chez certains l'audio part chez un
  tiers : pas une base sûre.
- **Parler (le texte vers la voix) peut commencer par le navigateur** : sa
  synthèse intégrée est gratuite et immédiate, de qualité moyenne. Suffisant
  pour une démo ; un modèle à nous plus tard si la voix devient un produit.
- **Le site montre, il ne raconte pas** : la réponse d'une demande est
  d'abord ce qui change à l'écran ; la voix dit une phrase (« c'est fait »,
  « je propose ceci »), pas un compte rendu.
- **À prévoir** : un geste pour parler (appuyer pour dire) plutôt qu'une
  écoute permanente ; dire à la personne que sa voix part vers un service
  quand ce n'est pas local ; le coût de la reconnaissance par visiteur dans
  le quota de la démo.

Avis de l'orchestration, de connaissance générale ; rien n'est essayé.
