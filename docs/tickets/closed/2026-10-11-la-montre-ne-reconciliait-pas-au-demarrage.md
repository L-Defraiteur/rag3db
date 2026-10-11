# La montre ne réconciliait pas le disque avec l'index à son démarrage

- **état** : corrigé (le commit de la réconciliation, 11 octobre 2026)
- **gravité** : réponse fausse (un état périmé rendu comme sûr, sans retard dit)
- **atteignable en service** : oui — tout fichier changé entre le dernier index et le
  démarrage de la montre (un redémarrage de l'hôte, une branche changée pendant l'arrêt)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Trouvé par la session hotfixs sur la base de Lucie (page 04 des codeparsers, `951dd5a79`).
L'index datait de 15 h 22 ; A8 a changé `transaction_manager.cpp` à 16 h 14 ; la montre
n'a démarré qu'à 17 h 10 et a pris l'état du disque comme photographie de départ. Les
fichiers changés entre l'index et le démarrage de la montre n'étaient jamais réindexés :
`callees writeRequestedCheckpointIfLastNoLock` rendait « aucune définition », avec les
numéros de ligne d'avant A7, et aucune ligne de retard ne le disait.

## La recette

Indexer ; éditer, créer et supprimer des fichiers **sans montre** ; démarrer la montre :
rien n'était repris.

## Le témoin

`backend::tests::the_watch_reconciles_what_moved_before_it_started` : index, une édition
et une création hors montre, démarrage de la montre → `usages` voit l'appelant ajouté et
le fichier neuf sans `index` ; puis une suppression pendant l'arrêt, redémarrage → les
usages du fichier supprimé partent. Unitaire :
`watch::tests::reconciling_with_the_index_says_what_moved_since`.

## La correction

Au démarrage, la montre réconcilie : la référence est **l'empreinte que l'index garde par
fichier** (`File.content_hash`, le blake3 du texte lu par l'ingestion), lue d'une requête,
sans photographie à côté. Empreinte différente → `modified`, absent de l'index →
`created`, indexé mais plus sur le disque → `removed` ; publié au premier tour, la
réaction déjà à l'écoute (les montres se montent après les réactions). Le retard se dit
pendant ce temps. Un index vide (jamais indexé) ne réconcilie pas : la montre ne fait pas
une indexation entière à la place d'`index`. Pendant un index entier, les événements de la
montre sont mis en reprise et rejoués à sa fin (`code_sync::note_change`).
