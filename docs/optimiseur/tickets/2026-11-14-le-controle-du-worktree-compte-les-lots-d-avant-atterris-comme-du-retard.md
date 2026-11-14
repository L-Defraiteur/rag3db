# Le contrôle du worktree compte les lots d'avant atterris comme du retard

- **État** : ouvert (14 novembre 2026, 23 h 50), relevé de codeparsers par c7 ; corrigé dans `docs/optimiseur/scripts/controle-du-worktree.sh`, à recopier au cache des deux postes au mot de c7.
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
