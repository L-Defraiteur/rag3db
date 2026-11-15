# tty-13 : les commandes `/` et `!`, Tab, et l'historique qui reste

Session tty, 15 novembre 2026, au mot de Lucie : « je veux pouvoir lancer une commande shell dont le retour doit être visible de l'agent, en préfixant par ! … et si prompt de mot de passe dans la même commande pouvoir l'entrer … je veux pouvoir aussi, pour les commandes / builtin autant que pour les commandes !, avoir l'autocomplétion sur Tab, je veux pouvoir parcourir l'historique comme dans un terminal avec haut et bas ». Elle a choisi :
- une commande `!` tourne là où est le terminal ;
- sa sortie part en consigne, précédée de « l'utilisateur a lancé telle commande… voici le retour » ;
- les commandes `/` qui ne demandent rien aux autres sessions viennent d'abord.

## Les commandes `/`

Une saisie qui commence par `/` est une commande du terminal, jamais un envoi à l'équipier ; `//` en tête envoie le texte avec un seul `/`.

| Commande | Effet |
|---|---|
| `/aide` | les commandes et les touches |
| `/quitter` | quitte |
| `/arreter` | arrête l'équipier, comme Ctrl-G |
| `/theme <nom>` | change de thème en cours de séance ; sans nom, la liste |
| `/tache <nom>`, `/depot <nom>` | la vue repart sur une autre tâche ou un autre dépôt, avec le journal et le thème de la séance, sans rien demander à l'équipier |
| `/journal` | où relire la séance, ou comment en ouvrir une |
| `/effacer` | efface l'écran visible ; le défilement du terminal et le journal restent |
| `/effacer historique` | supprime l'historique des envois |
| `/context` | le remplissage du contexte du modèle de l'équipier : jetons utilisés sur la fenêtre, en pour cent, et ce qui pèse, par `GET /ide/chat?backend=<backend de la tâche>` (son `context`, qui existe déjà chez l'hôte) |

`/compact`, `/model` et `/fork-agent` attendent l'hôte : la compaction par l'archiviste (96 et archi), les modèles déclarés dans le manifeste (ED et archi), le clone avec son worktree et sa branche (archi). C'est le partage tranché par c7 avec Lucie.

## Tab

Tab complète, jusqu'au plus long préfixe commun, et les candidats se disent en sourdine sur la ligne du haut jusqu'à la touche suivante. Il complète :
- le nom d'une commande `/` ;
- son argument : les thèmes, les tâches et les dépôts lus chez l'hôte, et `historique` pour `/effacer` ;
- pour `!`, le premier mot parmi les exécutables du `PATH`, les suivants parmi les fichiers du dossier où tournera la commande (`~/` et les chemins absolus compris, un dossier finissant par `/`).

## Les commandes `!`

`!<commande>` la lance par `/bin/sh -c` dans le worktree de la tâche s'il est sur cette machine (un ancêtre du manifeste de la tâche qui porte `arbre/`), sinon dans le dossier courant.
- **L'écran lui passe**, dans un pseudo-terminal à elle (`posix_openpt`, par la libc que la bibliothèque standard lie déjà : aucune dépendance de plus). Elle a sa propre session, dont ce pseudo-terminal est le terminal de contrôle : c'est lui que `sudo` ouvre pour lire le mot de passe. Les touches lui sont relayées telles quelles, et le collage en bloc et le protocole kitty sont rendus le temps qu'elle tourne. Sa sortie s'affiche, et une copie de cette sortie seule se garde.
- **Puis la zone vivante revient**, et la sortie part en consigne : « L'utilisateur a lancé la commande `…` dans <dossier> ; voici le retour (code N) : » suivi d'un bloc. La sortie y est sans séquences du terminal (couleurs, curseur, titres) ni retours chariot, et coupée au milieu au-delà de 8 Ko. Revenue par la boîte, elle ne s'affiche pas une seconde fois : sa première ligne seule.
- **Sous Windows**, la commande refuse et le dit : le pseudo-terminal est Unix. Le ticket Windows et macOS couvre le reste.

