# Le contrôle du worktree compte les lots d'avant atterris comme du retard

- **État** : ouvert (14 novembre 2026, 23 h 50), relevé de codeparsers par c7 ; corrigé dans `docs/optimiseur/scripts/controle-du-worktree.sh` et recopié au cache des deux postes à 23 h 46 (sha256 110283489f652d69 → 94d5c09b6c85b737, l'ancien en `.avant-cherry-pick`). Reste ouvert pour l'inventaire ci-dessous.
- **Famille** : produit
- **Gravité** : faux rouge ; une passe sur une tête de pile s'arrête avant de commencer.
- **Atteignable en service** : non, c'est l'outillage des passes.
- **Touche rag3weaver** : non ; `~/.cache/rag3weaver-build/controle-du-worktree.sh`, une copie de cache sans dépôt (md5 dc22b61d2568 aux deux postes).
- **Pour** : optimiseur (G).
- **Ouvert le** : 14 novembre 2026

## Ce qui s'est passé

Le contrôle de début de passe compte le retard par `git rev-list --count HEAD..origin/master`. Sur une tête de pile qui
n'est pas encore sur master (dual-du-creux-pile 870bb4a5e), les lots d'avant déjà atterris sous d'autres hashes comptent
comme manquants : 275, au-dessus du plafond de 200, et la passe s'arrête avant de commencer. Le vrai retard, compté par
contenu, est de 133. c7 a autorisé `RETARD_MAX=300` pour cette passe, avec le compte réel dit dans le journal.

## La correction

Compter par contenu, toujours : `git log --cherry-pick --right-only --no-merges --format=%h HEAD...origin/master | wc -l`.
Hors pile, c'est le même nombre ; en pile, les lots d'avant atterris ne comptent plus.

Témoin `docs/optimiseur/scripts/test_controle_du_worktree.sh` (un dépôt jouet : P1 atterri sous un autre hash, trois
autres lots sur master) : l'ancien script compte **4** et s'arrête sous un plafond de 3 (deux ROUGE) ; le neuf compte
**3**, et s'arrête au-dessus du plafond avec le compte vrai (deux vert).

## Les scripts du cache que les passes appellent hors de tout dépôt (inventaire du 14 novembre, 23 h 50)

Remarque de c7 : un script que toutes les passes appellent et qui n'est dans aucun dépôt est un trou, le même que les
copies de `check-lib.conf` de l'après-midi. Relevé par le blob de chaque fichier de `~/.cache/rag3weaver-build/`
comparé à ceux de master, puis par les scripts du cache qui le nomment. Les familles `I-*`, `cx-*` et `wt-*` sont des
scripts de session, un par lot : elles ne sont pas listées.

| script | portable (md5) | luciepc (md5) | dépôt | appelé par |
|---|---|---|---|---|
| `poste` | 5b4e3f21eef4 | 5b4e3f21eef4 | `extension/rag3weaver/scripts/poste`, **en retard** : la copie n'a pas `nproc --all` (garde-pile-2, 8565a06bf) | toutes les passes (199 scripts au portable, 444 à luciepc) |
| `passe_epinglee.sh` | 26424d472b8b | **d2dec56afa3e** | aucun ; cité par `docs/procedures/push-d-un-lot.md` | 9 scripts au portable, 57 à luciepc |
| `lib_seule.sh` | d94c09ffa908 | d94c09ffa908 | aucun | 3 au portable, 9 à luciepc |
| `controle-du-worktree.sh` | 94d5c09b (recopié) | 94d5c09b (recopié) | `docs/optimiseur/scripts/` depuis 0cc9bbf67 | les passes de lot (17 à luciepc) |
| `rag3db_libs.py` | — | 084f46dcabc3 | `extension/rag3weaver/scripts/rag3db_libs.py`, une autre version | 15 scripts `I-*` à luciepc |
| `mesure-hote.py` | — | 6ee9ee974ef7 | aucun | 5 scripts de mesure à luciepc |
| `push_garde.sh` | d9a6c083 (sha256) | d9a6c083 (sha256) | `tools/poste/push_garde.sh`, à jour | chaque push |

À faire, un par un, en pile, comme `push_garde.sh` : `passe_epinglee.sh` d'abord (deux versions selon le poste), puis
`lib_seule.sh`, dans `tools/poste/` ; `poste` et `rag3db_libs.py` recopiés depuis master, après relecture et au mot de
c7 ; `mesure-hote.py` reste un outil de mesure, sous `docs/optimiseur/scripts/` s'il sert encore.
