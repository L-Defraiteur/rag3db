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
