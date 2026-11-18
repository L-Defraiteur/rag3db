# Mesurer la variante R de l'archiviste

- **État** : ouvert ; attend la forme en deux étapes de l'archiviste (page de la session mémoire, 18 novembre au soir).
- **Famille** : produit
- **Gravité** : moyenne. C'est le choix de la forme de l'archiviste pour les sessions longues.
- **Atteignable en service** : oui. L'archiviste range les sessions longues.
- **Touche rag3weaver** : non. Une mesure.
- **Pour** : banc.
- **Signalé par** : c7, 18 novembre 2026.
- **Ouvert le** : 18 novembre 2026

## La variante

**R** : des fenêtres de 50 k sans chevauchement. Chaque fenêtre reçoit le **rapport structuré** que l'archiviste a rendu
à la fin de la précédente, à la place des résumés de S :
- les sujets et leur rangement ;
- les catégories proposées ;
- les sujets ouverts ;
- ce qui n'est pas rangé.

## La mesure

- Sur le même corpus que la mesure longue (`c7-99bc8451-f5`, 244 tours), avec la forme en deux étapes voulue par
  Lucie : d'abord l'extraction structurée des sujets de la fenêtre, sans la base, puis le rangement de chaque sujet.
- Contre S 50 (la forme de fenêtre retenue) et contre la N rejouée (`long/ref-N-0653-20261116-0305`).
- Avec le juge de la page du 15 (§ 4 et § 10) : P1, P2, les « rien » qui endorment, s par décision, jetons et part du
  cache.
- Le témoin d'abord : N contre N, P1 = 1.
- Le script s'arrête sur une erreur du modèle (`x_archive_long.py`, depuis le 18 novembre).

## Une boîte à outils, pas des variantes (c7, 18 novembre, au mot de Lucie)

Le contexte de l'archiviste se compose d'**ingrédients déclarés et débrayables**, chacun avec sa borne :
- le rapport de la tranche d'avant ;
- le résumé de la tranche d'avant, par l'extraction ;
- le résumé courant ;
- les résumés de S ;
- le chevauchement de F, en % ;
- le bilan ;
- les candidates.

F, S, R et leurs mélanges deviennent des **réglages** joués sur le même corpus, pas des variantes codées à part.

**La matrice à mesurer d'abord** (S 50 comme forme de fenêtre, la forme en deux étapes) :

| réglage | rapport de la tranche d'avant | résumé de la tranche d'avant |
|---|---|---|
| S 50 nu | non | non |
| S 50 + rapport (R) | oui | non |
| S 50 + résumé de tranche | non | oui |
| S 50 + les deux | oui | oui |

Chaque réglage est jugé contre la N rejouée, en **P1** (rappel des sujets), **P2** (doublons) et **P5** (coût : temps de
passe, appels, jetons d'entrée et part du cache), avec les définitions du § 4 de la page du 15. Les autres
ingrédients restent à leur réglage par défaut et sont écrits avec chaque passe. La matrice se joue après la page de
la session mémoire qui pose la forme.

## Le contexte d'après compaction, réglable aussi (c7, 18 novembre)

Le contexte qu'une session reçoit après une compaction se compose lui aussi d'ingrédients. Trois profils sont à juger
**contre la compaction de Claude Code**, sur des transcripts réels, dans la mesure prévue à la page
`docs/banc/15-11-2026/01-l-archiviste-contre-la-compaction.md` :
- les L1 seuls, dans l'ordre des tours ;
- le résumé global des tranches seul ;
- les deux.

**La vérité de terrain** est ce que les tours suivants ont eu besoin de savoir. Chaque profil est jugé sur la part de ce
besoin qu'il couvre, et sur son coût en jetons. Cela se fait après la page de la session mémoire.

## Pour fermer

Les quatre réglages de la matrice joués dans la forme en deux étapes, jugés contre N en P1, P2 et P5 ; les trois profils d'après compaction jugés contre la compaction de Claude Code ; les deux résultats écrits dans leurs pages.
