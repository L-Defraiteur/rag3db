# Le compte des liens sans bout n'est pas dans daemon_status

- **État** : ouvert (21 novembre 2026)
- **Famille** : produit
- **Gravité** : basse. Le compte existe : il est au journal du drain et dans les rapports d'ingestion et de
  réécriture. Mais `daemon_status`, ce qu'on lit d'un index, ne le dit pas.
- **Atteignable en service** : oui, à chaque synchronisation qui écarte des liens faute de bout.
- **Touche rag3weaver** : oui (`daemon_status`, l'état d'un index)
- **Pour** : session codeparsers
- **Signalé par** : c7, au lot des liens sans bout (`codeparsers/new-name-mentions-r`)

## Constat

Depuis le lot des liens sans bout, un lien écarté faute de bout est compté (`Catalog::take_dangling_links`), dit avec
un exemple dans les avertissements du drain, et porté par `CodeIngestReport.dangling_links` et
`ReingestReport.dangling_links`. c7 demandait aussi qu'un compte non nul se voie dans `daemon_status` de l'index. Ce
n'est pas fait : il faut noter le compte de la dernière synchronisation dans l'état de l'index, et que
`daemon_status` le rende.

## Témoin attendu

Une synchronisation qui écarte un lien faute de bout (un rendez-vous dont le scope n'existe pas, ajouté à une
analyse). Ensuite, `daemon_status` dit pour cet index « 1 lien sans bout à la dernière synchronisation ».
