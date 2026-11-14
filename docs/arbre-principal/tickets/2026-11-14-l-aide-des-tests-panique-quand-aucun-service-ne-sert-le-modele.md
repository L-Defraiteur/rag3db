# L'aide des tests panique quand aucun service listé ne sert le modèle demandé

- **État** : ouvert (14 novembre 2026), arbre principal.
- **Famille** : produit
- **Gravité** : un rouge d'environnement à chaque passe à luciepc qui joue `e2e_estimate` sous les
  services ; il se rejoue vert à part, mais il coûte la passe à qui ne le sait pas.
- **Atteignable en service** : non (les tests).
- **Touche rag3weaver** : oui (`tests/common/mod.rs`, `burn::par_le_service`, l. 241-249).
- **Pour** : embarquement.
- **Signalé par** : arbre principal, passe des suivies à luciepc, 14 novembre 2026, 23 h 12 ; sur la
  décision de c7 pour la forme attendue.

## Ce qu'on voit

`e2e_estimate::ce_depot_s_estime_avant_de_s_indexer` rougit à luciepc sous
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879` :

> RAG3WEAVER_EMBED_SERVICE : aucun service ne sert granite-107m — 127.0.0.1:7878 sert granite-278m ;
> 127.0.0.1:7879 sert bge-m3

`par_le_service` appelle `DaemonEmbedder::from_service(modele)` ; une variable posée qui ne sert pas le
modèle rend `Some(Err)`, et l'aide **panique** (`unwrap_or_else(|raison| panic!(…))`). Sans la variable,
elle rend `None` et le modèle se charge sur place. Le 7880 (granite-107m) est arrêté et désactivé à
luciepc depuis le 12 novembre (mot de Lucie, ticket fermé
`2026-11-12-revoir-les-services-d-embarquement-residents.md`) : aucune passe à luciepc qui embarque par
service ne peut donc jouer cette suite verte.

Rejoué sans la variable (le modèle sur place), le test est vert : ce n'est pas le code testé.

## Ce qu'on voudrait (décision de c7)

**Le repli, pas « SAUTÉ »** (un test sauté n'est pas vert) : pour un modèle qu'aucun service listé ne
sert, l'aide **charge le modèle sur place en le disant**, comme sans la variable (« ▸ granite-107m sur
place : aucun service listé ne le sert »). La panique reste pour un service listé qui sert le modèle mais
refuse ou se trompe de dimension : là, c'est une vraie panne.

## Le témoin à écrire

Sous une variable qui liste des services dont aucun ne sert le modèle, `par_le_service` rend `None` et le
dit ; sous une variable dont un service sert le modèle, il rend ce service ; un service qui sert le modèle
avec une autre dimension reste une panique nommée.
