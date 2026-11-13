# La mesure du terminal à travers ssh (tty-5)

13 novembre 2026, session tty. Le terminal (`rag3weaver-tty`) est mesuré à travers ssh dans les deux sens : du portable de Lucie (`lucie-tablette`) vers luciepc, puis de luciepc vers le portable.

## Ce qui n'a pas pu se mesurer

- **tmux et mosh** ne sont installés sur aucun des deux postes. L'installation demande le mot de Lucie (c7), elle n'a donc pas été faite : leur mesure attend, et le même harnais s'y prêtera.
- **Windows et macOS** : aucune machine ici. Le plan de test est plus bas, et le ticket `docs/tty/tickets/2026-11-13-le-terminal-sous-windows-et-macos-a-jouer.md` attend une machine.

## Comment on mesure

Deux outils, tous deux dans `extension/rag3weaver/tty/mesures/`, qui n'utilisent que la bibliothèque standard de Python :
- **`faux_hote.py`**, un faux hôte rag3weaver. Il sert ce que le terminal lit (`/ide/etat`, `/ide/teammate`, `/ide/teammate/list`, `/ide/describe`, `/highlight`, le flux SSE `/ide/flux`). Il note chaque POST, et joue un pas scripté : un appel au modèle, 200 tokens, un bloc de code, un outil qui écrit, puis une demande d'accord.
- **`mesure.py`**, le harnais, lancé sur le poste où l'on regarde. Il démarre le faux hôte sur l'autre poste, puis le terminal là-bas par `ssh -tt`, dans un pseudo-terminal qu'il pilote et dont il horodate chaque octet. Il répond à la demande de position du curseur (`ESC[6n`) comme le ferait un vrai terminal.

```bash
python3 extension/rag3weaver/tty/mesures/mesure.py <hôte ssh> <binaire distant> <faux_hote.py distant> <dossier distant> <thèmes.json> <sortie.json>
```

Le binaire est `rag3weaver-tty` en release, bâti du commit `361f08103` (2,2 Mo). Les résultats bruts sont dans `docs/tty/13-11-2026/mesures/`.

## Les chiffres

Les deux postes sont sur le même réseau (ping de 5 à 85 ms ce soir, 32 ms de moyenne). La charge de luciepc tournait autour de 12, avec trois travaux lourds d'autres sessions.

| | Portable → luciepc | luciepc → portable |
|---|---|---|
| **Latence d'affichage** (un token publié par l'hôte → ses octets reçus à l'écran), 20 essais | médiane 47,2 ms, p95 48,9, max 72,2 | médiane 47,5 ms, p95 49,6, max 51,1 |
| **Écho d'une frappe** (une touche → son écho), 20 essais | médiane 20,3 ms, p95 23,8, max 48,4 | médiane 10,8 ms, p95 15,7, max 21,4 |
| **Débit du pas scripté** (3,7 s) | 11 700 octets, 3,1 Ko/s | 11 557 octets, 3,1 Ko/s |
| **Redimensionnement** (100 → 60 → 100 colonnes) | redessiné les deux fois (320 et 271 octets) ; aucune écriture au-delà de la largeur | pareil |
| **Touches** | Échap → un `stop` ; `o` → un accord, `reply_to` vers la demande ; Ctrl-C → sortie en 150 ms, curseur rendu | pareil |
| **Couleurs des 5 thèmes**, en couleur vraie (`38;2;r;g;b`) et en 256 (`38;5;n`) | 10/10 : la couleur `texte` et le cœur du Cercle | 10/10 |

**Lecture.** La latence d'affichage tient à peu près à une image de la vue (33 ms : la vue redessine au plus 30 fois par seconde) plus un aller-retour. L'écho d'une frappe, lui, n'attend pas d'image pleine, parce que la touche réveille la boucle tout de suite.

## Ce que la mesure a trouvé : la vue paniquait sans position du curseur

