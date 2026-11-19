# Un appel n'attend plus une ouverture sans fin

Session tty, 19 novembre 2026, au mot de c7. Ticket repris : [« Un appel d'outil attend derrière un index sans le dire »](../../arbre-principal/tickets/2026-10-11-un-appel-d-outil-attend-derriere-un-index-sans-le-dire.md), ouvert depuis le 11 octobre. Son porteur d'origine (proto) n'existe plus.

## Ce que le relevé a trouvé

Le chemin d'un appel a été relevé, de l'entrée de l'hôte jusqu'aux nœuds de recherche ; le détail est au ticket. Les lectures du catalogue étaient déjà bornées (1 s, puis « Catalogue occupé »). Une attente ne l'était pas, du côté de l'hôte : le slot du backend en lecture. Une ouverture tient ce slot en écriture tout le temps de son schéma et de ses index, 26 s à l'hôte de Lucie le 11 octobre. Un appel arrivé pendant ce temps l'attendait sans limite, et n'avait aucune réponse avant la fin.

## Le correctif

`DaemonHost::lire_le_slot` lit le slot avec une borne de 5 s (`SLOT_READ_WAIT`). Au-delà :
- pendant une ouverture, l'appel répond « <manifeste> s'ouvre depuis N s (son schéma et ses index se chargent) : cet appel n'a rien fait, recommencez dans un moment » ;
- sinon, qu'une fermeture ou une réouverture tient le backend.

L'ouverture note son début dans `Hosted::opening`, que toute sortie efface.

## Les preuves

- Témoin `e2e_demon_hote::a_call_during_a_long_open_says_the_backend_is_opening` : une ouverture de 15 s, et un appel fait pendant. Vu rouge (d985884ee9 : l'appel attend 14,7 s), puis vert. Le refus dit l'ouverture et sa durée, et l'appel d'après est servi.
- La suite `e2e_demon_hote` entière reste verte (27).
- Les deux passes ont été jouées sous la lib épinglée 3d706f709 avec `RAG3WEAVER_MOTEUR_ANCIEN=1`, en le sachant : master a reçu ce matin du C++ de l'index vectoriel plus neuf que cette lib, et le lot ne touche que l'hôte.

## Ce qui reste, pour l'arbre principal

Comme c7 l'a dit, c'est au ticket :
- le premier appel attend l'ouverture entière, sans doute le rebâti du HNSW ;
- des `catalog.lock()` sont sans borne hors du chemin de la vue ;
- la requête d'une recherche fait la queue au service d'embarquement.
