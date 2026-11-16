# Le contrôleur de corpus exige un checkout Git

- **État** : ouvert — incompatibilité du contrôleur initial avec les archives figées de Banc
- **Famille** : outillage
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
