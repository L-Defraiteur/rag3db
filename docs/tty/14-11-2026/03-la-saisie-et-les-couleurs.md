# tty-8 : la saisie au clavier, et plus de rouge uni

Session tty, 14 novembre 2026, au mot de Lucie, qui a essayé le terminal : « très prototype, couleur rouge unie pas très sympa pour la vue, et surtout retour en arrière pas possible, navigation au clavier, insertions, pense à tous ces petits trucs du curseur ».

## La saisie, un éditeur de ligne

Avant ce lot, la saisie ne savait qu'ajouter un caractère à la fin et effacer le dernier. Elle devient un éditeur de ligne comme celui d'un shell (`extension/rag3weaver/tty/src/saisie.rs`) :

| Touche | Effet |
|---|---|
| ← → | un graphème : un accent combiné, un emoji composé ou un idéogramme compte pour un seul pas |
| Début, Fin, Ctrl-A, Ctrl-E | les deux bouts de la ligne du curseur |
| Ctrl-← Ctrl-→, Alt-← Alt-→, Alt-B Alt-F | un mot (lettres et chiffres ; un point ou une barre coupe un mot) |
| Retour arrière, Suppr | le graphème avant le curseur, et celui sous lui |
| Ctrl-W, Alt-Retour arrière | le mot d'avant, jusqu'à l'espace, sans passer le début de la ligne |
| Ctrl-U, Ctrl-K | jusqu'au début, et jusqu'à la fin, de la ligne |
| ↑ ↓ | la ligne d'au-dessus ou d'en dessous dans une saisie de plusieurs lignes, sinon l'historique des envois (sans doublon) ; la saisie en cours est gardée et revient |
| Alt-Entrée, Maj-Entrée | un saut de ligne, sans envoyer (Maj-Entrée là où le terminal la distingue) |
| un collage | il arrive d'un bloc (bracketed paste) : ses sauts de ligne restent dans la saisie, rien ne part avant Entrée |
| Ctrl-C | vide d'abord une saisie non vide, puis quitte |
| AltGr | son caractère : sous Windows, AltGr arrive comme Ctrl+Alt, et `@` ou `{` d'un clavier français étaient avalés comme une combinaison Ctrl, depuis tty-6 |

Ce qui ne change pas : avec une saisie vide, `o` et `n` accordent ou refusent la commande que l'équipier attend ; Échap et Ctrl-G l'arrêtent ; Ctrl-D quitte.

À l'écran, la saisie tient dans la largeur. Les caractères larges comptent pour deux colonnes, et un saut de ligne se montre `↵`. Une saisie trop longue défile autour du curseur, et chaque bout coupé se dit par `…`. Le curseur du terminal est posé là où est celui de la saisie.

## Les couleurs

Le rouge uni venait des lignes de la boîte : chaque réponse de l'équipier s'écrivait `[teammate · answer] …`, entière en rouge gras, dans la couleur d'accent du thème (`#ff3333` pour daemon). Désormais :

- **l'en-tête garde l'accent, en français** : `[équipier · réponse]`, `[toi · consigne]`, `[équipier · accord demandé]`, `[hôte · fini]`. Un genre ou un auteur inconnu s'écrit tel quel. `--plain` écrit les mêmes en-têtes ;
- **la prose prend la couleur du terminal**, qu'il s'agisse du corps d'une ligne de boîte ou du texte du modèle. Un thème peut en donner une par le champ facultatif `couleurs.prose`. Absent, il vaut la couleur du terminal : aucun des cinq thèmes ne le porte, et l'IDE, qui lit les mêmes fichiers, n'a rien à changer ;
- **les accents restent** : la ligne d'état, les animations, les outils, le cadre et la palette du code.

## Le Markdown courant

La prose se rend sans ses marques :
- un titre `#` à `######` s'écrit en gras, dans l'accent ;
- une puce `-`, `*` ou `+` devient `•` (`-` en ASCII) ;
- `**gras**` passe en gras et `*italique*` en italique ;
- le code en ligne reste teinté, avec ses accents graves.

Un astérisque ne compte que s'il entoure un mot : `src/*.rs` et `2 * 3 * 4` restent tels quels.

## Les témoins, rouges puis verts

Chaque morceau a eu son test rouge avant son code, et chacun a son commit sur la branche :

1. le curseur, les flèches, l'insertion, les graphèmes et l'affichage ;
2. les mots et les effacements ;
3. l'historique ;
4. la vue qui s'édite au clavier ;
5. la traduction des touches du terminal ;
6. AltGr ;
7. les couleurs, les en-têtes et le Markdown.

Les tests du crate passent de 45 à 58 avec la vue, et restent 14 sans elle.

Le harnais (`mesures/mesure.py`, séance 1d) tape les séquences d'un vrai terminal à travers le lanceur :
- la saisie : flèches, Début et Fin en séquences xterm, Ctrl-←, Suppr, retour arrière, Alt-Entrée, un collage entre `ESC[200~` et `ESC[201~`, Ctrl-W ;
- le rappel : ↑, Fin, « ! », Entrée.

La boîte du faux hôte doit recevoir les deux textes exacts, puis Ctrl-C doit vider une saisie avant de quitter.

| Passe | Binaire | Verdict |
|---|---|---|
| `mesures/clavier-ssh-rouge-avant-saisie.json` (ssh) | tty-7 | ROUGE : la boîte reçoit `lis vue.rssrc/L`, puis le collage coupé en deux envois, et Ctrl-C quitte |
| `mesures/clavier-ssh.json` | tty-8 | VERT |
| `mesures/clavier-tmux-ssh.json` (tmux à luciepc, vu par ssh) | tty-8 | VERT |
| `mesures/clavier-mosh.json` | tty-8 | VERT |

Sous les trois lanceurs, la boîte reçoit `Lis src/vue.Rs\npuis\nrésume tout`, puis le même texte suivi de « ! ». Les autres séances (le journal, les touches d'arrêt et d'accord, le redimensionnement, les couleurs des cinq thèmes) restent vertes.

## L'écho, départagé

Au premier vert sous ssh, l'écho d'une frappe valait 21,4 ms en médiane, contre 8,3 au vert de tty-7. Je l'ai rejoué sous ssh, binaire neuf, puis ancien, puis neuf (`mesures/clavier-echo-*.json`) :

| | Écho, médiane / p95 | Affichage, médiane |
|---|---|---|
| tty-8 | 10,7 / 24,2 ms | 47,6 ms |
| tty-7 | 13,1 / 15,5 ms | 47,8 ms |
| tty-8 | 14,8 / 35,3 ms | 48,2 ms |

L'éditeur ne coûte rien de visible. La médiane varie de 10 à 15 ms d'une passe à l'autre, quel que soit le binaire, et luciepc était chargé (charge de 6,5).
