# La ligne d'attente de la boîte de l'équipier est en français

- **État** : fermé le 19 novembre 2026 par la session tty (branche `cx-ligne-attente-anglais-2`). La ligne dit « "<tool>" has been waiting for N s (an index is probably holding the write); the call goes on, nothing is abandoned ». Ni le terminal, ni la page, ni l'hôte ne reconnaissaient l'ancien texte : rien d'autre ne change. Témoin : `a_waiting_tool_says_so_on_the_bus_once_and_says_when_it_resumes`, qui lit aussi la ligne, rouge (71ec1614c3), puis vert ; module `daemon_host::teammate` vert (44).
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
