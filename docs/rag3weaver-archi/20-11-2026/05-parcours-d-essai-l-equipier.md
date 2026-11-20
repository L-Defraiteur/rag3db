# Parcours d'essai de l'équipier — pour Lucie, le samedi 21 novembre 2026

La part d'archi du parcours d'essai, pour la page commune de tty : l'équipier de l'hôte, de bout en
bout. Ce qui suit vaut pour l'hôte posé ce soir. Ce qui n'y est pas encore est dit à la fin.

## 1. Une tâche, de bout en bout

- **Lancer.** Au terminal : `rag3weaver-tty --repo <dépôt> --task essai --request "<ta demande>"`.
  Dans l'IDE : la revue de tâche, puis « Écrire » avec le genre « demande ».
- **Attendu.**
  - La boîte montre `request`, puis le compte rendu « tâche ouverte : branche agent/essai… ».
  - Ensuite viennent les pas : un `step_done` à la fin de chacun (« fait : … »). Une demande qui en
    vaut la peine reçoit un mini-plan (`edit_plan`).
  - À la fin : `ended`, avec « fait sans plan » ou « plan accompli ».
- **Essayer.**
  - Une petite demande (« centre ce div »), qui se fait en un pas, sans plan.
  - Une demande en trois éléments, qui ouvre un plan et le suit pas à pas.
  - Une précision envoyée en cours de route : elle s'ajoute à la demande (« Follow-up »), elle ne la
    remplace pas.
  - Un « arrêt », qui coupe le pas en cours.

## 2. Le pas, et ce qu'il garde d'un pas à l'autre

- Chaque pas commence par la boîte et les plans. Il voit aussi ce que le pas d'avant a lu, écrit et
  lancé (« Previous step ») : il ne relit pas un fichier qu'il vient de lire.
- Un pas est borné par `teammate.*` : 60 appels d'outils, 40 appels au modèle, 64 000 jetons de
  contexte. Au-delà, il est prié de conclure par `step_done`.
- **Attendu** : pas de pas qui tourne en rond plus de trois fois. Au troisième pas de suite sans
  élément réussi, l'équipier s'arrête et le dit.

## 3. La porte des commandes et les accords

- Une commande de l'équipier passe par la porte de la tâche. Elle s'exécute dans un bac à sable
  (Landlock : il lit et écrit dans son dossier de tâche, rien ailleurs), et une commande lourde
  (cargo, cmake…) passe sous `poste lourd`.
- Une commande que la porte ne laisse pas passer d'elle-même devient une carte « accord demandé »,
  dans la boîte et au terminal. Un oui l'accorde pour toute la tâche, un non la refuse, et
  l'équipier continue sans attendre.
- **Essayer** : une demande qui fait lancer les tests (« joue les tests du crate X »). On voit la
  commande, sa sortie bornée et son code.

## 4. Une réponse coupée est relancée

- Si le modèle réfléchit trop et que sa réponse est coupée à son plafond de sortie avant d'appeler un
  outil, le pas ne s'arrête plus. Il reçoit une relance (« Your reply was cut… call one tool now, or
  step_done »), au plus deux fois par pas (`teammate.cut_relaunches_per_step`). La trace du pas dit la
  coupure et les jetons de raisonnement.
- **Attendu** : rare, puisque le modèle par défaut est désormais borné (point 5). Si ça arrive, on le
  lit dans la trace, et le pas continue.

## 5. Le modèle par défaut, à `low`

- Sans `models.llm` au manifeste, l'hôte prend Gemini sur Vertex, avec `reasoning: low`.
- **Le régler** :
  - pour un dépôt : `"models": {"llm": {"reasoning": "medium"}}` dans son manifeste, sans `model`
    (c'est le modèle par défaut de l'hôte, réglé) ;
  - pour tout l'hôte : `VERTEX_REASONING=medium` dans `rag3weaver-daemon.env`, comme `VERTEX_MODEL`.
- `/model` au terminal liste les modèles déclarés de la tâche.

## Ce qui n'y est pas encore (pour ne pas buter)

- Pas de `fetch` ni de recherche web, pas de cron, pas de navigateur (tty) : l'équipier ne sort pas
  sur le web et ne se réveille pas tout seul.
- Pas encore de palette par familles : l'équipier voit tous ses outils d'un coup.
- Pas encore le sous-agent à vues (en cours), ni `run` en fond avec sa notification : une commande
  longue tient le pas jusqu'à sa fin.
- Le mode pair (un équipier vu par les sessions Claude Code) existe, mais seulement avec
  `teammate.pair: true` au manifeste. Il pose une entrée dans `~/.claude/sessions` : à n'allumer
  qu'en le voulant.
- Si le raisonnement borné n'est pas encore atterri au moment de l'essai, le modèle par défaut n'est
  pas encore à `low`, et des réponses coupées peuvent finir un pas. Le lot est `raisonnement-borne`,
  dans la file du pousseur.
