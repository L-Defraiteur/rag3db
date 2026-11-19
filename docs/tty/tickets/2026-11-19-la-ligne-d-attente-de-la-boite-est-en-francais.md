# La ligne d'attente de la boîte de l'équipier est en français

- **État** : ouvert. Relevé par archi à la relecture de `cx-chat-waiting-2` (19 nov. 2026), écrit par la session tty ; pour tty, au prochain lot qui touche ce texte.
- **Famille** : produit
- **Gravité** : gêne. Un texte que l'hôte écrit pour le modèle est en anglais (règle du 16 nov.), et celui-ci ne l'est pas.
- **Atteignable en service** : oui, à chaque outil d'un pas d'équipier qui dépasse `teammate.tool_wait_notice_s`.
- **Touche rag3weaver** : oui (`src/daemon_host/teammate_tools.rs`, `Etanche::par_le_backend`).

## Ce que c'est

Au-delà du seuil, l'hôte écrit dans la boîte de l'équipier « « <outil> » attend depuis N s (un index tient sans doute l'écriture) ; l'appel continue, rien n'est abandonné ». Le modèle relit la boîte à sa frontière : la ligne doit être en anglais, comme les autres lignes de l'hôte qu'il relit.

## Ce qu'il faut faire

La ligne en anglais. Le terminal la montre-t-il ? Il la reconnaît par son texte : à vérifier dans `tty/src/vue.rs` avant de la changer, et à changer avec lui.

## La preuve

Un test de l'équipier, rouge d'abord : la ligne d'attente est en anglais, et le terminal la rend comme avant.
