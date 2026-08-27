# 10 — Les outils asynchrones : parler pendant que ça travaille

27 août 2026. Lucie : « faut dès maintenant gestion de tools asynchrone, avec
résolution qui vient plus tard, car en vocal ou même en chat c'est chiant si
l'agent sait plus parler en attendant un tool ».

C'est une contrainte d'**interaction**, pas de performance. Un agent muet
pendant six secondes n'est pas lent, il est *absent* — et en vocal, absent
veut dire cassé.

## 1. Ce qui bloque aujourd'hui

`Agent::run_inner` : le modèle rend des appels d'outils, on les exécute, on
colle les résultats, on rappelle le modèle. Entre les deux, **rien ne sort**.
Le tour ne se termine qu'une fois tous les outils revenus.

## 2. La contrainte que le protocole impose, et qu'on ne peut pas contourner

Un `tool_call` **doit** recevoir une réponse dans le même tour. C'est vrai de
l'API OpenAI comme des gabarits locaux : laisser un appel sans réponse casse
la conversation au tour suivant.

Donc « asynchrone » ne peut pas vouloir dire « on répondra plus tard à cet
appel ». Ça veut dire :

> **On répond tout de suite un accusé, et le vrai résultat arrive ensuite
> comme un message à part.**

C'est une contrainte, pas un compromis — et une fois admise, tout le reste
se dessine.

## 3. Le dessin : un appel d'outil devient un *run*

On a déjà tout, et c'est ce qui rend la chose petite :

| Pièce | Déjà là depuis |
|---|---|
| runs avec identité, parent, portée | 26 août (`RunStarted`, `execute_as`) |
| boîte aux lettres d'agent, curseur | 26 août (`inbox_topic`, `read_inbox`) |
| réacteur, `%% on:`, `%% policy:` | 26 août |
| poignées lisibles (`#execute-2`) | dessinées au [doc 13](../25-aout-2026-18h58/13-la-session-comme-graphe.md) §5, jamais faites |

Le déroulé :

1. le modèle appelle `search(...)` ;
2. la fiche est marquée asynchrone → au lieu d'exécuter, on **lance un run
   enfant** et on rend immédiatement `{ handle: "#search-3", statut:
   "en cours" }` ;
3. le modèle **continue de parler** — « je cherche, deux secondes » — et le
   tour se termine normalement ;
4. le run enfant finit et **poste son résultat dans la boîte de l'agent** ;
5. au tour suivant, la boucle vide la boîte et injecte le résultat comme un
   message ordinaire : « `#search-3` a rendu : … ».

Les poignées du doc 13 trouvent enfin leur usage : c'est **le nom par lequel
le modèle rattache un résultat tardif à sa demande**. Sans elles, un résultat
qui arrive trois tours plus tard n'est rattachable à rien.

## 4. Les trois décisions

### 4.1 Qui décide qu'un outil est asynchrone ? — **la fiche**

Pas le modèle, et pas un seuil de durée.

- **Le modèle** ne sait pas ce qu'il déclenche, et lui donner le choix, c'est
  un paramètre de plus à chaque appel — on a passé la journée d'hier à en
  retirer.
- **Un seuil automatique** (« au-delà de 500 ms, ça devient asynchrone »)
  rend le comportement imprévisible : le même outil répond parfois d'un
  coup, parfois en deux temps, et le modèle ne peut rien apprendre.

Donc `%% async: true` dans la fiche, comme `%% choices:` et `%% on:`. Une
propriété de l'outil, connue à l'avance, la même à chaque appel.

### 4.2 Que voit le modèle tout de suite ? — **un accusé qui se suffit**

Pas `null`, pas `""`. Quelque chose dont il peut parler :

```json
{ "handle": "#search-3", "statut": "en cours", "outil": "search",
  "attendu": "quelques secondes" }
```

Le modèle a alors de quoi dire une phrase vraie. Et le format le laisse
**décider d'attendre** : il peut appeler `await("#search-3")` si la suite
n'a aucun sens sans le résultat. L'asynchrone n'est pas une obligation de
continuer, c'est une permission.

### 4.3 Quand le résultat rentre-t-il ? — **à la frontière de tour, d'abord**

Deux niveaux, et on ne fait que le premier maintenant :

1. **Frontière de tour** : la boucle vide la boîte avant de rappeler le
   modèle. Simple, sans course, suffisant pour du chat.
2. **Interruption en cours de génération** : le résultat arrive pendant que
   le modèle parle et le fait bifurquer. Nécessaire en vocal pour être
   vraiment fluide, mais ça demande de gérer une génération interrompue et un
   flux à recoudre. Plus tard, et seulement si le premier ne suffit pas.

Dire les deux, n'en faire qu'un : le second niveau est une optimisation de
confort, le premier est ce qui débloque l'usage.

## 5. Ce qu'il faut se garder de faire

- **Attendre en silence.** Si un résultat n'est pas encore là et que le
  modèle ne l'a pas demandé, on ne bloque pas — mais on ne l'oublie pas non
  plus. Un run en cours doit être **visible** dans l'état de la session,
  sinon on refait la famille de défauts de la journée : l'absence invisible.
- **Perdre un résultat parce que la conversation a avancé.** La boîte aux
  lettres a un curseur ; un résultat non lu reste non lu. Il arrive tard,
  jamais jamais.
- **Laisser deux appels partager une poignée.** La poignée est l'identité
  d'une demande ; deux demandes identiques dans le même tour sont deux
  poignées.
- **Rendre l'accusé indiscernable d'un résultat.** Si le modèle ne peut pas
  distinguer « c'est parti » de « voilà », il racontera le premier comme s'il
  était le second — et il aura l'air de mentir.

## 6. Ce que ça donne côté vocal

C'est le cas qui a motivé la demande, et il révèle une chose : la latence
tolérable dépend de **ce qu'on peut dire en attendant**. Un agent qui dit
« je regarde dans le dépôt, ça prend deux secondes » a acheté deux secondes.
Un agent muet n'en achète aucune.

D'où une conséquence sur l'accusé : il doit porter **de quoi faire une phrase
naturelle** — le nom de l'outil et une idée de la durée. `{"pending": true}`
ne permet de dire que « euh ».

## 7. L'ordre

1. `%% async:` dans la fiche, et l'accusé avec sa poignée.
2. Le run enfant, et la publication du résultat dans la boîte.
3. Le vidage à la frontière de tour, et l'injection comme message.
4. `await(poignée)` pour quand la suite n'a pas de sens sans le résultat.
5. *(plus tard)* l'interruption en cours de génération.

Les quatre premiers points ne demandent **aucune pièce nouvelle** : runs,
boîte aux lettres, réacteur et fiches existent tous depuis hier. C'est de
l'assemblage, et c'est le bon signe.
