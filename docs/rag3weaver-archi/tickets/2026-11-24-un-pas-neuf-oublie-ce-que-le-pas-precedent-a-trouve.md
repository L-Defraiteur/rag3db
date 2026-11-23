# Un pas neuf oublie ce que le pas précédent a trouvé, et le modèle invente

- **État** : ouvert, le 24 novembre 2026
- **Famille** : produit
- **Gravité** : haute : un rapport écrit dans un pas neuf cite des symboles inventés, avec de faux numéros de ligne,
  en affirmant qu'ils sont attestés par l'index, alors que le pas d'avant avait trouvé les vrais
- **Atteignable en service** : oui (toute tâche dont un pas atteint `teammate.step_context_tokens`)
- **Touche rag3weaver** : oui (le bloc « Previous step » : `teammate.carry_over_chars`, `teammate.carry_over_node` ;
  `teammate.step_context_tokens`)
- **Pour** : archi
- **Signalé par** : c7, le pair de la nuit du 24 novembre (tâche `46b303be5b04/24-11-00h35`)

## Ce qui s'est passé

Le tour 2 du pair devait corriger les symboles inventés de son rapport en se servant de l'index. Il l'a fait en deux
pas :

| pas | run | outils | jetons du 1er et du dernier appel | `teammate_loop` dans la trace | noms inventés dans la trace |
|---|---|---|---|---|---|
| recherche | `agent-1a20ac80a50-213` | 32 (9 `search_code`, 7 `usages`, `callees`, `outline`, `read_file`), puis `step_done` | 13 156 → 64 369 | 48 | 0 |
| écriture | `agent-1a20acab16d-260` | `read_file`, `edit_file`, `read_file`, `step_done` | 13 223 → 23 866 | 0 | `execute_step` 9, `advance_task` 9 |

Le pas de recherche a atteint le plafond de contexte d'un pas (64 000 jetons) et a dû clore. Le pas d'écriture est
reparti d'un contexte neuf, avec la demande et le bloc « Previous step », qui ne garde que de la prose bornée : aucun
des noms, chemins et lignes que l'index avait rendus. Le modèle a écrit de mémoire, et inventé 13 noms plausibles.

Le tour 1 avait le même défaut : deux refus « the context of the step is full », puis un rapport avec 8 noms inventés
sur 32.

## La confirmation, et un second effet plus grave

- **Par l'autre bout** (tour 3) : avec la méthode « un appel, puis un `edit_file` qui recopie le résultat, point par
  point » (21 `grep_files`, 9 `edit_file`, aucun refus, 4 minutes), les six points sont justes, vérifiés par
  `git grep`. Avec les faits sous les yeux, le modèle n'invente pas.
- **Un pas sans les faits abîme ce qui était juste** (tour 2) : son `edit_file` a réécrit tout le tableau du rapport.
  Les lignes 6 à 24, justes au tour 1 (commit `5918c2d67b`), portent depuis `ad79437aab` 17 noms qui n'existent pas,
  et le fichier est passé de lignes détaillées à des lignes abrégées.

**La mesure compte donc tout le fichier** après chaque tour, et pas seulement les lignes demandées : les noms absents
du code (`git grep`) sur l'ensemble du rapport (8, puis 13 + 17, puis 17 ; la cible est 0).

**Une garde de plus, à mesurer** : un `edit_file` qui remplace beaucoup plus de lignes que la demande n'en vise est
refusé une fois, avec « tu réécris N lignes ; ne change que celles que la demande vise, ou dis pourquoi ». C'est une
condition sur l'appel d'outil (ses arguments et le fichier), pas sur le `step_done` : elle se branche sur la porte
déclarée, comme les classifieurs de l'accord explicite.

## Ce qu'on attend

Les faits trouvés par les outils survivent au pas qui les a trouvés. Trois leviers, déclaratifs, à mesurer sur ce cas :

1. **Le bloc « Previous step » garde les faits** : un gabarit livré pour `teammate.carry_over_node` qui conserve, des
   retours d'outils d'index du pas (`search_code`, `usages`, `callees`, `outline`), les lignes « nom — chemin:ligne »,
   bornées, en plus de la prose du modèle.
2. **Le plafond de contexte d'un pas suit le modèle** : `step_context_tokens` déduit de la fenêtre du modèle (le
   registre des points d'accès), avec une part déclarée, plutôt qu'une constante de 64 000 pour une fenêtre d'un
   million.
3. **Une condition de la chaîne de gardes** : au `step_done` d'un pas qui finit plein avec des résultats d'index, un
   refus « écris d'abord ce que tu as trouvé (noms, chemins, lignes) dans ton rapport ou ton plan ».

Le témoin : un pas qui trouve un symbole par `usages` puis atteint son plafond ; le pas suivant écrit un fichier qui le
cite ; le nom et la ligne écrits sont ceux que l'outil avait rendus.
