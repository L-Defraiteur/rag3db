# Un import Python pointé ne produit aucune bibliothèque

- **État** : corrigé (14 novembre 2026) — codeparsers d425495 : le nom lié d'un `import a.b` sans `as`
  est `a`, et un nom se cherche exact puis par le plus long de ses préfixes pointés ; témoin
  `codeparsers/tests/import_python_pointe.rs`, rouge sur la forme du ticket sans le correctif.
- **Gravité** : réponse incomplète (un usage d'une bibliothèque externe n'a ni nœud `Library` ni arête
  `USES_LIBRARY`).
- **Atteignable en service** : oui, tout fichier Python qui écrit `import paquet.sous_module` (et pas
  `import paquet` ni `from paquet import …`).
- **Touche rag3weaver** : non (codeparsers : l'extraction des imports Python) — sauf le pointeur.
- **Famille** : produit

- **Pour** : session codeparsers

## Ce que c'est

Sous les options de rag3weaver (« fichier seul »), sur codeparsers c7c4b2b :

```python
import os
import requests.adapters
import yaml

def lire():
    return os.getcwd(), requests.adapters, yaml.load
```

donne les bibliothèques `os` et `yaml`, **pas** `requests` ni `requests.adapters` ; avec `import requests`,
la bibliothèque `requests` existe. La forme pointée se perd entre l'extraction des imports
(`python_scope_extraction_parser.rs`, vers les lignes 1370-1400) et `resolve_uses_library_relations`.

## Ce qu'il faut pour le fermer

1. Un témoin rouge : `import requests.adapters` donne une bibliothèque (au nom `requests.adapters`, ou
   `requests` — à trancher avec la forme de `from requests.adapters import X`, pour que les deux formes
   donnent le même nom).
2. Le chemin trouvé et corrigé ; la provenance par les manifestes s'y applique par le module racine
   (`nom_de_paquet`).

## La correction (14 novembre 2026)

**La forme exacte du défaut**, plus étroite que le titre : `import requests.adapters` donnait bien sa
bibliothèque quand la mention allait jusqu'à un attribut (`requests.adapters.HTTPAdapter`, dont le
qualificatif vaut le nom importé entier), **pas** quand la mention était le sous-module seul
(`requests.adapters`, lu comme `adapters` qualifié par `requests` : aucun import ne s'appelle `requests`).

**Le correctif** (`python_scope_extraction_parser.rs`) : le nom qu'un `import a.b.c` sans `as` lie dans le
fichier est `a` (`nom_lie_d_un_import_pointe`), ajouté à la table des imports sans écraser un import exact
de `a` ; un nom se cherche exact, puis par le plus long de ses préfixes pointés (`import_d_un_nom`), dans
la classification Python et dans `resolve_imports_for_scope`.

**Le témoin** : `codeparsers/tests/import_python_pointe.rs`, trois cas — l'attribut
(`requests.adapters.HTTPAdapter`, `os.path.join`), la forme du ticket (rouge sans le correctif), les imports
relatifs (`from . import x`, `from .outils import f`) sans bibliothèque. Suite entière de codeparsers :
301 passés, 0 échec.

C'est le premier cas de la règle générale du nom importé au paquet
(`docs/codeparsers/14-11-2026/02-du-nom-importe-au-paquet.md`), qui viendra en données, langue par langue.
