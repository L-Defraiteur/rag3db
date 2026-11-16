# Le pas suivant ne voit plus ce que le pas d'avant a lu

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : bloquant pour une tâche longue : chaque pas relit tout (D6).
- **Atteignable en service** : oui, toute tâche de plus d'un pas.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (S2)

## Ce que c'est

Chaque pas repart d'une session neuve (test `each_step_starts_from_a_fresh_thread_with_the_request_and_the_steps_done`,
`teammate_tests.rs:1024`, décidé le 8 novembre pour borner les jetons : 5 275 au pas 1, 122 242 au
pas 10 avec tout le fil). Le message de frontière (`frontier_message`, `teammate.rs:936`) ne porte
que la demande et les `step_done` d'avant. Les retours d'outils du pas N sont perdus pour le pas
N+1 : l'agent relit tout.

## Ce qu'il faut faire

Porter au pas N+1 un **résumé borné du pas N** : les chemins lus avec leurs plages et leur
empreinte, les fichiers écrits, la dernière sortie de commande tronquée, et le dernier texte du
modèle. Le budget est réglable (`teammate.carry_over_chars`, défaut 8 000). C'est un bloc
`Previous step` du message de frontière, en anglais. Le fil neuf par pas reste : on ne renvoie pas
tout l'historique. L'empreinte de lecture (`empreintes.rs`) reste valable d'un pas à l'autre, pour
qu'un `replace_lines` n'exige pas une relecture inutile.

## Comment le prouver

- Rouge : faux modèle, pas 1 = `read_file src/lib.rs` puis texte sans `step_done` ; au pas 2, la
  frontière contient `src/lib.rs` et sa plage, et un `replace_lines` sur ces lignes passe sans
  relecture.
- Le témoin des jetons du 8 novembre reste vert (le pas 2 reste petit).
- Passe comme le ticket du plafond.

## Taille et blocage

1 jour. Après le plafond (même zone de `teammate.rs`).

