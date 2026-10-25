# Le témoin des vues de l'IDE rougit sous le lot des témoins navigateur

- **État** : ouvert, le 25 octobre 2026 à 23 h 55. Relevé par hotfixs pendant le rejeu de push de
  (A) (`create-concurrently-etat-6`), confié par l'orchestration à everything declarative. Aucun
  correctif écrit.
- **Gravité** : un faux rouge dans la passe de push. Un tour qui joue `scripts/temoins_navigateur.sh`
  peut rougir sans défaut du lot qu'il contrôle.
- **Atteignable en service** : non, c'est le témoin qui est instable ; rien ne dit que l'IDE servi
  le soit.
- **Touche rag3weaver** : oui (`src/daemon_host/navigateur.rs`, l'IDE React de 81fdc2f40).
- **Pour** : everything declarative (l'IDE React).

## Ce qu'on voit

`daemon_host::navigateur::les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur`, réécrit le
25 octobre à 23 h 15 par 81fdc2f40, échoue ainsi :

```
http://127.0.0.1:36663/ide/code/5bb2b6d35dbe/symbole/depart : « … 'Définitions (1)' … /fichier/main.rs#L5 … »
jamais vraie en 20 s ; la vue : "Chargement…" ; en vol : ["/ide/call {… \"name\":\"callees\",
\"arguments\":{\"name\":\"depart\",\"format\":\"json\"}}"]
```

La vue d'un symbole reste sur « Chargement… » pendant 20 s, l'appel `callees` toujours en vol.

## Les passes

À luciepc, tête 2bdc7715b (la -6 de (A), qui ne touche ni l'IDE ni `callees`), lib du lot
(`RAG3DB_LIB=bati`), `RAG3WEAVER_PAQUETS_REACT` posé, 25 octobre entre 23 h 44 et 23 h 52 :

| Passe | Résultat |
|---|---|
| lot des onze témoins (`scripts/temoins_navigateur.sh`, features des tours) | 10 verts, **ce témoin rouge** |
| ce témoin seul, deux fois | 2 verts sur 2 |
| lot des onze, deux fois de plus (charge 8 à 14) | 11 verts, 11 verts |

Un rouge sur trois passes du lot, aucun seul. Sous les dix autres témoins et la charge du poste,
le délai de 20 s ne suffit pas toujours à `callees`.

## Pour avancer

1. Reproduire sous charge : le lot en boucle, ou le témoin seul sous une charge de fond.
2. Lire où `callees` attend dans ce cas (le catalogue, le démarrage de l'hôte du témoin, l'index
   encore en cours) avant de toucher au délai. Un délai allongé sans la cause ne se justifie pas.
