# L'archiviste contre la compaction : la conception (15 novembre 2026)

Demande de Lucie, par l'orchestration (c7, 12 h 3x), copie à la session mémoire. Cette page dit ce
qu'on mesure et comment, avant tout appel et avant tout chiffre.

## 1. La question

Quand Claude Code compacte une conversation, il remplace tous les tours d'avant par un résumé. Ce
résumé est écrit par le modèle et rien ne le vérifie. L'expérience compare ce résumé à ce que la
mémoire longue saurait rendre au même point :
- l'archiviste range les tours d'avant : sections par tours, L1 par sujet, bilan ordonné ;
- un contexte est composé dans le budget de 100 000 jetons.

On juge les deux sur une vérité de terrain qu'on a : **ce que l'agent a eu besoin de savoir dans
les tours qui suivent**, dans le même transcript.

## 2. Les données

- **Les transcripts** : `~/.claude/projects/-home-lucied-git-workspaces-rag3db/*.jsonl`, un par
  session. Chacun porte :
  - ses frontières (`type` = `system`, `subtype` = `compact_boundary`, avec `compactMetadata` :
    `preTokens`, `postTokens`, les messages préservés) ;
  - le résumé que Claude Code y a produit (`isCompactSummary`).
- **La première conversation** : celle de l'orchestration, `99bc8451-…`. Elle compte 67 864 lignes et
  9 frontières. La première frontière est à la ligne 30, la conversation continue une session
  d'avant : il reste donc **8 frontières jugeables**. Les segments entre deux frontières font 3 à
  6 millions de caractères (environ 0,8 à 1,6 million de jetons) et 730 à 1 030 tours de
  l'utilisateur. Les résumés font 20 à 33 000 caractères.
- **Rien sur les originaux** : les transcripts sont lus en lecture seule et copiés dans un dossier
  de travail. Chaque frontière a sa propre copie de mémoire, neuve. Aucune écriture dans la mémoire
  vivante de Lucie.

## 3. Les quatre bras, à chaque frontière b

| bras | contexte d'après | budget |
|---|---|---|
| **CC** | le résumé de Claude Code à b, plus les messages qu'il a préservés | ce qu'il a pris (`postTokens`) |
| **ARCH** | l'archiviste a rangé les tours jusqu'à b sur une copie ; le contexte est composé à partir des L1 par sujet et du bilan ordonné | 100 000 jetons |
| **ARCH=** | le même contexte, coupé à la taille du bras CC | celui de CC |
| **CC+ARCH** | le résumé de CC, puis l'archiviste dans le reste du budget | 100 000 jetons |

- **ARCH=** sépare ce que l'archiviste sait de ce que le budget lui donne : à 100 000 jetons
  contre 20 000, un rappel plus haut ne prouverait rien seul.
- **La requête du rappel** : au moment de b, il n'y a pas encore de tour d'après. Le composeur
  rappelle donc sur les derniers tours d'avant b, la tâche en cours. Il ne voit jamais un tour
  d'après b.

## 4. La vérité de terrain : les besoins

On regarde une fenêtre W après b : les 60 premiers tours de l'utilisateur, au plus 300 000
caractères, sans franchir la frontière suivante. Un **besoin** y est un fait atomique qui remplit
trois conditions :
1. l'agent s'en sert ou le cherche dans W : une décision, une règle, un commit, un chemin, une
   valeur, l'état d'un lot, le nom d'une session ;
2. le fait est **établi avant b**, dans le segment d'avant ;
3. le fait n'est pas réintroduit dans W avant d'être utilisé. Un fait que l'utilisateur redit au
   tour 1 de W ne compte pas.

**L'extraction, sans voir aucun contexte** :
1. Gemini lit W et propose les faits que l'agent y tient pour connus ou qu'il redécouvre, chacun
   avec la phrase de W qui le montre ;
2. chaque fait est vérifié contre le segment d'avant. On repère les passages par ses identifiants
   (hash, chemin, nom), puis Gemini confirme sur ces passages. Un fait sans appui avant b est
   écarté.

**La part objective, en parallèle** : les appels d'outils de W qui relisent ce qui était déjà su
avant b. Ce sont les redécouvertes : un `Read` d'un fichier déjà lu, un `git log` d'un commit déjà
nommé, une question à une session dont la réponse était déjà là.

## 5. Les métriques

Pour chaque besoin et chaque bras, Gemini juge le contexte : **contient**, **en partie**,
**absent**, ou **contredit** (le contexte dit autre chose que la vérité de W). Les bras sont
anonymes (A, B, C, D) et leur ordre est tiré au sort à chaque frontière. Les besoins passent par
lots, un appel par bras et par lot.

- **M1, le rappel des besoins** : contient = 1, en partie = 0,5. Par frontière, puis sur toutes.
  La comparaison CC contre ARCH se fait besoin par besoin, appariée : gagnés, perdus, égaux, et un
  test du signe.
- **M2, les contradictions** : le nombre de « contredit » par bras. C'est le risque propre d'une
  mémoire : une fiche périmée qui dit faux avec assurance.
- **M3, le coût** : les jetons du contexte, et M1 à budget égal (ARCH= contre CC).
- **M4, les redécouvertes évitables** : la part des redécouvertes de W dont l'information était
  dans le contexte du bras.

**La règle de lecture** : ARCH bat CC s'il gagne en M1 à budget égal (ARCH=) sans perdre en M2.
ARCH à 100 000 jetons seul ne suffit pas.

## 6. La validation du juge

- **L'extraction** : je relis 30 besoins tirés au sort. Est-ce un vrai besoin ? Est-il établi
  avant b ? Je donne la précision. Je relis aussi 2 fenêtres entières pour estimer les besoins
  manqués.
- **La couverture** : je relis 30 jugements tirés au sort, et 20 jugements sont rejoués deux fois
  pour la stabilité. Je donne l'accord.
- **Sous 0,8 d'accord**, les chiffres ne sont pas publiés, et l'invite du juge est revue.

## 7. Le matériel et les bornes

- **Gemini sur Vertex**, au projet de Lucie, avec les identifiants de `.vault` passés par leur
  chemin, jamais lus. Les appels sont en série, avec une attente doublée à partir de 60 s sur un
  quota. À peu près 10 à 15 appels par frontière, soit 100 à 150 appels pour la première
  conversation.
- **Les secrets** : le texte envoyé est d'abord filtré (motifs de clés, de jetons et de chemins de
  `.vault`). Un passage qui matche est remplacé par « [masqué] » et compté.
- **L'archiviste et le composeur** sont ceux de la session mémoire, sur copie, sur le CPU ou par le
  service d'embarquement. Ce qui reste à recevoir d'elle :
  - le point d'entrée qui range une suite de tours en sections ;
  - le composeur au budget de 100 000 jetons.
- **Rien n'est lancé** avant que l'orchestration ait validé la métrique.
