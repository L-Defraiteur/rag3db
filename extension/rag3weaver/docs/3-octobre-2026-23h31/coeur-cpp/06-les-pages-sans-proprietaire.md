# Les pages sans propriétaire après une réouverture — une page avant le code

5 octobre 2026, session cœur C++. Ticket :
`docs/tickets/2026-10-05-pages-d-un-copy-journalise-perdues-au-rejeu.md` (bloque le basculement
du `COPY` journalisé). **[lu]** : vu dans le code à `71cffbc4b`, par une cartographie d'agent
non rejouée. **[déduit]** : tiré de cette lecture. **[mesuré]** : exécuté.

## 1. Le défaut, et il est plus large que le COPY journalisé

**[mesuré]** Trois cas de la suite Cypher, défaut basculé : après un `COPY` journalisé et une
réouverture sans point de reprise, 403 pages occupées pour 11 attendues, 949 pour 11, 232
pour 5 (`fsm_leak_checker.cpp:116`).

Pourquoi **[lu]** :

- Un `COPY` écrit ses pages directement dans le fichier de données pendant qu'il s'exécute,
  sans page fantôme (`Column::flushData`, `writePagesToFile`). Il les prend à l'espace libre
  connu, ou à la fin du fichier.
- Le rejeu ne touche pas au fichier : il relit le dernier point de reprise, puis réinsère en
  mémoire. Le point de reprise suivant écrit ailleurs.
- Rien de persistant ne dit où le fichier « finissait » au dernier point de reprise. L'en-tête
  de la base ne porte que les plages du catalogue et des métadonnées ; l'espace libre est une
  liste de plages ; le nombre de pages en mémoire est **la taille du fichier** à l'ouverture
  (`FileHandle::constructPersistentFileHandle`). Les pages ajoutées en fin de fichier par un
  travail qui n'a pas atteint de point de reprise sont donc comptées, et à personne.

Les pages prises à l'espace libre ne fuient pas : la liste persistée les dit encore libres.
Seule la fin du fichier fuit.

Ce qui tombe dans le même état **[déduit, à mesurer]** :

- le `COPY` forcé d'aujourd'hui, tué pendant son exécution ou avant la fin de son point de
  reprise — sans aucun journal à rejouer : **le défaut actuel a déjà cette fuite sur mort** ;
- un `COPY` replié (`71cffbc4b`) tué avant sa validation ;
- un point de reprise interrompu avant sa marque : les pages qu'il a ajoutées en fin de fichier ;
- une queue de fichier libérée par un point de reprise : `handleLastPageRange` réduit le compte
  en mémoire mais ne tronque pas le fichier (`removePageIdxAndTruncateIfNecessary`, malgré son
  nom) ; à la réouverture elle est recomptée et n'est plus dans l'espace libre.

