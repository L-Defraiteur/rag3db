# Le terminal sous tmux et sous mosh (tty-6)

14 novembre 2026, session tty. Lucie a installé tmux 3.8 et mosh 1.4.0 sur les deux postes (paquets `tmux 3.8-1.1` et `mosh 1.4.0-36.1`), après une mise à jour du système sans redémarrage.

**Sa règle** : « les gens n'installeront rien en plus de rag3weaver ». Le terminal n'exige ni tmux, ni mosh, ni un terminal particulier. Ces mesures vérifient seulement qu'il se tient dans ce que les gens utilisent déjà, et que chaque dégradation devient un repli dit.

## Comment

Le harnais de tty-5 (`extension/rag3weaver/tty/mesures/mesure.py`) gagne un lanceur, en 7e argument :

| Lanceur | Ce qu'il lance |
|---|---|
| `tmux-local` | tmux sur ce poste, l'hôte en `local` |
| `tmux-ssh` | tmux sur l'autre poste, vu par ssh |
| `mosh` | mosh-server sur l'autre poste |

Sous tmux et mosh, c'est le multiplexeur qui répond à la position du curseur et qui rend les couleurs. Le harnais note donc ce qui arrive (la vue inline démarre-t-elle, la couleur vraie passe-t-elle ou tombe-t-elle en 256) au lieu de l'exiger. Il arrête le serveur tmux de chaque séance et efface sa socket. Le binaire est `rag3weaver-tty` en release, du commit `e59817bb7`. Les résultats bruts sont dans `docs/tty/14-11-2026/mesures/`.

## Les chiffres

| | tmux ici | tmux à luciepc, vu par ssh | mosh portable → luciepc | mosh luciepc → portable | ssh seul (rappel) |
|---|---|---|---|---|---|
| Affichage, médiane / p95 | 14,4 / 15,4 ms | 48,1 / 147,3 ms | 47,1 / 50,0 ms | 47,5 / 64,4 ms | 47,5 / 52,8 ms |
| Écho d'une frappe, médiane | 0,3 ms | 15,3 ms | 16,7 ms | 16,9 ms | 9,8 ms |
| Échap → `stop` | **501 ms** | **522 ms** | 219 ms | 7 ms | 8 ms |
| Ctrl-G → `stop` | 1 ms | 42 ms | 114 ms | 2 ms | 21 ms |
| `o` → l'accord, Ctrl-C → la sortie, curseur rendu | oui | oui | oui | oui | oui |
| Redimensionnement 100 → 60 → 100 | redessiné | redessiné | redessiné | redessiné | redessiné |
| Couleurs des 5 thèmes (couleur vraie et 256) | 10/10, la vraie passe | 10/10, **la vraie tombe en 256** | 10/10, la vraie passe | 10/10, la vraie passe | 10/10 |
| Position du curseur sans réponse du harnais | tmux répond, la vue démarre | tmux répond | mosh répond | mosh répond | repli en `--plain` (tty-5) |

## Les dégradations, chacune en repli dit

| Dégradation | Le repli |
|---|---|
| **Échap attend environ 500 ms sous tmux** : c'est l'`escape-time` par défaut de tmux, qui attend la suite d'une séquence avant de rendre un Échap seul. | **Ctrl-G arrête sans ce délai** (`e59817bb7`), partout, de 1 à 114 ms. Échap reste, puisqu'il marche, juste plus lentement sous tmux, et rien n'est à configurer. Une autre combinaison Ctrl + lettre n'entre plus dans la saisie : l'ancien binaire tapait « g », et le `o` suivant ne valait plus accord. Le rouge d'avant le correctif est dans `mesures/tmux-local-rouge-avant-ctrl-g.json`. |
| **La couleur vraie tombe en 256 sous tmux vu par ssh** : la configuration par défaut de tmux ne sait pas que le terminal extérieur a la couleur vraie. | C'est tmux qui convertit, et la couleur la plus proche est juste pour les cinq thèmes (10/10). Aucun réglage n'est demandé. |
| **mosh ne garde aucun historique au-delà de l'écran** : les lignes que la vue envoie dans l'historique du terminal sortent de l'écran et ne se relisent pas par le défilement. C'est une propriété de mosh, qui ne synchronise que l'écran, et que le harnais ne peut pas observer octet par octet. | Non résolu : ticket `docs/tty/tickets/2026-11-14-sous-mosh-l-historique-du-terminal-se-perd.md`, avec la piste d'un journal de la séance à relire. |

La latence de tmux vu par ssh a une queue plus longue (147 ms au p95, contre 52 pour ssh seul) : tmux rend l'écran à son rythme, puis ssh le porte. La médiane reste à 48 ms.

## Le push par la pile

Ce lot est le premier poussé en mode pile (`docs/orchestration/14-11-2026/03-la-file-en-pile.md`). Le rejeu a été joué sur la tête du lot d'avant, `cd6d02907` (lucivy) : crate du terminal 39/39 avec la vue et 10/10 sans, `e2e_tty` 4/4, `e2e_catalogue_gabarits` 12/12. Le dernier pas a été la garde du poste, appelée à luciepc avec `PILE_AVANT=cd6d02907`. Elle a dit « pile tenue », a reposé les cinq commits sur master sans rien rejouer, puisque master n'avait reçu que des docs, et a poussé `ce4a3140a`.

**Pourquoi un `temoins.log` dans le dossier des journaux de cette passe.** La garde cherche la lib liée de la passe dans des journaux aux noms fixes, ceux d'une passe entière (`temoins.log` et les autres). Une passe courte comme celle-ci n'en écrit aucun : le témoin du lot, `e2e_tty`, écrit `e2e_tty.log`. J'ai copié `e2e_tty.log` sous le nom de `temoins.log` avant l'appel, ce qui ne trompe pas la garde : la passe ne nomme qu'une seule lib (`0c41f9fe6`), la même dans ses seize mentions, et c'est celle que la garde a imprimée. Ce fichier n'est donc pas la sortie d'une suite `temoins`. L'écart, avec celui de la lecture d'un fichier `base` qui ne sert pas en mode pile, est chez G pour garde-pile.
