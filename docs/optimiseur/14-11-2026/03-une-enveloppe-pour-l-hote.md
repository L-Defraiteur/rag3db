# Une enveloppe pour l'hôte (proposition, 14 novembre 2026)

Pour Lucie, qui décide : les unités systemd restent à sa main. Constat de lucivy, relu par G sur l'hôte du
portable ; la page propose un drop-in, dit ce qu'il protège et ce qu'il coûte, et explique les 168 fils `demon-#`.

## Le constat (portable, 14 nov., hôte MainPID 2943666, 4 h de vie)

- `rag3weaver-daemon.service` n'a **aucune enveloppe** : `cpu.max = max`, `CPUQuota` infini, `MemoryMax`
  infini, `Nice` 0. Pendant une indexation, il peut prendre les 32 cœurs du portable, à égalité avec l'écran.
  Lucie a vu l'écran geler pendant des indexations.
- **359 fils au repos** :

  | famille | fils | d'où |
  |---|---|---|
  | `demon-#` | 168 | voir plus bas : 8 fils de l'hôte, et 160 du moteur |
  | `rag3weaver-graphes` | 34 | les graphes de l'hôte |
  | `teammate-journa…` | 33 | le journal de l'équipier |
  | `scheduler-#` | 32 | lucivy : `LUCIVY_SCHEDULER_THREADS`, sinon `available_parallelism` |
  | le reste | ~92 | relais de l'IDE, journaux, montres |

- **Le temps CPU se concentre** dans deux groupes `demon-#`, ceux des pools du moteur : 201 820 ticks pour
  « demon-2 » (la base de code, qui indexe), 74 069 pour « demon-0 ». Viennent ensuite `montre-code` (68 599) et
  les `scheduler-#` de lucivy (environ 3 400 chacun).

## Pourquoi 168 `demon-#`

Le serveur de l'hôte a **8 fils** (`--threads`, 8 par défaut, `src/bin/rag3weaver-backend.rs:447`), nommés
`demon-0` à `demon-7` (`src/daemon_host.rs:1696`), plus `demon-releve`. Les 160 autres **ne sont pas des fils
de l'hôte** : sous Linux, un fil sans nom hérite du nom de celui qui l'a créé. Chaque base ouverte par une
requête crée le **pool de fils du moteur**. `rag3db::SystemConfig` part avec `max_num_threads = 0`, et le moteur
prend alors tous les cœurs : 32 fils par base, au nom du fil `demon-#` qui a fait l'ouverture.

Relevé par numéro :

| nom | fils | lecture |
|---|---|---|
| `demon-0` | 65 | 1 + 2 bases × 32 |
| `demon-2` | 33 | 1 + la base de code |
| `demon-4` | 33 | 1 + une base |
| `demon-5` | 33 | 1 + une base |
| `demon-1`, `-3`, `-6`, `-7` | 1 chacun | les fils du serveur seuls |

