# L'indexation portée par le démon — page courte, avant le code

*Arbre principal, 11 octobre 2026, nuit. Pour Lucie, au matin. Réponse au ticket
`docs/tickets/2026-10-11-indexation-enfant-du-serveur-mcp.md` (« l'indexation est un enfant
du serveur MCP et meurt avec lui »). Rien n'est codé : il y a trois formes à choisir, leur coût,
et ce qui peut partir tout de suite.*

## Le constat, et deux faits qui changent la réponse

Aujourd'hui, `index` lance un **fil** du processus serveur (`spawn_index`,
`dataflow/index_nodes.rs`), et rend aussitôt « indexation lancée en fond » avec un journal.
Quand le client ferme le tube, le serveur sort (`servir_mcp`, `bin/rag3weaver-backend.rs`) et
le fil meurt avec lui, sans un mot. Mesuré par la session mémoire sur rag3db : tube fermé après
`index`, 79 s, base vide.

Deux faits, lus dans le code cette nuit, changent ce que le ticket propose :

1. **`--demon` ne sauve pas l'indexation.** Le démon `rag3daemon` ne tient que la base : il reçoit
   du Cypher et rien d'autre. Le catalogue, la synchronisation, les vecteurs et les fichiers du
   plein texte restent dans le serveur MCP, même avec `--demon`. La phrase que le ticket propose
   pour la réponse d'`index` (« lancez un démon pour qu'elle lui survive ») serait donc fausse
   aujourd'hui.
2. **La commande en fond a le même défaut.** Son groupe de processus la protège des signaux,
   mais ses sorties passent par des tubes que lisent deux fils du serveur. Quand le serveur sort,
   le journal cesse d'être écrit et la commande reçoit SIGPIPE à sa prochaine écriture. Sa
   poignée vit dans une table du processus : elle est perdue avec lui. Ce « fond » vit dans
   l'hôte.

Et une limite connue : **relancer `index` après un serveur tué est refusé.** La session de
synchronisation laissée ouverte exige `takeover`, et `index` ne le passe pas (ticket
`2026-10-04-reprise-apres-arret-exige-takeover.md`, ouvert).

## Ce qui peut partir tout de suite, sans choix

- **Dire la vérité dans la réponse d'`index`** : « elle s'arrêtera avec ce serveur ; gardez cette
  session ouverte jusqu'à « indexation terminée » ». Sans mention de `--demon` tant qu'il ne la
  porte pas. Une ligne dans `IndexNode`, avec son témoin. Coût : une heure.
- **Écrire les journaux des commandes en fond directement dans leurs fichiers**, comme le fait déjà
  le lancement des démons (`Serveur::lancer`), et non plus par des tubes. La commande survit alors
  à son hôte, et son journal continue d'avancer. Ce qu'on perd : l'anneau de 64 Mio, qui lit le
  tube. Il faudrait le remplacer par un plafond appliqué au fichier par un surveillant. Coût :
  une demi-journée.

## Les trois formes pour la suite

### A. Arrêt propre entre deux paquets, dans le serveur (le plus petit)

Le serveur ne meurt plus en plein travail. À la fermeture du tube, il demande au fil d'index de
s'arrêter au prochain paquet validé, l'attend, ferme la base et sort. La session suivante reprend
là où il s'est arrêté.

- **Demande** : un drapeau d'arrêt lu entre deux paquets par `sync_source` et la boucle des
  vecteurs ; que la reprise ne refuse plus une session laissée par un processus qui a fini
  proprement (la marquer close à l'arrêt) ; `index` relancé reprend.
- **Ne règle pas** : l'indexation reste liée à une session ouverte, et un serveur tué
  (SIGKILL, plantage) laisse toujours la session ouverte, donc le refus de reprise.
- **Coût** : un à deux jours, témoins compris : tube fermé au paquet 2, puis reprise aux mêmes
  comptes qu'une passe sans arrêt, sur le modèle d'`e2e_tx_par_paquet_arret`.

### B. Un processus d'indexation détaché (intermédiaire)

`index` lance un processus `rag3weaver-backend index <manifeste>`, détaché (nouveau groupe,
journal écrit directement en fichier, comme les démons), et rend sa poignée (pid) et son journal.

- **L'obstacle** : un seul écrivain par base. Le serveur MCP tient la base ouverte en écriture,
  donc le processus détaché ne peut pas l'ouvrir à côté de lui. Il faudrait que le serveur la lui
  cède le temps de l'index : le serveur passe en lecture seule, et rouvre à la fin. Ce n'est
  possible que si toutes ses lectures savent attendre, ce que le lecteur affamé vient justement
  de régler côté moteur.
- **Coût** : trois à quatre jours. La cession de la base est le point dur, et elle demande une
  page à elle.

### C. Le démon possède le catalogue (la forme du ticket)

« Installer rag3weaver, c'est installer un démon. » Le démon n'est plus seulement la base : il
porte le catalogue entier, comme `rag3weaver-backend serve` le fait déjà (il possède la base et a
des routes). Les serveurs MCP deviennent des clients minces de ce démon, qui vont et viennent.
`index` devient un travail du démon : poignée, journal, reprise entre deux paquets. Sans démon, la
réponse le dit.

- **Demande** : une route de travaux dans le démon (lancer, suivre, arrêter), le serveur MCP
  qui passe par lui pour tous ses outils, et l'installation du démon par le paquet npm. Le paquet
  n'installe aucun démon aujourd'hui (`03-le-paquet-npm.md` §5 bis le prévoit, rien n'est codé).
  C'est la forme qui règle aussi le mode `--demon`, non validé (ticket
  `2026-10-11-mode-demon-ni-valide-ni-refute.md`).
- **Coût** : une à deux semaines, réparties entre l'optimiseur (le paquet), la session mémoire
  (le serveur MCP) et l'arbre principal (les travaux du démon).

## Ce que je recommande

1. **Tout de suite** : la ligne de vérité dans `index` et les journaux de la commande en fond
   écrits directement en fichiers. Ces deux pièces servent toutes les formes.
2. **Ensuite, A** : elle règle le cas mesuré (tube fermé, travail perdu) en un à deux jours, et sa
   reprise entre deux paquets servira telle quelle à C.
3. **C comme cible**, décidée avec le paquet npm : c'est la seule forme où un travail long survit
   à tout ce qui l'a lancé. **B ne vaut pas son coût** : la cession de la base est aussi dure que
   C, pour moins.

Ce qui reste à Lucie : choisir entre A puis C, ou C directement, et dire si le démon du paquet
npm est la cible de cette semaine.
