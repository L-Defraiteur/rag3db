# Pourquoi Algareth n'écrit pas : le jeu de lucivy 03 de 20 h 50 — le jeudi 19 novembre 2026

c7 a demandé la cause avant tout rejeu de cas. Ce jeu est le premier de la soirée où la consigne,
cargo et Vertex étaient tous en ordre (8,7 s par appel en moyenne). Il mesure donc Algareth
lui-même. Verdict du banc : échoué (grille sur master, 46cb2a1cca). Algareth a trouvé le bon
endroit, `verify_stored` dans `stored.rs`, n'a rien écrit, et s'est arrêté sur « 3 pas de suite
sans élément réussi ».

Dossier : `~/.cache/rag3weaver-build/campagne/03-cas-de-campagne-la-requete-floue-longue-sans-positions/20261119-205010/`
(tablette). Les runs bruts sont sous `~/.cache/rag3weaver-build/campagne/maison/.local/share/rag3weaver/runs/`.

## Ce que dit la trace, pas par pas

| Pas | Appels au modèle | Outils | Contexte max | Fin du pas |
|---|---|---|---|---|
| 1 | 33 | 12 grep, 18 lectures, 2 commandes | 63 710 | **coupé, `MaxTokens`** |
| 2 | 31 | 20 lectures, 7 grep, 3 commandes, `step_done` | 72 886 | `step_done` : la cause est trouvée |
| 3 | 24 | 19 lectures, 4 grep | 49 306 | **coupé, `MaxTokens`** |
| 4 | 31 | 22 lectures, 6 grep, 2 commandes | 64 834 | **coupé, `MaxTokens`** |
| 5 | 27 | 20 lectures, 5 grep, 1 commande | 50 817 | **coupé, `MaxTokens`** |

Chaque appel coupé a dépensé, d'après ses événements `Consumed` :

| Pas | Raisonnement | Sortie visible | Total |
|---|---|---|---|
| 1 | 3 930 | 162 | 4 092 |
| 3 | 3 930 | 162 | 4 092 |
| 4 | 3 929 | 163 | 4 092 |
| 5 | 3 931 | 161 | 4 092 |

Le plafond de sortie du chat est de 4 096 jetons, raisonnement compris. Les 162 jetons visibles
sont la fin d'un raisonnement écrit en clair (« Attends ! Pourquoi… », des bornes en LaTeX), coupé
avant tout appel d'outil.

## La cause, en trois couches

1. **Le modèle réfléchit sans borne.** Le gabarit de code ne déclare aucun modèle de langage
   (`models.llm` vaut `null`). L'équipier tourne donc sur le modèle par défaut de l'hôte, sans
   `reasoning_effort`. Le code le documente déjà (`src/llm.rs`, la mesure de `ReasoningEffort`) :
   sur Gemini 3.x, sans effort déclaré, la réflexion s'étend jusqu'à remplir `max_tokens` et
   tronque la vraie réponse. Une déclaration (`reasoning` d'un modèle, `src/model_source.rs`) le
   règle, mais aucun gabarit livré ne la pose.
2. **Une réponse coupée termine le pas.** Un tour qui finit sur `MaxTokens` sans appel d'outil est
   traité comme une réponse finale : le pas s'arrête là, sans `step_done`.
3. **Un pas coupé compte comme un pas raté.** Un pas sans `step_done` réussi compte dans la règle
   des trois pas (`MAX_STALLED`, `src/daemon_host/teammate.rs`), même quand il a lu vingt fichiers
   et avancé. Trois coupures d'affilée (pas 3, 4 et 5) arrêtent l'équipier, au moment où il
   tenait la conception (pas 4 : « conception validée du fenêtrage par les pièces »).

Ce n'est donc ni une lecture qui ne débouche jamais sur une édition, ni un pas « lire » compté à
tort. C'est la règle des trois pas qui coupe avant le correctif, sur des pas que le plafond de
sortie avait déjà coupés.

## Le lot qui en découle, du plus déclaratif au moins

1. **Déclarer le modèle de l'équipier avec un raisonnement borné** dans le gabarit de code :
   `models.llm` avec `reasoning: "low"` pour le modèle par défaut, et un `max_output_tokens`
   déclaré. Aucune ligne de Rust, et un manifeste peut changer les deux.
2. **Une réponse coupée ne finit pas le pas.** Un tour qui finit sur `MaxTokens` sans appel d'outil
   reçoit une relance (« ta réponse a été coupée : appelle un outil, ou step_done ») et le pas
   continue, dans les bornes du pas (`max_model_calls_per_step`). Le texte de la relance est un
   texte déclaré, comme les autres textes vus par le modèle.
3. **Un pas coupé n'est pas un pas raté.** La règle des trois pas compte les pas qui n'ont rien fait
   (aucun outil qui laisse une trace, aucune lecture neuve), et non ceux qui ont été coupés en
   chemin. La borne reste dans la politique déclarée (`teammate.max_failed_steps` et, nouveau,
   `teammate.max_stalled_steps`).

Les témoins, rouges d'abord :
- un faux modèle qui finit deux fois sur `MaxTokens` puis appelle un outil : le pas continue et
  aboutit ;
- trois pas coupés qui lisent puis écrivent au quatrième : l'équipier ne s'arrête pas ;
- le gabarit livré déclare un modèle avec `reasoning` (le manifeste se lit, la valeur arrive au
  client).

Ensuite seulement, rejouer lucivy 03, puis les autres cas.

## Taille

Une demi-journée pour 1 et 2, une demi-journée pour 3, côté hôte.
