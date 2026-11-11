# Le juge des outils sur Gemini range parfois un paramètre obligatoire en description trompeuse

- **État** : ouvert (7 novembre 2026), vu par l'arbre principal dans la passe du lot du juge ; pour l'arbre principal (le juge).
- **Famille** : produit
- **Gravité** : faux rouge (un témoin sur vrai modèle qui exige une classe exacte) ; peut-être une réponse moins précise du juge en service.
- **Atteignable en service** : oui pour la classe rendue (le juge en service peut ranger le même défaut sous une autre classe) ; non pour le rouge (le témoin).
- **Touche rag3weaver** : oui (`tests/e2e_juge.rs`, le gabarit du juge, `src/daemon_host/juge.rs`).

## Ce que c'est

`e2e_juge::le_juge_des_outils_sur_gemini_nomme_les_defauts_de_schema_et_de_retour` exige que le
juge sur Gemini nomme `parametre_devrait_etre_optionnel` (avec son `call_id` et sa forme) pour
l'outil de banc `chercher_dans`, qui dit chercher « dans tout le projet » mais exige `fichier`.
Une fois sur quatre, le modèle range ce même défaut en `description_trompeuse` (bloquant), avec
une preuve juste (« description : Cherche un mot dans tout le projet, alors que les paramètres
exigent obligatoirement fichier ») : le défaut est vu, sa classe diffère.

## Les chiffres (ce poste, lib 17a094d12 épinglée, `--features openai-llm`, identifiants Vertex)

| où | rouges |
|---|---|
| la passe du lot juge + graphe cité, 22 h 27 (base dcaea137f) | 1 rouge (la classe `description_trompeuse`) |
| seul, 20 h 30 (base 207d334f9) | vert |
| seul, 22 h 55, deux fois (base dcaea137f) | 2 verts |

## Le témoin

Le test lui-même ; il rougit sans changement du code, sur la seule variance du modèle.

## Ce qu'il faut pour le fermer

Trancher ce que le témoin juge : soit les deux classes sont deux lectures justes du même défaut
(le témoin accepte l'une ou l'autre, pourvu que la preuve nomme `fichier` et le `call_id`), soit
la consigne du juge doit les départager (« un paramètre exigé que la description dit facultatif
est `parametre_devrait_etre_optionnel`, pas `description_trompeuse` »), et le témoin le vérifie
sur plusieurs tirages. Jusque-là, une passe où ce seul témoin rougit le nomme ici et le rejoue
seul avant de pousser.
