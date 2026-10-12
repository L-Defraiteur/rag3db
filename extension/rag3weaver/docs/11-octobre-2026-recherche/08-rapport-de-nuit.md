# Session recherche — la nuit du 11 au 12 octobre

*Pour Lucie et pour l'orchestration. Tout est sur master ; rien n'est en vol.*

## Le lot 0 des agents qui se parlent : clos

Après le rapport du jour (page 06) :
- le pont vivant d'un sous-graphe (3b, `0cacde894`) : sous l'exécuteur
  concurrent, un sous-graphe s'exécute sur les canaux de son parent ;
- la batterie de clôture : 596 passés, un seul rouge, d'environnement (le
  banc des fiches de la mémoire, sans ses services ; depuis, `run_e2e.sh`
  l'écarte en le nommant, par la mémoire).

## Le magasin de libs par identifiant de bâti (page 07)

L'incident de 20 h 39 en est l'origine : une lib rebâtie sans
`fileLockWaitMs` a cassé le bâti Rust de quiconque avait rebasé sur B2, et
l'hôte de luciepc a bouclé sur un symbole manquant.

| lot | commit | ce qu'il fait |
|---|---|---|
| 1 | `b8facf434` | le pont cxx se recompile quand la lib ou l'en-tête change (`rerun-if`) |
| 2 | `f5fdc7e9a`, `f3d2290ad` | la lib porte son identifiant (section ELF, relue par le cœur), le binaire le sien ; `rag3db-check-lib` les compare **avant le chargeur** (nos binaires sont en BIND_NOW), et le filet dans le processus ; le ticket du cœur est fermé |
| 3a | `7927672f8` | `scripts/rag3db_libs.py` : un dossier immuable par identifiant, `courant`, et le ménage qui épargne ce qui est mappé et ce que vise un RUNPATH ; l'extension est cherchée à côté de la lib |
| 3b | `f846db8e6` | `run_e2e.sh` prend sa lib au magasin avec `RAG3DB_LIB=courant`, résolu une fois au départ |
| 3c | `5e08ca07b` | les 97 sites de 82 suites cherchent l'extension par un appel commun |

**Prouvé de bout en bout** sur la lib de 02 h 34 : publiée au magasin ; un
backend bâti contre son dossier ; `rag3db-check-lib` dit « même bâti », puis
refuse en nommant les deux identifiants ; une passe réelle sous
`RAG3DB_LIB=courant`, 82 passés.

**Décidé avec G** : le magasin ne sert qu'aux postes Linux de
développement, car le paquet npm lie le moteur en statique. Un binaire
installé et son unité suivent un identifiant fixe, jamais `courant`.

**Reste** :
- chez G : sa recette de luciepc sur le magasin, et l'`ExecStartPre` ;
- chez l'orchestration : un RPATH sur les bâtis de ce poste, et le mode
  magasin par défaut dans `run_e2e.sh`, qui sont deux changements de
  conception.

## Les relectures

Pour le proto : `Route` et `Decide` en flux (un lecteur qui lâche n'est pas
une faute), l'équité du verrou du catalogue (les lecteurs sont comptés par
catalogue), la montre et le retard dit (W1 à W3).

## Les leçons

- **Un contrôle dans le processus ne passe pas avant le chargeur** quand
  le binaire est lié en BIND_NOW. La forme proposée par le ticket a donc été
  remplacée par une section lue de l'extérieur.
- **L'index des tickets conflicte à chaque fermeture voisine.** Mon script de
  push le résout désormais seul : il prend l'index de master, y refait la
  fermeture et passe `check_tickets` avant de continuer. Tout autre conflit
  l'arrête toujours.
- **Un push dans la file qui chargera la lib** se retire proprement, par
  pid, quand un rebâti est annoncé.
