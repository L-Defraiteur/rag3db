# Le témoin du chat d'une tâche rougit sous la charge d'autres passes

- **État** : ouvert (14 novembre 2026, session recherche).
- **Gravité** : faux rouge de passe. Il coûte un rejeu entier.
- **Atteignable en service** : non établi. Le témoin attend la page du chat, et sous charge la page
  n'a jamais rendu sa vue.
- **Touche rag3weaver** : oui (`src/daemon_host/chat_qui_code.rs`, `src/daemon_host/navigateur.rs`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

Le 14 novembre à 23 h 3x, à luciepc, la passe du lot des orphelins (ff4ced00a, lib 373b5477b) a
rougi un seul des 45 témoins navigateur :
`daemon_host::chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page`.

Le message :
`…/ide/code/<dépôt>/chat : « … #model === 'modèle : faux-modele' && #lien === 'flux : ouvert' »
jamais vraie en 20 s (20 s de calme) ; la vue : "" ; en vol : [] ; erreurs : []`

La page n'a donc rien rendu, sans requête en vol et sans erreur.

D'autres passes lourdes tournaient en même temps (everything declarative, banc). Rejoué seul, sur
le même target et la même lib, le témoin est vert trois fois de suite, en 2,2 à 2,4 s. Le lot ne
touche ni le chat ni l'hôte.

## Ce qu'il faudrait

Lire ce que la page attend quand elle reste vide sans rien en vol : un module qui ne se charge pas,
ou un premier rendu suspendu à un événement manqué. Ensuite, soit le témoin attend la cause, soit la
page le dit. Un délai plus long ne serait pas une réponse : 20 s de calme sans aucune requête, c'est
un blocage, pas de la lenteur.
