# Le découpeur laisse passer un morceau de 53 000 caractères

- **État** : fermé (16 novembre 2026, ED) — `Chunker::chunk_lines` coupe une ligne plus longue que `max_size` en cœurs d'au plus `max_size` octets, sans contexte, et borne le contexte de chaque côté à `max_size` ; témoin `chunker::tests::a_line_longer_than_the_chunk_size_is_cut_and_the_cores_give_back_the_text` sur les cinq fichiers du banc et une ligne de 53 000 caractères, ROUGE sur 9ee1848fa (« copy_edge_case.test: a chunk of 53294 bytes ») puis vert. Les morceaux déjà posés d'une base ne changent qu'à la réécriture de leur contenu.
- **Famille** : produit
- **Gravité** : moyenne. Un morceau bien au-delà de la taille de chunk se forme. L'embarquement le
  coupe désormais au plafond du service, mais ce qui dépasse n'est pas cherchable par le vecteur.
- **Atteignable en service** : oui, pour tout dépôt qui porte un fichier de données d'une ou deux lignes
  énormes (fixtures de test, benchmarks, listes sérialisées).
- **Touche rag3weaver** : oui (le découpage des `ScopeContent_Chunk`)
- **Pour** : everything declarative (ED), propriétaire du découpage ; décision de c7
- **Signalé par** : banc (mesure B′), puis embarquement
- **Ouvert le** : 16 novembre 2026

## Ce que c'est

Le découpeur des `ScopeContent_Chunk` coupe par lignes. Une ligne plus longue que la taille de chunk
(1 500 caractères par défaut) forme donc un seul morceau, de la taille de la ligne. Sur l'index B′ du
banc (arbre 284cc00c0b, master bc554c2cee, à luciepc), 31 morceaux dépassent 12 000 caractères. Le
plus long en compte 53 294. Tous viennent de fichiers de données faits d'une ou deux lignes énormes :
`copy_edge_case.test` (53 294), `demo.test` (42 784), `duplicated.test`, `q24.benchmark`,
`large_list_of_struct_answer.txt`. Le détail est au ticket du banc,
`docs/banc/tickets/2026-11-16-un-texte-au-dessus-du-plafond-du-service-fait-tomber-tout-l-embarquement.md`.

Ces morceaux ont fait refuser en entier chaque lot de 512 par le service d'embarquement (le 413 du
plafond par texte), et l'index B′ n'a reçu aucun vecteur de scope. Ce défaut-là est corrigé chez
embarquement : le client coupe chaque texte au plafond du service, et le service coupe au lieu de
refuser le lot. Il reste le défaut d'amont : un morceau ne devrait jamais dépasser de beaucoup la taille
de chunk.

## Ce qu'il faudrait

Le découpeur borne un morceau en caractères. Une ligne plus longue que la taille de chunk se coupe en
plusieurs morceaux, sur une frontière de caractère, et chacun garde sa plage de lignes. Témoin, rouge
d'abord : un fichier d'une seule ligne de 53 000 caractères donne des morceaux tous sous la taille de
chunk (avec la tolérance du découpeur), et leur concaténation rend la ligne.
