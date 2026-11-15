# tty-11 : les appels d'outil qu'on lit, et Ctrl-O

Session tty, 15 novembre 2026, au mot de Lucie. Elle voulait d'abord que l'agent lise un fichier, pour montrer la coloration. Puis : « les bouts de code embeddés genre ```typescript … ``` doivent aussi apparaître highlightés, quand c'est dans un md par exemple ». Et enfin : « extensible l'extrait à l'appel d'outil lecture/écriture… un clic et hop ça s'ouvre plus grand sans faire perdre le scroll, en général les appels d'outils ».

## Ce qui change

- **Chaque appel d'outil est numéroté** et dit ce qu'il vise : `» read_file src/vue.rs  ⟨3⟩`.
- **Une lecture montre un aperçu** : les 12 premières lignes de ce que `read_file` a lu, dans la langue du fichier, colorées par les grammaires de l'hôte (`POST /highlight`). Une note suit, avec ce qui reste (« 100 lignes de plus : Ctrl-O ⟨1⟩ »). Un `.md` se rend comme de la prose : titres, puces, gras, et chaque bloc ```` ```<langue> ```` coloré dans sa langue.
- **Une écriture montre ce qu'elle écrit** : le remplacement d'`edit_file` (`old` vers `new`) en lignes retirées (`- `, la teinte des commentaires) et ajoutées (`+ `, celle des chaînes), ou le début du `content` écrit, dans la langue du fichier. Les palettes de tty-12 leur donneront des rôles à elles.
- **Un bloc de code dans la boîte** (un compte rendu, une réponse de l'équipier) est un bloc de code de sa langue, comme dans le texte en flux du modèle.
- **Un extrait coupé le dit.** L'hôte ne rend que les 4 premiers Ko d'un résultat (`result_excerpt`), avec sa taille entière (`bytes`). L'aperçu comme la visionneuse disent alors « extrait, 4 Ko sur 16 Ko », et un extrait coupé ne paraît jamais entier. Le résultat entier demandera une route à l'hôte, plus tard.
- **Ctrl-O ouvre la visionneuse** dans un écran alterné, celui de `less` ou de `vim`. Elle montre l'appel en entier, avec ses arguments. Ses touches :
  - ↑ et ↓ défilent, Page précédente et Page suivante (ou l'espace) sautent d'une page, Début et Fin vont aux bouts ;
  - ← et → passent à l'appel d'avant ou d'après ;
  - Échap, `q`, Ctrl-O ou Ctrl-C ferment.

  L'écran principal, son défilement et l'historique du terminal restent intacts derrière, et reviennent tels quels. Le pied commence par « Échap fermer », parce qu'un écran étroit coupait la fin du pied (vu par son test).

**Le clic sur une ligne n'est pas fait.** La vue écrit dans le défilement du terminal lui-même. Capter la souris lui retirerait la molette, et elle ne sait pas où sont ses lignes une fois qu'on a défilé. Un mode plein écran, qui tiendrait son propre historique et déplierait sur place, reste une piste pour un lot à part, proposée à Lucie, avec son prix : refaire le défilement, la recherche et la copie.

## Les témoins

Chaque morceau a été vu rouge avant son code :
- le module des appels (`src/outils.rs`) : numérotation, cible, lecture d'un extrait, aperçu, `.md` en prose, écriture, extrait coupé, chaîne JSON lue même coupée ;
- la vue : l'appel gardé et son aperçu, un bloc de la boîte, Ctrl-O ;
- la visionneuse (`src/visionneuse.rs`) : ses touches, son écran ;
- la teinte d'un remplacement, et la traduction de Ctrl-O et des pages.

Le crate passe de 66 à 80 tests avec la vue, et en garde 14 sans elle.

Un trou dans l'historique de la branche : le premier commit rouge (`84257c530`) déclarait le module `outils` sans le fichier, qui n'était pas suivi. `git commit -- <chemins>` ne prend pas un fichier non suivi. Ce commit ne compile donc pas seul ; le suivant (`c3100d5a3`) ajoute le fichier. L'historique poussé n'a pas été réécrit.

Le harnais (`mesures/mesure.py`, séance 1f) :
- le faux hôte lit un fichier de 400 lignes, rendu en extrait coupé à 4 Ko sur 16, puis écrit un remplacement ;
- le harnais vérifie les numéros des deux appels, la mention de l'extrait coupé, l'ouverture de la visionneuse sur le dernier appel, l'appel d'avant, Fin, puis Échap ;
- sous tmux, il compare l'écran principal capturé avant et après (`capture-pane`).

| Passe | Binaire | Verdict |
|---|---|---|
| `mesures/outils-ssh-rouge-avant.json` (ssh) | tty-10 | ROUGE : ni numéros, ni aperçu, ni visionneuse |
| `mesures/outils-ssh.json` | tty-11 | VERT |
| `mesures/outils-mosh.json` | tty-11 | VERT |
| `mesures/outils-tmux-ssh.json`, `outils-tmux-local.json` | tty-11 | VERT, l'écran principal identique avant et après la visionneuse |

La première passe de tty-11 a trouvé un vrai défaut. Le faux hôte écrit son JSON comme Python, avec une espace après les deux-points (`"result": "…"`), et la lecture de la chaîne l'exigeait collée, comme l'écrit l'hôte en Rust : aucun aperçu ne paraissait. Le JSON admet ces blancs, et le code les admet désormais : test rouge puis vert. Deux autres rouges de cette passe venaient du harnais lui-même :
- ratatui ne réécrit que les cellules qui changent, de sorte que le rang « ⟨1/2⟩ » ne part pas entier. Le harnais guette maintenant une ligne que seule la lecture porte ;
- tmux et mosh traduisent l'écran alterné, dont les séquences ne se guettent donc pas sous eux. Sous tmux, la comparaison des écrans juge.

## L'essai réel

Sur l'hôte du portable, sous tmux (`mesures/essai-reel-outils-tty11.txt`), l'équipier a lu `docs/lucivy/13-11-2026/outils/cout-envoi/src/main.rs` et une page de docs de mars, puis a répondu par un `report` contenant un bloc TypeScript :
- l'aperçu du Rust est coloré, avec « 100 lignes de plus : Ctrl-O ⟨1⟩ · extrait, 4 Ko sur 6 Ko » ;
- la page `.md` est rendue en prose ;
- le bloc TypeScript de la boîte est coloré (`class`, `TaskQueue`, ponctuation) ;
- Ctrl-O ouvre la visionneuse sur ⟨4/4⟩, et l'écran principal est identique après Échap.

Un premier essai, avec des fichiers de `extension/rag3weaver/tty/`, a vu les deux lectures refusées : le worktree de la tâche `essai-tty` part de `d61fe30d7`, où ces fichiers n'existent pas. L'équipier y a déclaré « exemple TypeScript fourni » sans rien fournir. C'est signalé à archi, avec le run (`agent-1a1df060b51-346`).
