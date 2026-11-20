# Le lanceur ne fournit pas la lib à un cas qui en a besoin

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : un cas dont le témoin demande les features `code` et `rag3db-native` ne se joue pas.
  Le rejeu de nuit du motif `*bundle*` d'ED (ed9568a811) en a été écarté.
- **Atteignable en service** : non, c'est l'outillage de la campagne
- **Touche rag3weaver** : non (`tools/campagne/lanceur.py`)
- **Signalé par** : archi, le 21 novembre 2026, aux rejeux de nuit

## Ce qui manque

La règle est écrite dans la page du lanceur (`19-11-2026/04-…`, depuis le 19 novembre) : un cas
`needs_lib: true` exporte la lib **épinglée du magasin** (`RAG3DB_LIB`, puis `exporter_la_lib`) pour le
rouge au parent comme pour le bac à sable de l'agent, et ne la bâtit jamais. `prepare` et `play`
l'ignorent encore. Ils lancent `cargo test` sans lib, et un témoin sous `rag3db-native` ne compile
pas.

## Ce qu'il faut faire

1. `prove_red` : pour `needs_lib`, exporter la lib de `courant` dans l'environnement de la commande, et
   ajouter les features du témoin (`code,rag3db-native`, ou celles que le bloc déclare dans
   `features`).
2. `play` : poser la même lib dans `sandbox.env` du gabarit figé (`RAG3DB_LIB` et le chemin des
   bibliothèques), pour que le cargo de l'agent la trouve.
3. `case.json` nomme la lib employée.

## Comment le prouver

Un témoin rouge : un cas synthétique dont le test exige une variable de lib refuse au parent sans
elle, puis se prouve rouge sur l'assertion avec elle. Ensuite, le cas `*bundle*` d'ED se prépare et
se joue.

## Taille et blocage

Une demi-journée. Il ne bloque que les cas à lib de la campagne.
