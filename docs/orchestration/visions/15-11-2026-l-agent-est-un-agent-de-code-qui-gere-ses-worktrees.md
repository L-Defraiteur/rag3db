# L'agent est un agent de code, qui gère ses worktrees et sait quand son index est prêt

Lucie, 15 novembre 2026, 16 h 01, devant son terminal (mots gardés) :

> « en gros pour moi l'agent est un agent de code, sauf qu'il gère builtin des worktrees etc., et ça les
> indexe automatiquement, genre là je lui dis "fais telle feature" hop il s'en occupe, il prend tel
> worktree, il peut faire des recherches en phase avec ce worktree sur lequel il fait un truc, etc. etc.
> Il sait quand l'index de recherche est prêt pour s'en servir, malgré qu'il ait grep etc. »

Et juste avant : « j'aimerais vraiment retrouver l'expérience d'un agent de code standard tout en gardant
toutes ces fonctionnalités, là ça parle un peu trop de gabarits » ; « il faut toujours une réponse à
l'utilisateur » ; « les étapes, je les vois défiler à côté de "ça travaille", pas dans l'historique ».

## Ce que ça fixe

1. **Un agent de code d'abord.** Lire, éditer, écrire, lancer, chercher dans les fichiers : le socle
   d'un agent standard, toujours présent dans une tâche, quel que soit le manifeste du backend. Le
   déclaratif (gabarits, déclarations) est une capacité de plus, en bas de liste, pas la langue de l'agent.
2. **Les worktrees sont à lui.** « Fais telle feature » : il ouvre ou prend une tâche avec son worktree
   et sa branche, sans qu'on le lui dise ; une tâche est une couche (13 novembre).
3. **Les recherches sont en phase avec le worktree** où il travaille : la vue par couche (2b′, 2b-C) est
   ce qui rend ça vrai ; d'ici là, le worktree du backend et le repli sur les fichiers.
4. **Il sait quand l'index est prêt.** L'état d'index et l'attente nommée (wait_output) lui disent ce
   qu'il peut demander à l'index ; avant, grep et la lecture font le travail, et il le dit.
5. **Il parle à l'utilisateur**, une réponse par tour ; les étapes se voient dans la zone vivante, pas
   dans l'historique.

## Ce qui le porte, en file

- Le socle (archi : le pas, l'invite, la porte ; ED : les primitives) après le lot (B) de l'équipier muet.
- Les vues par couche (arbre 2b′, recherche + arbre 2b-C, ED l'écrivain et 2a′-3).
- Le terminal (tty : étapes dans la zone vivante, /model, o/n sur une proposition).

## Ajout de 16 h 10 : la porte

Lucie : « il faut le mode auto par défaut, et presque jamais me demander si c'est déjà dans ce que j'ai dit ;
il faut qu'un modèle décide, et soit pas trop strict ».

6. **Auto par défaut, et un modèle qui décide.** Le bac à sable (Landlock) est la garde dure ; dedans, une
   action que la demande de l'utilisateur couvre passe sans question, écriture ou bâti compris ; seule une
   action hors de la demande, hors du bac ou vers le réseau non déclaré demande. Le décideur est un
   modèle qui lit la demande, le plan et l'action, et penche vers « passe » dans le doute ; le journal
   des passes et des refus le règle. La métrique : le taux de demandes inutiles, pas le taux de refus.

## Ajout de 21 h 0x : l'index prêt, sans bruit

Lucie : « faut peut-être pas non plus le re-notifier à chaque petit changement qu'il cause lui-même ; quand il
fait une recherche, on estime si le temps d'attente est supérieur à 6 secondes d'indexation ; si oui il sera
notifié et il reçoit que les index ne sont pas prêts ; si non son retour est immédiat et on attend le temps
qu'il faut en synchrone ».

7. **Une recherche attend l'index si c'est court, sinon elle le dit une fois.** L'agent n'est pas notifié de
   chaque réindexation que ses propres écritures provoquent. À la recherche, l'hôte estime le reste à indexer
   pour ce worktree : sous le seuil (6 s par défaut, réglable), la recherche attend en synchrone et répond
   complète, sans un mot ; au-dessus, elle répond tout de suite « index pas prêt » avec l'estimation, et une
   seule notification vient quand l'index est prêt. L'estimation se mesure sur le journal d'index (lignes
   restantes, débit observé), pas sur une constante.
