# La copie de l'hôte se reconnaît par un champ, pas par son texte français

- **État** : corrigé. Lot tty-28 (18 nov. 2026) : le champ `origin: "host_copy"` est sur master (archi, `pas-va-au-bout`), et le terminal le lit ; page `docs/tty/18-11-2026/09-tty-28-la-copie-de-l-hote-par-son-champ.md`.
- **Famille** : produit
- **Gravité** : gêne latente. Si l'hôte passe ses textes en anglais (règle de Lucie du 16 nov.), le préfixe « le pas N s'est terminé sans step_done ; sa réponse : » réapparaît à l'écran.
- **Atteignable en service** : oui, à chaque pas fini sans `step_done` qui a répondu en texte.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`, et l'hôte `src/daemon_host/teammate.rs`).

## Ce que c'est

À la fin d'un pas sans `step_done`, l'hôte pose dans la boîte la réponse du modèle, préfixée en français (`src/daemon_host/teammate.rs`, vers la ligne 1476 : « le pas {n} s'est terminé sans step_done ; sa réponse : {texte} », ou « … ni réponse »). Le terminal (`src/vue.rs`, `sans_mot_d_hote`) reconnaît ce texte pour ne pas afficher le préfixe, et ne réaffiche pas la réponse si elle est identique au texte streamé.

## Ce qu'il faut faire

- **Chez archi** : poser un champ sur cette ligne, par exemple `origin: "host_copy"` (ou `copy_of: "final_text"`), avec la réponse seule dans `text` ; et pour « ni réponse », `origin: "host_copy"` avec `text` vide.
- **Dans le terminal** : lire ce champ dans `ligne_de_boite` ; garder la reconnaissance du texte français tant que des boîtes anciennes existent, et la retirer plus tard.

## La preuve

Un test de la vue, rouge d'abord : une ligne `origin: "host_copy"` identique au texte streamé ne s'écrit pas ; une différente s'écrit sans préfixe. Le test actuel `the_answer_streamed_is_not_shown_again_by_the_copies_of_the_end_of_the_step` reste vert.

## Taille et blocage

1 heure côté terminal. Bloqué par le champ d'archi.
