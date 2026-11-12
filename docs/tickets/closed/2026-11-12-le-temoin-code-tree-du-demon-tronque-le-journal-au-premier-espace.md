# Le témoin code-tree du démon tronque le journal au premier espace

- **État** : remplacé ; doublon du ticket canonique6bf5bd4ea, preuves fusionnées sans abandon du défaut.
- **Famille** : produit
- **Gravité** : témoin bloqué sur un chemin de journal inexistant, masque la fin d'indexation réelle.
- **Atteignable en service** : test e2e_demon_hote avec RAG3WEAVER_JOURNAUX_DIR contenant un espace ; pas de blocage produit démontré.
- **Touche rag3weaver** : oui, tests/e2e_demon_hote.rs::code_tree_pose_enregistre_clone_et_retire_un_arbre.
- **Pour** : recherche pour la fixture de reçu/journal (à coordonner avec C7) ; ED conserve les preuves du lot accords.
- **Signalé par** : ED, globale source6f5276b39 du12novembre2026.
- **Ouvert le** : 12 novembre2026.

Source6f5276b39, e2e_demon_hote.rs605-611 : le reçu est converti en String puis
split("journal : ") et take_while(!is_whitespace && !=\\ && !=guillemet).
La racine privée du driver est « racine de journaux ». Le témoin lit donc
I-luciepc/accord-publication-20261112/globale/racine, qui n'existe pas, puis boucle
jusqu'à300s en attendant « indexation terminée ». La borne externe120s arrête la
suite ; douze autres tests ont fini verts, aucun verdict final13 n'est rendu.

Le seul journal d'index du PID227077 est
`racine de journaux/index/1794515589662-227077.out` ; il contient un index plein texte
fini, deux scopes, quatre relations, puis « indexation terminée — reste dû :3morceaux
sans vecteur ». Ce n'est pas une preuve d'un deadlock du nouveau verrou d'accord.
Le dernier reçu HTTP n'a pas été capturé dynamiquement : extraction déduite du code
et corroborée par journal réel terminé/chemin tronqué absent. Aucun nouveau run pour
ce diagnostic. Primitive finaleROUGE/restes[]/nettoyétrue, quiescence indépendante[].

Preuves aux deux postes : I-luciepc/accord-publication-20261112/globale :
e2e_demon_hote.log, diagnostic-journal-demon.json, quiescence-arret-demon.json,
provenances/227077.json et227077.maps, supervision/validation-6ogi87eb/{enfants,verdict}.json.
Le rouge initial reste conservé ; source/runtime du lot accords ne corrige pas cette fixture.

Fin attendue : lire le reçu structuré Value/result/champjournal ou sa vraie ligne
complète, en préservant espaces/échappements comme les témoins lectures déjà corrigés ;
témoin de non-régression et e2e_demon_hote dans une racine privée arbitraire avec espace.
Ne pas allonger simplement la borne ni imposer un nom de dossier. Si reprise du lot
accords avec chemin sans espace, elle doit être annoncée comme adaptation de recette
avec cette dette ouverte, pas comme correction de portabilité de la fixture.

Ticket canonique conservé : docs/tickets/closed/2026-11-12-le-temoin-code-tree-tronque-le-chemin-du-journal-aux-espaces.md.
Ce déplacement ferme seulement le doublon documentaire, pas la correction ni la validation.
