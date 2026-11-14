# La recherche par nom de l'archiviste lit un classement tronqué

- **État** : corrigé le 14 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96).
- **Famille** : produit
- **Gravité** : réponse fausse. Une section crée une fiche alors qu'une fiche voisine existe : le même sujet se disperse.
- **Atteignable en service** : oui, depuis la première passe sur la mémoire de Lucie (95 fiches créées, quatre paires voisines restées séparées).
- **Touche rag3weaver** : oui, le gabarit `memory` : `archive_subject.mmd`, `archive_subject.rhai` et `archive.mmd`.
- **Pour** : session mémoire.
- **Signalé par** : [la première passe de l'archiviste](../../memoire/14-11-2026/01-la-premiere-passe-de-l-archiviste.md) ; l'orchestration (c7) y a vu l'odeur d'un plafond silencieux.

## Ce qu'on voyait

Quatre paires de noms voisins, par la règle de `sujets_voisins`, sont restées séparées : « Client
terminal rag3weaver-tty » et « client tty de rag3weaver » ; « Hooks de rag3weaver » et « hooks
Claude Code de rag3weaver » ; « Hooks de rag3weaver » et « hooks de rag3weaver vers la mémoire
longue » ; « Vues par worktree » et « modèle des vues par worktree ».

La mesure a été faite sur la base rendue, par `recall_fiches` avec le nom de l'une pour requête.
Rendue dans les 8 premières, la voisine ne l'est que dans deux sens sur huit, au 2e rang. Dans les
six autres, elle se classe 5e, 14e, 20e, 29e, 37e et 38e sur 116. L'étage `archive_subject`
cherchait par `search_structured` avec les options des candidates (`limit: 8`) : un classement
tronqué, là où il faut une égalité.

## Le correctif

L'étage `archive_subject` compare le nom proposé à **tous** les noms de fiches en base. Il passe par
`select_structured` (`limit: 0`), une sélection sans classement, et non plus par un classement. Il
garde le nom exact d'abord ; sinon, parmi les voisines, celle qui diffère du moins de mots. Les
options de la recherche des candidates ne le bornent plus.

## Le témoin

`e2e_archiviste::la_recherche_par_nom_trouve_la_voisine_parmi_cent_vingt_fiches` : 120 fiches. La
seule voisine est « Hooks de rag3weaver », avec un `about` sans rapport. 119 leurres, « Claude Code
de rag3weaver, essai k », ont un `about` qui contient le nom proposé. La section, sans candidate et
avec `search_options: {"limit": 1}`, répond create avec « hooks Claude Code de rag3weaver ».

Rouge sur master 6b6e12f8f (aucun apport sur la voisine), vert après. `e2e_archiviste` : 12 sur 12.

Les deux premières formes du témoin (des leurres sans « rag3weaver », puis `limit: 1` seul) étaient
vertes sans correctif : la voisine se classait encore première par son nom. Il a fallu des leurres
plus proches que la voisine pour rendre le classement visible.

## Ce qui reste

- Une sélection de toutes les fiches par section créée coûte une lecture de la table Fiche par
  section. C'est négligeable à 116 fiches. À plusieurs milliers, un filtre sur les mots du nom
  (`contains`) la réduirait.
- La fusion par le sens (des noms sans mots communs) reste une piste, à Lucie.
