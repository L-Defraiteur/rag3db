# Un import Python pointé ne produit aucune bibliothèque

- **État** : ouvert (14 novembre 2026) — relevé en écrivant le témoin de la provenance par les manifestes
  (codeparsers `provenance-manifestes-2`) ; non corrigé.
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
