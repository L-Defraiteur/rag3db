# L'équipier tourne encore après 15 min sur Gemini 3.8 flash

- **État** : ouvert (8 novembre 2026), relevé par hooks-essai dans la passe de tache-sans-git ;
  intermittent nommé, la passe est comptée verte par ce ticket au mot de c7.
- **Famille** : produit
- **Gravité** : témoin intermittent. Un essai réel de l'équipier dépasse sa borne de 15 min.
- **Atteignable en service** : à départager. Un équipier qui ne finit pas sa tâche se verrait
  aussi en service, si c'est le modèle qui boucle.
- **Touche rag3weaver** : oui (`tests/e2e_equipier.rs`, la boucle de l'équipier ; peut-être le
  modèle).

## Ce que c'est

`e2e_equipier::l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges`, sur Gemini 3.8 flash
(`google/gemini-3.8-flash @ global`), a rougi par « l'équipier tourne encore après 15 min »
(`e2e_equipier.rs:129`, 900,72 s). Juste avant, le juge avait noté 9 le bilan de la tâche
(« écris la documentation de ce petit dépôt rust… »). La tâche était donc faite, mais l'équipier ne
s'est pas arrêté.

- **Quand** : le 8 novembre 2026, de 12 h 28 à 12 h 43, à luciepc, dans la passe de
  tache-sans-git (tête a767585eb, lib cbc06c3e1).
- **La charge** (le résumé de run_e2e.sh) : CPU au plus 90 % (12 h 34), RAM 29 %, swap 20,6 Go,
  card2 à 17 % avec 18,4 Go de VRAM, attente CPU 9 %. luciepc était plein (6 lourds sur 6, avec
  les rejugements de codeparsers).
- **Rejoué seul**, sur la même tête et peu après : **vert en 260 s**.
- **Ce qu'on ne sait pas** : le dossier gardé (`e2e-gardes/e2e_equipier-20261108-124317-788772`)
  n'a que `checkpoints/`. La boîte de l'équipier vivait dans le tempdir du test, effacé avec lui :
  on ne sait pas ce qu'il faisait pendant les dix dernières minutes.

## Une seconde forme (8 novembre, après-midi)

Dans la passe de git-init-identite (tête 5e194f390, toute la passe sous `GIT_CONFIG_GLOBAL=/dev/null
GIT_CONFIG_NOSYSTEM=1`), le même test a rougi autrement : « le juge n'a pas jugé chaque pas en 10
min : {"declare":true,"fiches":[]} » (`e2e_equipier.rs:162`, 635 s). Rejoué seul avec la config
git normale, il est vert en 567 s.

La config vide est une cause possible : le juge écrit ses fiches par `declare`, qui commite sous
l'identité de la machine. Mais aucun refus d'identité (« Author identity unknown », « git a refusé
le commit ») n'est dans le journal de la passe : ce n'est pas prouvé. La cause reste à
départager, avec celle de la première forme.

## Ce qu'il faut pour le fermer

- Que le test garde la boîte et le fil de l'équipier quand il rougit, dans son dossier gardé, pour
  qu'on lise s'il boucle sur un outil, attend une réponse ou relance des pas.
- Puis départager la cause : le modèle qui ne dit pas qu'il a fini ; ou la boucle qui ne s'arrête
  pas quand le bilan est posé ; ou la machine chargée, qui allonge chaque appel.
- Témoin : le test vert sur dix tirages sous charge ; ou bien la cause corrigée, avec son témoin.
