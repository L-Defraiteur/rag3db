# Le contrôleur de corpus exige un checkout Git

- **État** : corrigé — 23f2bea8a6 ; témoin sans .git rouge rc128 puis vert, gel altéré refusé, sept tests verts
- **Famille** : produit
- **Gravité** : bloque les corpus conformes au contrat d'acquisition
- **Atteignable en service** : non, banc isolé
- **Touche rag3weaver** : non, contrôleur documentaire uniquement
- **Pour** : codeparsers
- **Signalé par** : orchestration et Banc le 16 novembre

Le contrat c96 exige identité, commit et octets figés, pas un checkout Git.
Le contrôleur initial appelle pourtant `git rev-parse HEAD` sur toutes les racines.
Banc fournit `source/`, `freeze.json`, `source.tar.gz`, `inventory.jsonl`.
Créer un témoin archive minuscule sans .git, constater le refus avant correction,
puis accepter uniquement un gel cohérent (identité/racine/hash archive/inventaire).
Conserver le refus des gels altérés. Aucun code public ni Cargo à exécuter.

## Preuve de correction

`test_controller.py` : 7 tests verts en 0,092s ; aucun Cargo ni code public
exécuté. Le chemin réel du superviseur est également vérifié avant lancement.
L’en-tête Famille initial « outillage » a été refusé par check_tickets ;
rectifié en « produit » dans ce commit de clôture.