Cela fait cinq bases ouvertes (code-rag3db-depot, memoire-longue, essais-c7, essai-atelier, l'atelier), **5 × 32
= 160**. L'hôte ne passe jamais `max_num_threads` au moteur : seul un test le fait
(`src/rag3db_connection.rs:1160`). `RAG3WEAVER_EXEC_THREADS` borne les fils **d'une exécution**, par connexion,
mais pas la taille du pool, créé à l'ouverture de la base.

## Le drop-in proposé

À poser par Lucie dans `~/.config/systemd/user/rag3weaver-daemon.service.d/enveloppe.conf`, au portable (32 cœurs,
`coeurs-libres` 2), puis `systemctl --user daemon-reload` et le redémarrage de l'hôte à l'heure qu'elle choisit :

```
[Service]
# Les cœurs utiles, comme pour un lourd : (32 − 2) × 100 %.
CPUQuota=3000%
# L'écran et les sessions d'abord, quand tout le monde veut du CPU (100 par défaut).
CPUWeight=20
Nice=5
IOSchedulingClass=best-effort
IOSchedulingPriority=6
# Les bornes lues au démarrage : lucivy et les exécutions du moteur.
Environment=LUCIVY_SCHEDULER_THREADS=8
Environment=LUCIVY_WRITER_THREADS=4
Environment=RAG3WEAVER_EXEC_THREADS=8
```

À luciepc (24 cœurs, l'hôte rarement à l'écran), la même forme avec `CPUQuota=2200%`.

## Ce qu'il protège, et ce qu'il ne règle pas

- **`CPUWeight=20` et `Nice=5` font le gros du travail pour l'écran.** Quand le bureau, VSCodium et l'hôte se
  disputent les cœurs, le noyau donne environ cinq fois plus de temps au bureau. Le quota, lui, ne réserve que
  deux cœurs.
- **Les sessions qui parlent à l'hôte** (MCP, crochets, IDE) passent par ses 8 fils de serveur. Le quota ne les
  affame pas tant que l'indexation laisse du temps, et `LUCIVY_SCHEDULER_THREADS=8` limite à 8 les fils de
  lucivy qui travaillent en même temps.
- **Ce qu'il ne règle pas : les 160 fils du moteur.** La taille du pool se fixe à l'ouverture de la base, et
  aucune variable d'environnement de l'hôte ne la règle aujourd'hui. Il faut **un lot Rust** : lire
  `max_num_threads` dans une variable (par exemple `RAG3WEAVER_DB_THREADS`), ou le dériver du quota, et le
  passer à `SystemConfig` à l'ouverture. Ces fils au repos coûtent surtout de la pile, pas du CPU : c'est une
  borne de propreté, pas la cause du gel.
- **Rust lit le quota de lui-même** : `available_parallelism` suit `cpu.max` (cgroup v2). Sous le quota, lucivy
  prendrait 30 fils de planificateur et non plus 32, même sans la variable. Le C++ du moteur, lui, prend le
  nombre de cœurs de la machine.

## Ce qu'il coûte

- **Une indexation complète plus lente quand l'écran travaille**, et seulement à ce moment. Écran au repos, le
  poids ne joue pas, et le quota laisse 30 cœurs sur 32. Je ne l'ai **pas chiffré** : la carte et luciepc étaient
  pris. La mesure à faire pour chiffrer : un index complet d'une copie de code-lourd, sur l'hôte d'essai 7991,
  avec et sans ces réglages, le poste au repos puis sous une charge d'écran simulée. Comme ordre de grandeur,
  lucivy bridé à 8 fils de planificateur au lieu de 32 ralentit d'abord le plein texte. Les vecteurs dépendent
  du service d'embarquement à luciepc, et pas du CPU du portable.
- `LUCIVY_WRITER_THREADS=4` réduit aussi la mémoire de l'écrivain (25 Mo par fil) et le nombre de segments créés
  en parallèle.
- **Rien n'est réversible sans redémarrage de l'hôte** : retirer le fichier, `daemon-reload`, puis redémarrer.

## À côté

`~/.config/systemd/user/rag3weaver-daemon.service.d/` contient au portable plus de soixante anciennes copies
`check-lib.conf.avant-*`, laissées par mes poses (`bascule-pose.sh`). systemd ne les lit pas, puisqu'elles ne
finissent pas par `.conf`, mais elles encombrent. Elles peuvent partir, au mot de Lucie.

## MALLOC_ARENA_MAX=2 : mesuré sur l'hôte réel, non retenu (14 novembre, 22 h 30)

**Le chiffre d'ED** (ticket `docs/everything-declarative/tickets/2026-11-14-une-recherche-a-versions-garde-20-gio-de-memoire-residente.md`) :
50 recherches sans filtre sur une base à versions (la copie migrée de 1b), embarqueur connecté, dans un processus de
test. La mémoire résidente passe de **20,6 Gio à 9,8 Gio**, en palier plat du 11e au 50e appel ; la médiane passe
de 2 832 ms à 2 515 ms. Pour le cœur, la pente venait des arènes de glibc : chaque fil de tokio prend la sienne, et la
mémoire rendue reste dans l'arène du fil d'avant. Le pic d'une analyse (3,6 Gio pour un Cypher de 2,9 Mo) est rendu ;
c'est sa rétention par arène qui s'empilait.

**La place** : le fichier d'environnement que l'unité lit déjà (`EnvironmentFile=-…/rag3weaver-daemon.env`). Aucune
unité n'est touchée, et la ligne suit les poses : `bascule-pose.sh` ne réécrit que `RAG3DB_LIBRARY_DIR` (par `sed`).
Le drop-in de la section précédente n'est pas nécessaire pour elle.

**Avant de la poser**, une mesure sur l'hôte réel (mot de c7) : un second hôte sur un port à part (jamais 7878), sur
une copie reflink de la même base à versions, avec puis sans la ligne, de vraies requêtes `search_code` par HTTP,
le VmRSS relevé à chaque appel. La sonde d'ED mesure un processus de test qui cherche ; cette mesure-là dit l'hôte.

**La mesure, faite le 14 novembre de 21 h 45 à 22 h 27** à luciepc, sous `poste mesure` :
- le montage : binaire de master 10b7a0edb (il porte 1b), sous 373b5477b, non posé ; un hôte à part sur 7996, avec son
  propre dossier de démon ; une copie reflink de `~/.cache/rag3weaver-build/1b-copie/migree-20261114-1641`, avec
  `.extensions` réécrit sur la libvector de 373b5477b ; le manifeste d'ED sans `watches` ;
- le relevé : 50 `search_code` par HTTP (10 requêtes en boucle), le VmRSS de l'hôte après chaque appel ;
- la mesure : deux paires, dans les deux ordres, pour séparer le réglage de la charge du poste (load 24, puis 13 à 6).
  Script : `~/.cache/optimiseur-mesure/arena/mesure.sh` à luciepc.

| série | ordre | RSS 1er → 10e → 50e appel | pente 11e → 50e | médiane 2–50 |
|---|---|---|---|---|
| sans | 1re | 5 034 → 5 678 → 6 005 Mio | +170 Mio | 460 ms |
| avec | 2e | 4 682 → 4 888 → 5 083 Mio | +38 Mio | 793 ms |
| avec | 1re | 4 795 → 4 921 → 5 028 Mio | +60 Mio | 771 ms |
| sans | 2e | 5 048 → 5 614 → 6 044 Mio | +301 Mio | 436 ms |

Le réglage gagne un peu moins d'1 Gio (−16 %) et aplatit la pente. Mais **il ralentit les appels de 70 à 77 %**, dans
les deux ordres : la lenteur vient du réglage, pas de la charge du poste. L'écart avec les 20,6 → 9,8 Gio d'ED est
attendu, d'après c7 : sa sonde projetait un Cypher de 2,9 Mo (la rétention de la projection), et l'hôte de master
passe désormais par le court-circuit sans masque, qui ne projette plus ainsi.

**Conclusion : la ligne n'entre pas dans l'environnement de l'hôte.** Elle reste une note, « sous un hôte qui
projette, à mesurer ». La commande ci-dessous reste écrite pour ce jour-là, et ne se pose pas sans une nouvelle mesure.

**La commande, si une mesure la justifie un jour**, aux deux postes, au mot de Lucie (redémarre l'hôte : les bases se rouvrent au premier
appel) :

```sh
grep -q '^MALLOC_ARENA_MAX=' ~/.config/rag3weaver/rag3weaver-daemon.env \
  || echo 'MALLOC_ARENA_MAX=2' >> ~/.config/rag3weaver/rag3weaver-daemon.env
systemctl --user restart rag3weaver-daemon.service
cat /proc/$(systemctl --user show rag3weaver-daemon -p MainPID --value)/environ | tr '\0' '\n' | grep MALLOC_ARENA_MAX
```

Pour revenir : retirer la ligne, puis `systemctl --user restart rag3weaver-daemon.service`.

Les services d'embarquement de luciepc (`rag3weaver-embeddings@bge-m3`, `@granite-278m`) sont d'autres unités ;
aucune mesure ne dit qu'ils ont la même pente. Ils restent hors de la proposition.

**Aujourd'hui, aucune base de Lucie n'est à versions** : rien ne presse avant que 1b ne migre une base servie.
