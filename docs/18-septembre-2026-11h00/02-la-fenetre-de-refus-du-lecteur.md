# La fenêtre de refus du lecteur : mesurée, et ce qu'elle dit du budget de reprise

**18 septembre 2026, soir.** Un transitoire côté Rust :
`un_lecteur_qui_insiste_pendant_qu_on_ecrit` a rendu « refus sporadique : 2/80 »
une fois dans la journée, 0/80 à toutes les autres passes. Deux ouvertures en
lecture seule ont vu leur refus survivre au budget de reprise de `read_only`
(250 ms, tentatives à 5, 10, 20, 40, 40… ms, soit environ huit).

## Ce qu'est le refus, exactement

Le lecteur en lecture seule ne refuse que dans un état précis : quand le dernier
enregistrement du journal est un CHECKPOINT (`wal_replayer.cpp:113-115`, puis
`shadow_file.cpp:93`). Cet état s'ouvre à `wal->logAndFlushCheckpoint` et se
ferme à `wal->clear()` (`checkpointer.cpp:158-162`). Entre les deux,
`applyShadowPages` réécrit les pages fantômes dans le fichier de données et le
synchronise sur disque (`shadow_file.cpp:79`). La fenêtre est **par construction
non déplaçable** : le CHECKPOINT est journalisé avant l'application, pour qu'une
panne au milieu se rejoue depuis le fichier fantôme.

## La mesure

Le test C++ `LeRefusSeResoutParUneNouvelleTentative` chronomètre désormais
chaque épisode, du premier refus au premier succès, à la cadence de l'appelant
Rust mais sans plafond de budget (`945cd6706`). Sur ce poste, 24 cœurs,
base minuscule (400 lignes, point de reprise toutes les cinq écritures) :

| condition | passes | fenêtre max | reprises max / épisode |
|---|---:|---:|---:|
| au repos | 3 | 17,7 – 18,5 ms | 2 |
| écriture disque directe + fsync en boucle | 3 | 37,8 – 38,5 ms | 3 |
| 24 boucles processeur occupées | 13 | 18,5 – 41,7 ms **sauf une : 567 ms** | 2 – 3, **sauf une : 16** |
| 48 boucles occupées | 3 | 19,4 – 22,4 ms | 2 |
| 48 boucles + écriture disque | 3 | 21,9 – 43,7 ms | 3 |

**Le phénomène est reproduit : une fois sur vingt-deux passes chargées, la
fenêtre est passée à 567 ms — plus du double du budget — avec seize reprises.**
Chez le pair : une fois dans la journée, 2 sur 80. Même forme, même rareté.

## Ce que ça veut dire, et ce que ça ne veut pas dire

- **Ce n'est pas un défaut du moteur ni du report de Vela.** La fenêtre est
  légitime et courte ; elle s'étire quand le processus écrivain ne progresse
  pas *à l'intérieur* de la fenêtre. La contention processeur en est un
  mécanisme plausible — l'échantillon à 567 ms est tombé sous charge
  processeur, et un `cargo check` tournait chez le pair — mais **un seul
  échantillon ne suffit pas à épingler la cause**. Ce document dit « reproduit
  », pas « expliqué ».
- **Le budget de 250 ms est une hypothèse sur l'ordonnanceur, pas sur le
  moteur.** Il tient cent fois sur cent au repos et dans la quasi-totalité des
  cas chargés ; il lâche quand le poste est saturé.
- **La fenêtre ne peut pas être raccourcie côté C++** sans toucher au contrat
  de reprise après panne. Ce qui peut bouger est côté appelant.

## Ce que ça suggère, sans le décider

Deux formes, à trancher avec Lucie :

1. **Élargir le budget par défaut de `read_only`** — une seconde, par exemple.
   Le coût ne se paie que dans une vraie panne, qui échoue alors en une seconde
   au lieu d'un quart, et l'erreur porte toujours le compte de tentatives.
2. **Faire porter au test l'invariant, pas le chronomètre.** « Ou refusé, ou il
   lit juste, jamais du bruit » ne dépend pas de la machine ; « aucun refus en
   250 ms » en dépend. Le test peut passer par `read_only_patient` avec un large
   budget et garder son assertion tranchante — un refus qui survivrait à *une
   seconde* serait, lui, un vrai défaut.

Les deux ne s'excluent pas.
