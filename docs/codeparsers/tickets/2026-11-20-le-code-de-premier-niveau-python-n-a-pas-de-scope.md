# Le code de premier niveau Python n'a pas de scope

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit
- **Gravité** : réponse incomplète : ce qu'un fichier Python fait hors de ses fonctions et de ses classes (appels au
  chargement, constantes construites, enregistrements, `if __name__ == "__main__":`) ne laisse aucune référence ;
  ses usages manquent à `usages` et à `impact`
- **Atteignable en service** : oui, dans tout dépôt Python
- **Touche rag3weaver** : oui, par codeparsers (les scopes Python)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au lot des paquets importés par leur nom

## Constat

Rust, C, C++, C# et Go appellent `extract_file_scopes` : les trous entre les scopes deviennent des scopes de fichier
(`file_scope_NN`), qui portent leurs références. Le parseur Python ne l'appelle pas. Un `__init__.py` qui ne fait que
réexporter (`from monpaquet.lib import Catalog`) n'avait donc aucun scope, et sa réexportation aucun porteur : le lot
des paquets lui donne ses scopes de fichier, à lui seul.

## Ce qu'il faudrait

Des scopes de fichier pour tout fichier Python, comme les autres langages ; mesurer d'abord le delta (sonde de marque
sur `tools/python_api` et un corpus Python plus large), puisque des références nouvelles apparaîtront partout.