Au départ, le harnais ne répondait pas à `ESC[6n`, et le terminal s'arrêtait en panique : `failed to initialize terminal … The cursor position could not be read within a normal duration` (ratatui 0.30, `init_with_options`). Un vrai terminal répond, mais un multiplexeur qui ne répond pas, ou un lien qui met plus de deux secondes, finirait pareil.

**Le correctif** (`361f08103`). La vue s'ouvre maintenant par `try_init_with_options`, avant tout envoi à l'hôte. Si elle ne démarre pas, le terminal le dit (« la vue ne démarre pas (…) : je passe en texte brut (--plain) ») et le texte brut prend la suite, sans doublon de demande.

**Le témoin**, la séance 0 du harnais : un pseudo-terminal qui ne répond jamais au curseur.

| Binaire | Résultat | Fichier |
|---|---|---|
| d'avant (`789ec0993`) | rouge : `panique: true`, aucun repli | `mesures/portable-vers-luciepc-rouge-avant-le-correctif.json` |
| d'après | vert dans les deux sens : `repli_dit: true`, `texte_brut_suit: true`, `panique: false` | `portable-vers-luciepc.json`, `luciepc-vers-portable.json` |

## Le plan de test pour Windows et macOS (à jouer quand une machine existe)

| Système | Terminaux | Ce qu'on vérifie |
|---|---|---|
| Windows 10 et 11 | Windows Terminal ; la console classique (conhost) ; PowerShell et cmd ; ssh d'OpenSSH pour Windows vers luciepc | le bâti (`cargo build --release --bin rag3weaver-tty`, sans le moteur) ; le mode brut et les touches (Échap, Ctrl-C, Entrée, retour arrière, `o` et `n`) ; le redimensionnement de la fenêtre ; les séquences VT (couleur vraie, 256) ; les glyphes ⛧, runes et blocs, avec le jeu sûr par défaut et `RAG3WEAVER_TTY_GLYPHES=ascii` ; le repli en texte brut quand la position du curseur ne revient pas ; `--plain` |
| macOS (Apple Silicon) | Terminal.app ; iTerm2 ; ssh vers luciepc | les mêmes points. Terminal.app n'a pas la couleur vraie : le terminal doit tomber en 256 couleurs par `TERM=xterm-256color` sans `COLORTERM`. |

**Comment.** On lance d'abord le même `mesure.py`, depuis la machine vers luciepc (ssh), puis le terminal localement contre un hôte joint par tunnel (`--daemon-cmd`, `--adresse`). Les chiffres attendus sont ceux du tableau, avec un ordre de grandeur de 50 ms pour l'affichage et de 10 à 20 ms pour l'écho. Ce qui ne se mesure pas par le harnais (les glyphes à l'œil, la police) passe par une capture d'écran par terminal, rangée à côté.

## Le push (ec4f7aa6d)

- **Repli moteur ancien, lot sans effet sur le moteur, lib e93710f36.** Le rejeu court sur la tête rebasée a tourné sous `RAG3WEAVER_MOTEUR_ANCIEN=1`, admis par c7 pour ce push seul. Le C++ du cœur (`0c41f9fe6`) était plus récent que la lib épinglée, et la lib neuve est arrivée pendant la passe.
- **La passe.**

  | Étape | Résultat |
  |---|---|
  | crate du terminal | 39/39 avec la vue, 10/10 sans |
  | `e2e_tty` | 4/4 |
  | `e2e_catalogue_gabarits` | 12/12, au second essai |

  Le premier essai du catalogue avait fait 5/12 : le service bge-m3 de luciepc (7879) ne répondait plus pendant la passe, et les 7 tests qui l'exigent sont tombés au démarrage. La suite a été rejouée seule sur la même tête quand 7879 est revenu. Aucun service n'a été touché.
- **Le rebase** s'est fait avec `git -c merge.directoryRenames=false`. Le ticket fermé par hotfixs avait vidé `docs/tty/tickets/`, et git voulait y voir un dossier renommé et ranger le ticket ouvert « Windows et macOS » parmi les fermés.
