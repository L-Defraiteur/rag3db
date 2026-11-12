# Le témoin code-tree tronque le chemin du journal aux espaces

- **État** : ouvert ; diagnostic de fixture concordant ED/archi, correction et reprise bornée à réaliser.
- **Famille** : produit
- **Gravité** : validation du parcours hôte bloquée par son témoin.
- **Atteignable en service** : non pour ce parseur de test ; aucune panne d'indexation établie par ce délai.
- **Touche rag3weaver** : oui, tests/e2e_demon_hote.rs::code_tree_pose_enregistre_clone_et_retire_un_arbre.
- **Pour** : ED, revue archi ; composer le parseur de journal déjà éprouvé dans les lectures.
- **Signalé par** : arrêt de la passe ED accords, diagnostic indépendant archi puis coordination.
- **Ouvert le** : 12 novembre 2026

## Reproduction et diagnostic

Source figée6f5276b39bdd1803dbd80d40b5352c68612e3ad3, passe ED accords.
F5/minimal/compilations/inventaires, lib2229/0/65, NAV40/0/0 et lectures27/0/0
étaient acquis ; 7879 a été rendu avec santé vérifiée. La suite hôte a atteint
la borne120s sans verdict complet, puis son superviseur a arrêté ses descendants.
Le rouge et le journal sont conservés dans
`~/.cache/rag3weaver-build/I-luciepc/accord-publication-20261112/globale/`.

Le test copie le vrai gabarit code et indexe une fixture synthétique. Son
extraction après « journal : » s'arrête au premier caractère blanc. Le chemin
réel sous `globale/racine de journaux/index/1794515589662-227077.out` devient
donc `globale/racine`. Le test lit ce chemin inexistant avec unwrap_or_default
dans une boucle dont la borne interne est300s ; la borne externe120s coupe
avant le diagnostic attendu par le test.

Archi et ED ont lu le vrai journal complet : il existe et termine par
« indexation terminée », avec trois vecteurs encore dus explicitement annoncés.
Ce fait écarte l'hypothèse d'un blocage de cette indexation faute de fournisseur
sparse. Le parcours ne décide pas d'accord ; aucun interblocage du correctif
de publication n'est démontré par ce délai. La santé des modèles et une fin
d'indexation ne prouvent pas pour autant la disponibilité de tous les signaux.

## Correction et fin attendue

Consommer la vraie réponse d'outil, avec son enveloppe JSON et son chemin de
journal complet. Réutiliser ou extraire la primitive déjà éprouvée dans
e2e_lectures plutôt que créer une nouvelle variante de split au premier blanc.
Conserver espaces, Unicode et échappements ; ne pas déplacer la fixture vers
un chemin sans espaces ni déduire le journal du dossier courant ou du dernier
fichier trouvé. Toutes les assertions d'index, usages, clone et retrait restent.

Publier source, diagnostic et témoins avant reprise. Éprouver les formats de
réponse concernés et rejouer les consommateurs modifiés ; F5 vérifie les
montages de tests si le helper devient commun. Les bibliothèques et NAV déjà
vertes sans code produit modifié ne sont pas à rejouer pour ce parseur seul.
La recette corrective hôte est une nouvelle étape bornée explicite dans le
budget global initial6000s, qui conserve les120s déjà consommées et tous les
rouges ; aucun reset ni rejeu inchangé, pas d'allongement aveugle de la suite.

Fin : chemin réel correctement extrait, tests de formats et consommateurs
modifiés verts, suite hôte13/0/0 sans ignorer code-tree, provenance et quiescence
vérifiées, correction relue puis intégrée. Fermer ce ticket dans le même lot ;
le ticket publication d'accord reste distinct et porte son propre verdict.

## Reprise ED — preuve et correction bornée

Preuves du doublon fermé fusionnées : I-luciepc/accord-publication-20261112/globale,
e2e_demon_hote.log (12/13finsok), journal réel
`racine de journaux/index/1794515589662-227077.out` déjà terminé, chemin tronqué
« racine » inexistant. Aucun decider_accord dans ce parcours, pas de deadlock établi.
PrimitiveROUGE/restes[]/nettoyétrue et quiescence indépendante[] ; propriétairePID227077
/start/cgroup/maps capturé avant arrêt. Lecture indépendante archi concordante.
ReçuHTTP précis/pile non capturés ; code+vrai journal sont la preuve du diagnostic.

GO C7 : préserver espace/Unicode/JSONescaped, aucune baisse de signaux/skip/borne.
La fonction journal_de_l_index existante de lectures est déplacée SANS changer son
comportement dans tests/common/mod.rs. Lectures importe cette primitive ; ses trois
témoins de reçu restent aux mêmes noms et la suite reste27tests. Hôte conserve le
Value brut de /call pour index puis le passe au helper commun ; les autres outils
gardent leur conversion texte. Assertions index/usages/clone/retrait et300interne
restent identiques ; la suite est toujours bornée120 par la recette. Pas de
modification Rust produit, ni modèle ajouté pour masquer une erreur de test.

Avant reprise : source publiée, archi revue seule, F5montages au jeu exact puis
compilation e2e JSON, helpers3 et consommateurs modifiés lectures27/hôte13 sur
la MÊME racine avec espace. Tous dans le reste du6000original deadline866610.959853126,
120déjàconsommés comptés ; pas de lib/NAV globale rejouée ni reset. Recette à publier
avant toute commande. L'ancienne proposition sansespace est abandonnée, jamais exécutée.
