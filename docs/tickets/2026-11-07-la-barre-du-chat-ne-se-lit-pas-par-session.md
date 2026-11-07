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

## Ce qu'il faut pour le fermer

La barre s'inverse.

- **Premier niveau : la session**, c'est-à-dire une conversation ou un agent : son nom, son
  modèle, sa récap d'usage (la jauge) et ses fiches écrites.
- **Sous chaque session**, en hiérarchie, les **dépôts** qu'elle a. Chacun montre :
  - son worktree et sa branche ;
  - « chargé / non chargé » (le backend ouvert ou fermé) ;
  - « rag : indexé / en cours / absent » (l'index du dépôt et sa progression).
- **La mémoire de la session** paraît sous elle, avec son accès rag (oui ou non).
- **Un dépôt attaché à aucune session** paraît en bas, « disponible ».

Aujourd'hui une session n'a qu'un dépôt (le chat d'un backend) : la hiérarchie a un seul enfant,
mais la forme est déjà celle de la session multi-dépôts. Le JSON de `/ide/etat` (sessions →
dépôts) s'accorde avec everything declarative.

Témoin navigateur : une session, et sous elle son dépôt « rag3db · master · chargé · rag indexé »
et sa mémoire.
