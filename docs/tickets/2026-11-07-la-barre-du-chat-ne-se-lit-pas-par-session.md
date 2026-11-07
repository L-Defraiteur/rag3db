# La barre du chat ne se lit pas par session : les dépôts devraient être sous l'agent

- **État** : ouvert (7 novembre 2026), retour de Lucie sur la pose de 21 h 51 ; pour hooks-essai,
  dans le lot de la barre, avant la coloration B ; la forme du JSON à accorder avec everything
  declarative (page 02, sa direction).
- **Famille** : produit
- **Gravité** : gêne (la barre se comprend mal : on navigue par dépôt alors qu'on travaille par
  agent).
- **Atteignable en service** : oui, la barre du chat de l'IDE.
- **Touche rag3weaver** : oui (`/ide/etat`, `ui/ide/src/views/Chat.tsx`).

## Ce que c'est

Lucie : « on comprend mal, du coup, vu qu'on navigue par dépôt, ça va toujours pas ; je crois
qu'on doit quand même naviguer par "agent" ou "session", mais que dans la barre on doit voir en
hiérarchie de lui les dépôts qui sont chargés ou pas pour lui, et en accès rag ou pas ».
Depuis 3d09b32c2, la barre range ses conversations sous « Dépôts », « Mémoire » et « Projets ».

## Ce qu'il faut pour le fermer (forme tranchée avec Lucie, c7, 7 novembre 2026 au soir)

- **Le premier niveau est la session**, c'est-à-dire l'agent : son nom, son modèle, son usage et ses
  fiches. Sous chaque session, deux groupes repliables : **Dépôts** (ce sur quoi elle travaille)
  et **Sous-sessions** (à qui elle délègue).
- **Un dépôt s'étend vers ses worktrees**, chacun avec sa branche, « chargé / non chargé » et
  « rag : indexé / en cours / absent ». Le worktree est la feuille : c'est lui qu'on ouvre, et
  c'est lui qui porte la conversation de tâche.
- **Une sous-session est une session à part entière**, avec le même motif sous elle et son état :
  en cours, en attente d'une réponse, finie. Quand elle travaille dans un worktree du dépôt de la
  mère, ce worktree porte « tenu par <sous-session> » sous Dépôts, et la sous-session montre le
  même worktree sous elle : une seule donnée, deux regards.
- **Tout est replié sauf le chemin actif.** Le chat porte en tête un fil d'Ariane :
  session › dépôt › worktree, ou session › sous-session › worktree.
- **Autour de l'arbre** :
  - un filtre « actifs seulement » ;
  - les dépôts attachés à personne, en bas, sous « disponibles » ;
  - la mémoire, comme enfant de la session, avec son accès rag.
- **Un menu déroulant en tête de la barre change de vue** (Lucie : « un dropdown pour la vue par
  dépôt plutôt que par agent, pour switcher »). La vue « par session », par défaut, est la forme
  ci-dessus. La vue « par dépôt » est l'arbre inversé : dépôt › worktree › les sessions qui le
  tiennent. Les données sont les mêmes, et le choix est mémorisé (localStorage).
- **Le JSON de `/ide/etat`** (`chat_sessions`, `available_repos`, les sous-sessions par parent)
  s'accorde avec everything declarative.

Témoins navigateur :
- l'arbre par session, avec un dépôt tenu par une sous-session (le badge des deux côtés) ;
- le menu qui inverse ;
- le filtre ;
- le fil d'Ariane.
