# Un arrêt laisse le catalogue ouvert, et l'ouverture suivante rejoue seize minutes sans rien dire

- **État** : ouvert (21 novembre 2026, écrit par G à la demande de c7)
- **Pour** : everything declarative (l'arrêt et la réaction de la montre), cœur (le rejeu et le point de reprise)
- **Famille** : moteur (le rejeu du WAL), et produit (l'arrêt de l'hôte)
- **Gravité** : haute. Pendant seize minutes, la recherche du dépôt de la tablette a été indisponible pour toutes les
  sessions, et l'ouverture ne disait pas si elle avançait.
- **Atteignable en service** : oui, à tout arrêt de l'hôte pendant une réindexation de la montre
- **Touche rag3weaver** : oui (l'arrêt au Drop d'un backend, la réaction `reindex_on_change`, l'ouverture)

## Ce que c'est

Le 21 novembre 2026 à 01 h 55, la pose d'un hôte neuf à la tablette arrête l'ancien. Son journal
(`~/.local/state/rag3weaver/daemon.log`) dit, pour code-rag3db-depot :

```
01:55:18.187 [réaction] réaction figée (catalogue) : réacteur figé : pas arrêté en 30 s (4 exécution(s) faites) — sa tâche est lâchée sans être attendue
01:55:18.187 [backend] arrêt au Drop : 1 réacteur(s) figé(s) — catalogue laissé ouvert, rejoué à la prochaine ouverture : …
01:55:18.611 [démon] fermeture : …/code-rag3db/templates/backends/code/rag3db.json : 1 réacteur(s) figé(s) — catalogue laissé ouvert, …
```

Ce réacteur est la réaction de la montre (`reindex_on_change`) : à l'arrêt, elle en était à sa 4ᵉ exécution, une
réindexation qui écrivait dans le catalogue. Il reste à côté de la base un `.wal` de 15,7 Mo, daté de 01:54:55.

Le nouvel hôte ouvre le dépôt à 01:55. Jusqu'à **02:11:27**, `/call` répond 422 : « …/rag3db.json s'ouvre depuis N s
(son schéma et ses index se chargent) : cet appel n'a rien fait, recommencez dans un moment ». Pendant ces seize
minutes :
- un seul fil (`demon-3`) calcule à ~95 % d'un cœur et lit très peu (rchar d'environ 1 Mo en 5 s) ;
- la base et le `.wal` ne changent ni de taille ni de mtime, et l'hôte n'écrit presque rien (wchar +265 Ko en 60 s) ;
- le journal de l'hôte ne dit rien de l'ouverture ;
- la pile ne se lit pas (ptrace refusé aux utilisateurs).

À 02:11:27, la base et le `.wal` sont réécrits d'un coup (wchar +200 Mo), puis la recherche répond. C'était le rejeu du
WAL, puis le point de reprise et la mise à jour de l'index vectoriel (lecture d'ED). La lib était 4ad394f4d2, sans le
moteur HNSW neuf de hotfixs.

## Trois défauts

1. **L'arrêt laisse le catalogue ouvert, sans point de reprise**, parce qu'une réaction de la montre ne s'arrête pas en
   30 s (ED). Un arrêt devrait interrompre la réaction entre deux fichiers, ou attendre sa fin bornée, et faire son
   point de reprise.
2. **Rejouer 15,7 Mo de WAL prend ~16 min de CPU** avant de rendre la base (cœur et hotfixs). À reproduire hors
   service : avec la lib 4753938c9d (le HNSW neuf), et avec 4ad394f4d2.
3. **L'ouverture ne dit ni son étape ni sa progression.** « s'ouvre depuis N s » ne distingue pas un rejeu qui avance
   d'une boucle. Il faudrait l'étape (rejeu du WAL, point de reprise, index) et une mesure de progression, au journal
   et dans la réponse 422.

## D'ici les correctifs

G pose un hôte neuf seulement quand la montre est au repos (`watches.code` sans réaction en cours), pour ne pas couper
une réindexation.

## La preuve de la fermeture demandée (21 novembre 2026, 06 h 17 et 07 h 08)

- **06 h 17, arrêt brutal** (pose de l'hôte du matin à la tablette) : la montre, au repos depuis 06 h 04, s'était
  réveillée à 06 h 17 min 19 s. À l'arrêt, à 06 h 17 min 43 s, le réacteur s'est de nouveau figé (« pas arrêté en
  30 s (32 exécution(s) faites) — catalogue laissé ouvert »). Le rejeu à l'ouverture n'a duré qu'environ une minute :
  la réaction coupée avait peu écrit.
- **07 h 08, fermeture demandée avant l'arrêt** (pose du second hôte à la tablette) : le dépôt et la tâche atelier ont
  été fermés par `POST /backend/close` avant l'arrêt de l'hôte, la montre ayant été active à 07 h 01 min 58 s. Le
  dépôt s'est fermé en environ 10 s, la tâche en moins d'1 s. **Le point de reprise a été fait** : aucun `.wal` à côté
  de la base après la fermeture, et aucun « réacteur figé » ni « catalogue laissé ouvert » au journal, ni à la
  fermeture ni à l'arrêt qui a suivi. L'hôte neuf a rouvert le dépôt sans rejeu.

**D'ici le correctif**, toute pose d'un hôte ferme d'abord ses backends de code par `/backend/close`, puis arrête
l'hôte. C'est la règle tenue par G depuis le 21 novembre au matin.

## Un quatrième défaut : la fermeture d'une pose survit au redémarrage (07 h 08 - 07 h 2x)

Les deux fermetures de la pose de 07 h 08 (`/backend/close`, raison « pose du second hôte (G, 21 nov.) ») ont été
**relues par l'hôte neuf** : à 07 h 24, `daemon_status` montrait encore 12389e156742 (le dépôt de Lucie) et
2fb80390ed75 (la tâche atelier) en `held_closed`. Les recherches et écritures de Lucie, qui commençait son essai,
étaient rangées ou refusées jusqu'à ce que G les relève par `POST /backend/open`. Une fermeture dont l'auteur a disparu
(sa session fermée, l'hôte redémarré) ne devrait pas tenir au-delà de son délai sans le dire ; et un script de pose
doit relever ses propres fermetures après le réveil (fait dans celui de G).