### Les deux gardes de c7

1. **Une commande `!` ne vient que de la frappe.** Elle n'existe qu'à la sortie de la saisie, validée par Entrée. Une ligne de l'équipier ou de l'hôte qui commence par `!` s'affiche, sans jamais être lancée. Un texte collé reste dans la saisie jusqu'à Entrée.
2. **Rien de ce que lit `sudo` ne sort.** Seul ce qu'écrit la commande se garde, jamais ce qui se tape. `sudo` coupe l'écho pendant qu'il lit le mot de passe, qui n'entre donc ni dans la sortie gardée, ni dans la consigne, ni dans le journal de séance (seules les lignes de l'historique y vont, pas l'écran de la commande). Une limite, celle de tout terminal : une frappe tapée d'avance, avant que la commande ait coupé l'écho, est renvoyée à l'écran par le pseudo-terminal. Il faut donc taper un mot de passe après son invite.

## L'historique qui reste

Comme celui d'un shell, l'historique des envois survit à la séance. Il vit dans `<dossier d'état>/historique.txt`, dans `~/.local/state/rag3weaver/tty/` par défaut :
- un envoi par ligne, en chaîne JSON, de sorte qu'un envoi sur plusieurs lignes tient sur une seule ;
- les mille derniers, sans doublon de suite ni envoi vide, et sans les `o` et `n` d'un accord ;
- lisible par soi seul sous Unix (`600`).

↑ y retrouve les envois des séances d'avant, et `/effacer historique` le supprime.

## Les témoins

Les tests du crate :
- `historique` : la survie, plusieurs lignes, la borne, une ligne illisible sautée ;
- `commandes` : la lecture de `/`, `//` et `!`, et la complétion ;
- `coquille` : une vraie commande dans son dossier avec son code, un faux `sudo` dont le secret ne paraît pas, le nettoyage de la sortie ;
- la vue : Entrée sur une commande, Tab, la consigne revenue en une ligne ;
- le terminal : la touche Tab, `/context`, le dossier d'une commande.

Le crate passe de 82 à 93 tests avec la vue, et de 14 à 19 sans elle.

Ce qui a été vu rouge, et comment :
- l'historique, les commandes et la complétion, la vue, la coquille : rouges avant leur code ;
- `/context`, le dossier d'une commande, la touche Tab et la consigne en une ligne : leur code a été écrit avant leurs tests. Ces tests ont donc été vus rouges contre un code faussé exprès (une fonction rendue fausse le temps d'une passe), puis verts contre le vrai.
- Le premier rouge du faux `sudo` venait du test lui-même, qui tapait le mot de passe avant l'invite. Le test tape maintenant après elle, comme une utilisatrice, et c'est ce qui a fait écrire la limite de la frappe d'avance.

Le harnais (`mesures/mesure.py`, séance 1g ; le faux hôte gagne `POST /faux/ligne`, une ligne de boîte venue d'ailleurs) :
- une ligne d'équipier `!touch …/jamais-1` ;
- un `!touch …/jamais-2` collé puis effacé par Ctrl-C ;
- `!sh faux_sudo.sh`, avec un mot de passe tapé après l'invite.

Il vérifie l'invite et le retour à l'écran, une seule consigne qui porte le retour, l'absence du secret dans la consigne, le journal de séance et l'écran, et qu'aucun des deux `!touch` ne s'est lancé ni n'est parti en texte.

| Passe | Binaire | Verdict |
|---|---|---|
| `mesures/bang-ssh-rouge-avant.json` (ssh) | tty-11 | ROUGE : le `!sh` part comme du texte à l'équipier, sans consigne, et le mot de passe tapé reste à l'écran et dans le journal |
| `mesures/bang-ssh.json` | tty-13 | VERT |
