# Le code de premier niveau Python n'a pas de scope

- **État** : corrigé (20 novembre 2026) — branche rag3db `codeparsers/python-file-scopes-export-star-2`, règles de l'analyse 12
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

## Fermé

20 novembre 2026, session codeparsers. Le parseur Python appelle `extract_file_scopes`, comme Rust, C, C++, C# et Go :
le code de premier niveau a ses scopes de fichier et leurs références (codeparsers c5a263f, `tests/python_top_level.rs`,
rouge vu puis vert). Ses liaisons ne sont pas des références (15d123d) : le texte du premier niveau est réanalysé, et ses
cibles de boucle et de compréhension, ses déballages, ses alias `as` et ses modules importés sont exclus ; le nom importé
reste une référence. La première forme, sans ces exclusions, reliait une vingtaine de liaisons par le nom à un homonyme
d'un autre fichier : la sonde l'a vu, le témoin le tient.

Sonde de marque (cinq corpus, contre 4eed1f8ef0) : aucune arête retirée ; `tools/python_api` +84 (imports et
variables de module), le crate rag3weaver entier +805. Un scope de fichier court est un `module` : la règle 2.12 le
tient hors de la recherche sous 80 caractères. Suites e2e_usages 46, e2e_code_sync 25, e2e_postgres 39 ; codeparsers
77 lots sur 77.
