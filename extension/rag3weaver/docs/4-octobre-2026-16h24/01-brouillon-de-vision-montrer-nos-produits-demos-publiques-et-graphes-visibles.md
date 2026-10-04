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
