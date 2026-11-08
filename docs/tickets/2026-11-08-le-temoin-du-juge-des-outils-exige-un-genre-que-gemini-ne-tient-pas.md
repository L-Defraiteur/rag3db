# Le témoin du juge des outils exige un genre que Gemini ne tient pas

- **État** : ouvert (8 novembre 2026), porteur everything declarative.
- **Famille** : produit
- **Gravité** : une passe complète rougit par hasard. Trois rouges le 8 novembre, chacun sur une tête qui le passait à côté. Ils coûtent un rejeu seul et une passe complète de plus, près de 30 minutes à chaque fois.
- **Atteignable en service** : non. Le juge fonctionne. C'est le témoin qui demande une forme que le modèle ne rend pas toujours.
- **Touche rag3weaver** : oui, `tests/e2e_juge.rs`, `le_juge_des_outils_sur_gemini_nomme_les_defauts_de_schema_et_de_retour` (assertion à la ligne 422).

## Ce qui se passe

Le témoin fait appeler par un faux agent un outil `chercher_dans`, que sa description dit
chercher « dans tout le projet », mais dont le schéma exige `fichier` et `ligne`. Il demande
ensuite au juge des outils, sur Gemini, de nommer les défauts. Il exige alors quatre genres
précis, chacun avec son `call_id` et une forme proposée : `parametre_devrait_etre_optionnel`,
`parametre_mal_nomme`, `retour_hors_sujet` et `retour_incoherent`.

Le défaut du `fichier` obligatoire se défend sous deux genres. Gemini le range tantôt en
`parametre_devrait_etre_optionnel`, tantôt en `description_trompeuse`, avec la même preuve («
description: "Cherche un mot dans tout le projet.", required: ["fichier", "ligne"] ») et la
même forme proposée (« Rendre le paramètre `fichier` optionnel »). Le 8 novembre :

| tête | essais | rouges |
|---|---|---|
| master 46633adcc seul | 2 | 0 |
| lot des commandes 5fcef1ea0 | 5 | 2 |
| lot de l'atelier c329efb1c | 1 | 1 |

Les autres essais de la journée étaient verts.

## Pour le fermer

Le témoin doit accepter qu'un défaut se nomme sous l'un des genres qui le décrivent. Pour celui
du `fichier` obligatoire, ce sont `parametre_devrait_etre_optionnel` ou `description_trompeuse`,
pourvu que la preuve cite `required` et que la forme proposée rende `fichier` optionnel. Ce qu'il
mesure, que le juge voit le défaut et propose la bonne correction, reste exigé. Une variante,
plus juste encore : demander au juge un genre principal et des genres voisins, et accepter le
défaut s'il figure parmi eux.
