# Un graphe de clic vide fait répondre 422 à l'aperçu pendant une écriture en place

- **État** : corrigé (8 novembre 2026, session archi, lot `apercu-clic-vide`), relu par everything
  declarative.
- **Gravité** : faux rouge en témoin ; en service, une ligne d'erreur à la console du navigateur à
  chaque enregistrement d'un graphe de clic par un éditeur qui écrit en place. La page s'en remettait
  au passage suivant.
- **Atteignable en service** : oui, l'aperçu d'un composant à graphe de clic ouvert pendant qu'on
  enregistre son `.clic.mmd`.
- **Touche rag3weaver** : oui (`src/components.rs`, `src/components/modules.rs`, `src/ide.rs`).
- **Famille** : produit

## Ce qu'on a vu

codeparsers, 8 novembre vers 15 h 13, luciepc, charge 32 :
`apercu_a_chaud::le_graphe_de_clic_s_allume_a_chaque_clic_dans_l_apercu` rouge à 23/24, la console
disant `Failed to load resource: … 422 … /ide/apercu/<id>/Compteur/modules.js?v=…`. Le `?v=` dit que
c'est la relecture à chaud. La ligne « magasin de l'aperçu : 65 objets effacés » vient du banc
(`prune_store`, avant le démarrage de l'hôte) : il n'y avait pas de course avec un vidage du magasin.

## La cause

Le témoin réécrit le graphe par `std::fs::write`, qui tronque le fichier puis l'écrit, comme un
éditeur qui écrit en place. Sous charge, la montre des composants voit le fichier vide, et la page
relit ses modules dans cet état. Une source TSX vide se compilait déjà vide, marquée `empty_source`,
et la page attendait sa version suivante. Un graphe de clic vide était refusé, d'abord par la
vérification du contrat (`Components::load` : « missing 'graph LR' or 'graph TD' header »), puis
par sa compilation. L'aperçu répondait donc 422, et le navigateur l'écrivait à sa console.

## Le correctif

- Un graphe de clic blanc passe la vérification du contrat et se compile comme une source vide :
  un module vide, marqué `empty_source`. La page dit « comp:<Nom> est vide » et attend la suite.
- Au premier chargement de la page (sans `?v=`), un graphe de clic vide n'est pas un instant
  d'écriture : l'aperçu répond « graphe de clic vide (components/…) : rien à rendre tant qu'il n'est
  pas écrit » (relecture d'everything declarative).

Témoins, rouges d'abord : `components::tests::an_empty_click_graph_makes_the_preview_wait_instead_of_failing`
et `daemon_host::apercu_a_chaud::un_graphe_de_clic_vide_fait_attendre_l_apercu_sans_erreur`. Ce
dernier force l'ordre (le fichier vide, l'attente vue par la page, puis le texte), exige une console
vide et l'état gardé, puis ouvre la page à neuf sur un graphe vide et lit « graphe de clic vide ».