Ce qui fuit par un autre chemin, **hors de ce lot** (pages prises sans l'allocateur
« optimiste », jamais rendues à l'annulation) **[lu dans les commentaires]** : la création
d'un index FTS annulée, un `ALTER … ADD` annulé sur des groupes déjà sur disque, un point de
reprise annulé (`FreeSpaceManager::rollbackCheckpoint`). À ticketer à part.

## 1 bis. Mesuré le 10 octobre, avant de coder

Les deux formes « déduites » sont mesurées (programme nu, un `COPY` de 200 000 lignes, mort
base ouverte, réouverture, tables retirées, point de reprise, pages occupées ; 9 pour la scène
validée avec point de reprise) : **`COPY` forcé d'aujourd'hui tué avant sa validation, 559
pages ; `COPY` replié tué, 560** ; journalisé validé puis tué, 559 ; deux morts de suite,
1 269 — la fuite s'additionne. Le défaut est donc **celui du moteur d'aujourd'hui**, le `COPY`
journalisé n'y ajoute que la réouverture par le rejeu ; reclassé par l'orchestration hors de la
condition 1 de la stèle (de l'espace, pas une perte), corrigé maintenant.

**Le seuil du groupe plein** : un `COPY` n'écrit ses pages avant la validation que par groupe
plein de nœuds (131 072 lignes) ; en deçà rien ne fuit (3 000 lignes : 4 pages occupées). Les
témoins écrits avant la pause à 60 000 lignes étaient verts sur le moteur d'aujourd'hui — un
faux rouge évité par la mesure ; ils sont à 200 000.

## 2. Ce que font les moteurs établis

PostgreSQL ne rend rien au rejeu : les pages d'une transaction morte restent dans le fichier
de la table, mortes, et c'est le ménage (`VACUUM`) qui les rend à l'espace libre de la table ;
il ne tronque que la queue vide, et sous verrou. Le rejeu reste simple, la récupération est
différée et sûre. Nous n'avons pas de ménage ; l'équivalent le plus proche est le point de
reprise, qui persiste l'espace libre.

## 3. La proposition

**Persister l'étendue du fichier au point de reprise, et rendre à l'espace libre, à
l'ouverture en écriture, tout ce que le fichier porte au-delà.**

1. **Au point de reprise** : écrire dans l'en-tête de la base le nombre de pages du fichier à
   cet instant (`Checkpointer::writeDatabaseHeader`). Toutes les allocations du point de
   reprise ont déjà eu lieu à ce moment **[lu]** : c'est une borne haute de ce qu'il référence.
   L'en-tête passe par une page fantôme : il est atomique avec le point de reprise.
2. **À l'ouverture en écriture**, après l'application éventuelle des pages fantômes et la
   lecture de l'espace libre (`Checkpointer::readCheckpoint`), avant tout rejeu : si le fichier
   compte plus de pages que l'étendue persistée, la plage `[étendue, pages du fichier)` est
   sans propriétaire **par construction** — ni l'état persisté ni le journal, qui est logique,
   ne désignent une page physique. Elle est ajoutée à l'espace libre en mémoire, qui est marqué
   à persister.
3. **Pas de troncature.** Comme PostgreSQL : rendre, ne pas couper. Le rejeu et le point de
   reprise suivant réutilisent ces pages ; le fichier ne grossit pas d'une mort à l'autre. Couper
   demanderait de raisonner sur un lecteur en lecture seule qui tient le fichier, pour un gain
   (rendre des octets au système de fichiers) que personne ne demande aujourd'hui.

Propriétés :

- **Idempotent** : mort avant le point de reprise suivant, l'excédent est retrouvé à
  l'identique à l'ouverture d'après.
- **En lecture seule** : rien n'est fait ; un lecteur n'alloue pas, l'excédent ne le gêne pas.
- **Une base d'avant** (en-tête sans étendue) : étendue inconnue, rien n'est rendu ; elle
  l'acquiert à son premier point de reprise. À vérifier au code : que la fin de la page d'en-tête
  est lue comme « inconnue » et non comme zéro pris pour une étendue ; sinon, numéro de version.
- **Couvre d'un coup** : le `COPY` journalisé rejoué, le `COPY` forcé tué, le `COPY` replié tué,
  le point de reprise interrompu, la queue non tronquée.

Risques à lever par des témoins, pas par lecture :

- l'ordre avec les pages fantômes : rendre **après** leur application (un point de reprise mort
  après sa marque installe un en-tête neuf, dont l'étendue fait foi) ;
- une page allouée mais jamais écrite (le fichier plus court que l'étendue) : ne rien faire ;
- la plage rendue est réutilisable **tout de suite**, pendant le rejeu : c'est voulu, et sûr
  seulement si rien ne lit ces pages — le témoin du point de reprise interrompu le dira.

## 3 bis. Deux limites (relecture du banc, 10 octobre)

- **Les pages perdues avant la version 40 ne reviennent jamais** : le premier point de reprise
  en 40 écrit une étendue qui les englobe. Une base qui a subi des morts sous l'ancien moteur
  garde ses pages orphelines ; seule une réindexation les rend.
- **Une ouverture en écriture qui rend des pages a « quelque chose à persister »** (le
  gestionnaire d'espace libre marque sa version) : un point de reprise de fermeture, s'il y en
  a un, écrira, sans qu'aucune requête ait écrit. C'est voulu.

## 4. Les témoins (rouges d'abord)

Mesure commune : pages occupées = pages du fichier − pages libres, après avoir retiré toutes
les tables et fait un point de reprise (le contrôle de `fsm_leak_checker`), comparées à celles
d'une base témoin qui a fait le même travail sans mourir.

1. `COPY` journalisé validé, mort base ouverte (`.wal` non vide exigé), réouverture, rejeu.
2. `COPY` forcé (le défaut d'aujourd'hui) tué avant sa validation : le chiffre qui dit si le
   défaut actuel fuit.
3. `COPY` replié tué avant sa validation.
4. Point de reprise interrompu avant sa marque, et après elle (les points de mort du banc).
5. Deux morts de suite sans point de reprise entre elles : la fuite ne s'additionne pas.
6. La même base ouverte en lecture seule entre-temps : rien n'est écrit, le contenu est juste.
7. Une base écrite par le moteur d'avant : elle s'ouvre, rien n'est rendu à tort, son contenu
   est intact après un `COPY` et un point de reprise.
8. Les trois cas Cypher du ticket, verts sous le défaut basculé.

Relecture du banc avant le push : c'est la reprise.
