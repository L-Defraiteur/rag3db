# Rapport de la nuit du 23 au 24 novembre 2026

Écrit par la session d'orchestration (c7) à 3 h 40, pour le réveil de Lucie. Il sera complété au matin.

## En une phrase

Un agent rag3weaver a travaillé seul, sans écran, sur une vraie tâche du projet ; son travail a montré un défaut
de notre politique de contexte, corrigé dans la nuit et mesuré sur le même cas.

## Retrouver la session du pair

```
cd ~/git_workspaces/lucie/23-11-2026-essai-agent && rag3weaver-tty --continue
```

Quatre consignes, quatre comptes rendus, de 0 h 35 à 0 h 51. Son rapport :
`docs/pair/24-11-2026/01-ce-qui-est-en-dur.md`, sur la branche `lucie/23-11-2026-essai-agent`, en local seulement.

## Ce que le pair a fait

La consigne : trouver, avec l'index, ce qui est écrit en dur dans la boucle de l'agent et dans le terminal, sans
lire l'inventaire d'everything declarative.

| Tour | Durée | Ce qu'il a fait | Noms absents du code, sur tout le rapport |
|---|---|---|---|
| 1 | 4 min | un rapport de 24 points | 8 |
| 2 | 4 min | une correction demandée par l'index, écrite de mémoire | 30 |
| 3 | 4 min | six points corrigés, un appel d'outil puis une recopie | 17 |
| 4 | 1 min 20 | les lignes d'origine reprises dans son historique git | 0 |

- Sans écran, aucune carte, une réponse en texte et un commit par tour.
- Noté contre l'inventaire d'everything declarative : environ 19 de ses 24 points recoupent les 52 de l'inventaire ;
  quatre points sont neufs et ont été versés à l'inventaire (la coupe des retours d'outils, le masquage des
  secrets, l'ouverture d'une tâche depuis le chat, le rendu des outils dans le terminal).
- À chaque tour, il affirmait que tout était vérifié, y compris quand tout était faux.

## Le défaut trouvé

Un pas de l'agent s'arrêtait à 64 000 jetons. Le pas suivant repartait d'un contexte neuf, sans les noms, les
chemins et les lignes rendus par l'index. Il écrivait alors de mémoire, inventait des noms plausibles, et a
réécrit un tableau entier dont dix-neuf lignes étaient justes. La preuve est dans la trace brute des appels,
gardée depuis le 23.

## Ce qui a été corrigé dans la nuit

| Correctif | État à 3 h 40 |
|---|---|
| Le plafond d'un pas suit la fenêtre du modèle (un quart, environ 262 000 jetons pour Gemini 3 Pro) | posé à la tablette à 3 h 03 |
| Les faits de l'index passent d'un pas au suivant | sur master, mais pas branché dans le manifeste livré ; le branchement est déposé |
| Les règles de fin de pas dans un graphe et une table, modifiables sans compilation | sur master |
| Les descriptions des outils d'index disent ce qu'un plan de fichier ne trouve pas | déposé |
| Le rappel automatique de mémoire avant un tour | l'outil est sur master ; son branchement sur la boucle est le lot suivant |

## La mesure, sur la pose de 3 h 03

La même demande, mot pour mot, dans deux dossiers neufs.

| | Tour 1, noms absents | Tour 2 (correction), noms absents | Lignes modifiées au tour 2 |
|---|---|---|---|
| Ancienne pose, cette nuit | 8 sur environ 32 | 30 | tout le tableau |
| Passe A, plafond de 64 000 forcé | 2 sur 34 | 0 sur 36 | 9 |
| Passe B, réglages par défaut | 0 sur 36 | 0 sur 41 | 8 |

Ce que la mesure ne prouve pas : l'effet du passage des faits d'un pas à l'autre, qui n'était pas branché. Elle
sera rejouée après sa pose, dans un dossier dont l'index est à jour.

Ce qui reste ouvert : sans qu'on le lui demande, l'agent ne lance aucune recherche dans l'index.

## Les autres atterrissages de la nuit

- Mémoire : « moi » désigne l'agent qui pose la question ; filtres par auteur sur les tours, les fiches et les
  apports.
- Moteur : un nœud de l'index vectoriel laissé sans lien après une suppression est rattaché ; la bibliothèque
  `fc64aca6f3` est épinglée et posée.
- Terminal : deux tours d'affilée testés contre un vrai hôte ; le thème lu chez l'hôte ; une consigne posée par un
  agent s'affiche au nom de cet agent ; à la reprise, le texte vient avant la ligne des fichiers.
- IDE : la page d'une tâche montre chaque appel d'outil à mesure ; une tâche qui travaille dans le dossier ne
  propose plus de fusion.
- Poste : un travail lourd n'entre que si la mémoire disponible le couvre ; le pousseur garde ensemble deux
  déclarations de tests ajoutées au même endroit ; chaque sortie de son second créneau dit sa raison.

## Les incidents

- 2 h 08 à 2 h 15 : le pousseur en pause, sept minutes. Son script neuf était installé avant le programme dont il
  dépend. Remis à la main, puis corrigé.
- 1 h 02 : un test de navigateur rouge sous charge (le focus à l'ouverture d'un dossier). Le test lisait trop tôt.
  La correction est en cours : le champ prend la frappe dès l'accueil, seul l'envoi attend l'agent.
- 3 h 05 : en mode sans écran, une conversation neuve affichait les lignes d'une autre conversation. La vue riche
  n'est pas touchée. Corrigé, en passe.

## Ce qui attend le mot de Lucie

1. **La mémoire d'un dossier neuf.** Le gabarit livré n'en déclare aucune, donc aucun rappel. Recommandation : une
   mémoire longue par utilisatrice, commune à ses dossiers.
2. **La migration du manifeste de rag3db en surcouche.** Trente réglages livrés manquent à sa copie. La commande
   montrera les différences avant d'écrire.
3. La boucle par ré-entrée sur événement et son plafond de 200.
4. La ligne `"imports"` du manifeste de rag3db, qui donne à l'agent les outils de la mémoire longue.
5. lucivy 4.5.1 ; le juge lourd ; Cloudflare ; l'essai d'index d'un dépôt de Sairen.
6. Des retraits : les six worktrees de recherche, le dossier d'essai du pair et les deux dossiers de mesure
   (`~/git_workspaces/lucie/24-11-2026-mesure-a` et `-b`), `root-grants.json.avant-23-11` à luciepc.
7. À luciepc : `systemctl --user restart wireplumber pipewire pipewire-pulse`.
