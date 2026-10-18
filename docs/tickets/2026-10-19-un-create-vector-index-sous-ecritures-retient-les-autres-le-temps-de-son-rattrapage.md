# Un CREATE_VECTOR_INDEX sous écritures retient les autres le temps de son rattrapage

- **État** : ouvert, défaut connu (19 octobre 2026, seconde session cœur C++). Il vient du
  correctif des lignes perdues pendant le bâti, la forme (δ) : le ticket fermé
  `closed/2026-10-12-les-lignes-validees-pendant-le-bati-d-un-create-manquent-a-l-index.md`. Sa
  correction est (A), le CREATE concurrent en trois transactions (page
  `extension/rag3weaver/docs/10-octobre-2026-coeur-cpp-tickets/07-create-index-concurrently.md`,
  relue par l'orchestration).
- **Gravité** : blocage temporaire. Toutes les transactions, lecteurs compris, attendent leur BEGIN
  pendant le rattrapage. Rien n'est perdu ni faux.
- **Atteignable en service** : oui, dès qu'un index vectoriel se bâtit sous des écritures
  soutenues. C'est le cas du produit, qui reconstruit ses index pendant que la montre écrit.
- **Touche rag3weaver** : oui.

## Ce que c'est

(δ) rattrape, à la validation du CREATE, ce que les autres ont validé pendant son bâti. Il le
fait une fois les autres partis, pendant que les nouveaux BEGIN attendent : c'est la seule fenêtre
où rien ne change sous lui. Le rattrapage coûte une opération HNSW par ligne changée. Sous vingt
écrivains, des milliers de lignes ont changé, et la retenue dure autant que le rattrapage.

C'est le prix retenu par l'orchestration (19 octobre) pour la justesse : avant (δ), master
perdait ces lignes du graphe, en silence. Une réponse fausse passe avant une attente.

## Les mesures

Le scénario des vingt écrivains, `TwentyWriters`, à 2 fils de test, sur luciepc :

| | durée du CREATE | retenue max des autres |
|---|---|---|
| master 8f4779bdc, sans (δ), qui perd des lignes | 11,2 s | 7,8 s, par la famine du bâti (ticket à part) |
| avec (δ), DDL toutes les 15 s | 126 s | 115,7 s |
| avec (δ), DDL toutes les minutes | 307 s | 273,7 s |

Une forme intermédiaire a été essayée et écartée : (δ′), des passes de rattrapage avant la
validation, pendant que les autres écrivent. Elle ne converge pas, parce qu'un seul fil rattrape
ce que vingt produisent, et ses arêtes accumulées remplissent le tampon (CREATE refusé au bout
de 86 s). Elle est gardée, non commitée, dans les notes de la session.

## Témoin

À écrire avec (A) : sous vingt écrivains, aucune transaction n'attend plus d'une borne écrite
pendant un CREATE (page 07, §5). `TwentyWriters` en donne la mesure aujourd'hui, sans borne.

## Pour le fermer

(A) sur master, et le témoin vert, à 8 fils et à 2, côte à côte avec ce tableau.
