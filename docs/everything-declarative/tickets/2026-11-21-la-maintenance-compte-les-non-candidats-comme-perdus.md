# La maintenance du plein texte compte les non-candidats comme des écritures perdues

- **État** : ouvert (21 novembre 2026, matin)
- **Famille** : produit (l'état d'une base, la garde des comptes du plein texte)
- **Gravité** : un faux avertissement, mais alarmant. Sur le dépôt de la tablette, l'état dit « plein texte de « Scope »
  au relevé d'il y a 169 min : 102191 documents pour 113518 lignes, dont 0 orphelin(s) — des écritures ont été
  perdues ». Aucune écriture n'est perdue : l'écart est fait des non-candidats que le repassage en règles 13 a écartés du
  plein texte.
- **Atteignable en service** : oui, sur toute base dont une règle de non-candidats retire le plein texte, après la
  maintenance de la nuit.
- **Touche rag3weaver** : oui (`src/catalog.rs`, `refresh_noted_counts_from_maintenance`).
- **Pour** : everything declarative
- **Signalé par** : ED, à la relecture de `daemon_status` après la pose de l'hôte du matin.

## La cause

La garde des comptes compare des documents à des lignes. Depuis la garde 2.12, la fin d'un index compte les **lignes
candidates** (`candidate_rows_of` : les lignes, moins celles des règles qui retirent le plein texte). Mais la maintenance
du plein texte (03 h 30) rafraîchit ces comptes notés avec `graph_docs`, le nombre de **toutes** les lignes de la table
(`refresh_noted_counts_from_maintenance`). L'écart revient donc à chaque maintenance, et l'état le dit comme une perte.

## Ce qu'il faut pour le fermer

La maintenance note `rows` = les lignes candidates de l'entité, comme la fin d'un index. Témoin rouge d'abord : une
base avec des conteneurs repliés écartés, une maintenance, puis l'état sans avertissement de perte.
