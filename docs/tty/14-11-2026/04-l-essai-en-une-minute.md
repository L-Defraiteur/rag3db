# Le terminal, à réessayer en une minute

Pour Lucie, le 14 novembre 2026 au soir. À ton essai de 18 h 35, tu l'as trouvé « très prototype » : un rouge uni, et une saisie où l'on ne pouvait ni revenir en arrière ni se déplacer. Ce qui est sur master depuis (`b3f58c30e`) a été rejoué contre ton hôte (7970), dans les mêmes conditions que toi, sous tmux puis sous mosh.

## La commande

Dans un terminal du portable :

```
~/.cache/rag3weaver-build/rag3weaver-tty --depot rag3db --tache essai-tty --demande "bonjour, présente-toi en deux lignes"
```

Depuis l'autre poste, par mosh (le tunnel ssh joint l'hôte du portable) :

```
mosh luciepc -- bash -c "ssh -N -L 17970:127.0.0.1:7970 lucie-tablette & ~/.cache/rag3weaver-build/tty7-mesure/rag3weaver-tty-saisie --depot rag3db --tache essai-tty --daemon-cmd 'ssh lucie-tablette cat .local/share/rag3weaver/daemon.json' --adresse 127.0.0.1:17970; kill %1"
```

## Ce que tu verras de différent

- **Plus de rouge uni.** Seul l'en-tête de chaque ligne de la boîte garde le rouge du thème, et il est en français (`[équipier · …]`, `[toi · demande]`, `[hôte · fini]`). Le texte qui suit prend la couleur normale de ton terminal.
- **La saisie s'édite.** Tape une phrase avec une faute, puis corrige-la aux flèches : ← → déplacent le curseur, et ce que tu tapes s'insère là où il est. Début et Fin vont aux deux bouts, Ctrl-← et Ctrl-→ sautent les mots, et ↑ rappelle ce que tu as déjà envoyé. Ctrl-C vide la ligne ; un second Ctrl-C quitte. À l'essai, « merci, c'est cair » est devenu « Merci, c'est clair ! » en neuf touches.
- **Sous mosh**, la ligne du haut dit où relire la séance, `less ~/.local/state/rag3weaver/tty/seance-….txt`, puisque mosh ne garde pas l'historique. Le journal de l'essai portait toute la séance.

## Ce que tu ne verras pas encore

- **Le Markdown.** Le terminal sait rendre titres, puces, gras et italique, mais l'équipier n'en a pas écrit. Je lui ai demandé deux fois une présentation avec un titre, une liste et un mot en gras. Il a répondu chaque fois par un compte rendu, « Présentation fournie au superviseur conformément au format demandé », sans la présentation. C'est le comportement de l'équipier, pas celui du terminal : c'est signalé à l'orchestration.

  **À relire à ton prochain essai** (note du 15 nov., vers minuit) : archi a corrigé l'équipier. Désormais, `report` ou sa réponse porte le texte lui-même, et le texte d'un pas arrive dans la boîte. Le banc ne reproduisait pas ta séance (10/10 avec et sans le correctif) : le juge, c'est ton prochain essai dans le terminal, sur un hôte qui porte le correctif (il atterrit dans la nuit du 14 au 15). Demande-lui un titre, une liste à puces et un mot en gras : tu dois les voir rendus, et non plus « Présentation fournie au superviseur… ». Si le compte rendu revient à la place du contenu, le ticket de l'équipier se rouvre, avec ta séance.

## Les défauts vus à l'essai, corrigés au lot tty-9 (`05-les-defauts-de-l-essai.md`)

- Sous tmux, ta demande s'est affichée **deux fois**, alors que la boîte de l'hôte n'en a qu'une : le terminal la reçoit par deux chemins.
- La ligne d'état dit **« fini »** dès l'ouverture, avant que l'équipier ait commencé : c'est la fin du run précédent de la tâche.
- Deux genres de la boîte restent en anglais : `report` et `step_done`.
- Une ligne longue se **coupe au milieu d'un mot**, au bord de l'écran, au lieu de passer à la ligne entre deux mots.
- La phrase du repli sous mosh est trop longue pour 110 colonnes : elle est coupée par le début.
