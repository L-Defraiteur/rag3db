# tty-20 : `--plain --demande` reste attaché jusqu'à la fin de sa demande (T1)

Session tty, 16 novembre 2026. Défaut T1 de la campagne D2–D6 (`docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`).

## Le défaut

Lancé en mode script, `rag3weaver-tty --plain --depot … --tache … --demande … --journal …` rendait la main en 3 s avec le code 0. Il disait « demande posée (l'équipier part) », et son journal ne portait que la demande, sans aucune ligne d'outil.

La cause : la relecture de la boîte, toutes les 2 s, repartait de la première ligne de la boîte. Elle y trouvait la fin (`ended`) d'une demande précédente de la même tâche, la prenait pour la sienne et arrêtait tout. Une tâche neuve, sans `ended` ancien, n'avait pas le défaut ; c'est pourquoi on le voyait seulement selon l'état de la tâche.

## Le correctif

- Avant de poser la demande, le terminal lit la boîte et part de sa longueur. Seules les lignes arrivées après comptent, pour l'écrire comme pour s'arrêter.
- Le flux est filtré sur le manifeste de la tâche (`backend` de `GET /ide/teammate`), comme dans le terminal riche ; une trame d'une autre tâche de l'hôte n'entre plus dans le journal. Si la tâche naît de la demande, le manifeste se lit dès qu'il existe.
- Effet de bord : sans `--demande`, le mode texte n'écrit plus l'historique ancien de la boîte, seulement ce qui arrive. Il s'arrête à la prochaine fin.

## Le témoin

`extension/rag3weaver/tty/mesures/plain_stays_attached.py`, sur le faux hôte :
1. il pose la fin d'une demande précédente ;
2. il lance `--plain --demande go --journal …` ;
3. il exige que le terminal soit encore attaché 8 s plus tard, et que le journal porte `» read_file` et `» edit_file` ;
4. il pose la fin de cette demande-ci et exige que le terminal rende la main dans les 10 s, avec le code 0.

Vu rouge sur le binaire de master (b76c1842b) : « il a rendu la main avant la fin de sa demande (code 0, au bout de moins de 8 s) », et aucun des deux outils dans le journal. Puis vert.

    python3 extension/rag3weaver/tty/mesures/plain_stays_attached.py <binaire> extension/rag3weaver/tty/mesures/faux_hote.py <dossier>
