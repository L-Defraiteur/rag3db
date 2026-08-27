# 12 — Conversations à plusieurs : pauses, natures, et pourquoi le plafond était une mauvaise idée

27 août 2026. Remplace le §2 du [doc 11](11-le-droit-de-se-taire.md), sur
correction de Lucie.

## 0. Ce que je proposais, et pourquoi c'était faux

Je proposais un **plafond de tours sans progrès** entre pairs non humains :
au bout de *N* échanges sans donnée nouvelle, le fil se ferme tout seul.

Lucie : « non pas de plafond codé en dur, faut mécanisme vraiment abstrait…
faut juste une action pour s'arrêter de dialoguer non ? la conversation est
toujours possible avec tel interlocuteur ».

Elle a raison, et l'erreur mérite d'être nommée parce qu'elle est classique :
**je traitais le symptôme.** Le symptôme, c'est « ils parlent trop » ; la
cause, c'est que **chaque tour est une prise de parole, et une prise de
parole appelle une réponse**. Un plafond compte les tours d'une boucle qu'il
laisse exister. Retirer la pression de répondre la fait disparaître.

Et un plafond a deux défauts propres : il coupe une conversation légitime
qui prend son temps, et il **détruit** au lieu de suspendre — alors que la
conversation devrait rester possible.

## 1. Le modèle

Trois objets, et rien de plus :

| Objet | Ce que c'est |
|---|---|
| **Conversation** | un fil, avec un ensemble de **participants**. Elle ne se ferme pas. |
| **Participant** | une identité et une **nature** : humain, agent. (Plus tard : service, déclencheur.) |
| **Posture** | pour un couple ordonné (A → B), dans une conversation : *parlant* ou *en pause*. |

La posture est **par couple**, pas par conversation. C'est ce qui permet à
deux agents de se taire l'un l'autre tout en restant présents pour l'humain.

## 2. Les verbes

```
pause_dialogue(avec: <participant>, raison)   // je n'ai plus rien à te dire
confirm_pause(avec: <participant>, raison)    // moi non plus, et je le dis
leave(raison)                                 // je ne suis plus participant
```

Et **aucun verbe pour fermer une conversation.** Elle reste toujours
possible ; ce qui finit, c'est un *run*, pas un fil.

### 2.1 La règle qui fait tout tenir

> **Se mettre en pause, c'est arrêter de parler. Ce n'est pas arrêter
> d'entendre.**

Sans cette asymétrie, une pause serait une surdité : le pair ne pourrait plus
jamais réengager, et « la conversation est toujours possible » serait faux.
Avec elle :

- A se met en pause vers B, avec une raison ;
- B est **notifié** : *« A a mis la communication en pause — raison : …. Si
  tu penses devoir la clore de ton côté aussi, appelle
  `confirm_pause(A, raison)`. »* ;
- B confirme → le couple est silencieux des deux côtés, la raison est
  gardée **en session** ;
- ou B parle → A l'entend, et **la pause de A tombe d'elle-même**, parce
  qu'on ne peut pas être en pause et répondre en même temps.

### 2.2 Pourquoi ça tue la boucle de politesses

```
A : « au revoir, merci beaucoup »      → puis pause_dialogue(B, "échange terminé")
B : reçoit une notification, pas une réplique
B : confirm_pause(A, "de même")        → fin, deux tours
```

Une pause **n'est pas une prise de parole** : elle n'appelle pas de réponse.
La boucle infinie n'existait que parce que tout ce qu'on pouvait produire
était une réplique. Rien à plafonner, rien à compter.

## 3. Les cas, un par un

### 3.1 Un humain, un agent

Le cas courant. L'agent peut se mettre en pause (« j'attends ta réponse »),
et l'humain n'a **rien à confirmer** : il parle quand il veut, la pause de
l'agent tombe.

Un agent ne devrait **jamais** demander à un humain de confirmer une pause :
ce serait lui donner du travail administratif pour une chose qu'il fait
naturellement en se taisant.

### 3.2 Plusieurs humains, un agent

La posture étant par couple, une pause vers Lucie n'est pas une pause vers
quelqu'un d'autre. Et une parole adressée **au fil** est entendue par tous,
y compris par un pair envers qui on est en pause — ce n'est pas une
violation : la pause dit *« je n'ai plus rien à te dire »*, pas *« je refuse
d'être entendu de toi »*.

### 3.3 Deux agents qui font un travail ensemble, dans le fil d'un humain

Ils peuvent se mettre en pause l'un l'autre — leur partie est finie — **sans
quitter** la conversation, et rester disponibles pour l'humain. C'est
exactement ce que la posture par couple permet et qu'un « close » de
conversation aurait interdit.

### 3.4 Un agent dans la conversation, qui parle à un autre ailleurs

Deux conversations, deux ensembles de participants, deux jeux de postures.
Un agent est un participant de plusieurs fils, et son état de pause est
**(fil, moi, toi)**. Rien de spécial à prévoir : c'est le même objet, deux
fois.

### 3.5 Tout le monde en pause

Le fil est **silencieux**, pas fermé. Quelqu'un parle → il reprend. C'est le
sens de « la conversation est toujours possible », et c'est ce qui rend un
plafond inutile : le silence n'a pas besoin d'être décidé une fois pour
toutes.

### 3.6 Un participant s'en va

`leave(raison)` : différent d'une pause. Il n'est plus dans la liste, on ne
lui adresse plus rien, et un message pour lui n'attend pas. Un agent dont le
travail est fini part ; un humain part quand il veut, et **jamais parce
qu'un agent l'a décidé**.

## 4. Le seul vrai piège : l'attente circulaire

C'est le mode de panne que mon plafond attrapait par accident, et il faut le
remplacer par mieux :

> A est en pause **en attendant B** ; B est en pause **en attendant A**.

Personne ne parle, personne n'est en faute, et rien ne se passe. Un plafond
l'aurait cassé au hasard ; on peut faire exact, parce que c'est un problème
de **graphe** et qu'on en est une base :

- une pause porte sa raison, et une raison peut être *« j'attends X »* ;
- « qui attend qui » est un graphe orienté ;
- **un cycle dans ce graphe est un blocage**, et ça se détecte sans juger de
  quoi que ce soit.

Quand un cycle apparaît, on ne devine pas qui doit reprendre la parole : on
le **dit**, aux participants et à la trace. Un blocage annoncé est un
problème ; un blocage silencieux est une panne.

Et c'est le remplacement honnête du plafond : au lieu de compter des tours en
espérant que ça corresponde à quelque chose, on détecte **exactement** la
seule situation qui ne peut pas se résoudre toute seule.

## 5. Ce qui vit où

| Ce que c'est | Où ça vit | Pourquoi |
|---|---|---|
| la posture d'un couple, la raison | **en session** | c'est de l'état d'interaction, pas de la connaissance |
| `DialoguePaused`, `PauseConfirmed`, `Left` | **sur le bus** | donc tracé, écoutable, rejouable |
| la nature d'un participant | **dans l'enveloppe** | ça se lit, ça ne se devine pas au style du texte |
| la liste des participants | **la conversation** | un fil est un objet, pas une suite de messages |

## 6. Ce qu'on ne fait pas

- **Fermer une conversation.** Il n'y a pas de verbe pour ça, et c'est
  volontaire.
- **Plafonner les tours.** §0.
- **Deviner la nature d'un pair au contenu.** Faux un jour sur dix, et ce
  jour-là on raccroche au nez de quelqu'un.
- **Faire confirmer une pause à un humain.** §3.1.
- **Traiter une pause comme une surdité.** §2.1.

## 7. L'ordre

1. `pause_dialogue(avec:, raison)` et la notification au pair — c'est ce qui
   supprime la boucle de politesses, et c'est petit.
2. `confirm_pause(avec:, raison)` et la posture gardée en session.
3. La détection d'attente circulaire, quand deux agents s'attendront pour de
   vrai.
4. `leave(raison)`.
5. La politique par nature (ne jamais faire confirmer un humain, etc.),
   quand il y aura plus de deux natures.

Les postures se rangent dans la session, les notifications sont des
événements du bus, les participants sont ceux du fil : **toujours aucune
pièce nouvelle**. Il manque les verbes, et un graphe d'attente à regarder.

## 8. La raison est un couple : un genre, et un texte

Proposition de Lucie : « tu peux pauser peut-être avec un arg enum en plus de
la raison texte — `finished`, `waiting for` + run id, `waiting for
instruction`… ». Oui, et c'est la même discipline que les `%% choices:` du 26
août : **une liste exacte plutôt qu'un champ libre**, parce qu'un moteur ne
peut rien faire d'une phrase.

```
pause_dialogue(avec: B, genre: waiting_for_run("#search-3"),
               raison: "je cherche d'où vient ce symbole")
```

- le **genre** est lu par le moteur ;
- le **texte** est lu par un humain, ou par le pair.

### 8.1 Le critère d'admission d'une variante

Il en faut un, sinon la liste enfle jusqu'à ne plus rien vouloir dire :

> **Le genre est la condition de réveil.** Deux genres qui se réveillent
> pareil sont un seul genre.

Ça tranche tout de suite : « poliment terminé » et « travail fini » se
réveillent pareil (un nouveau message), donc c'est le même genre. La nuance
appartient au texte.

### 8.2 Les six

| Genre | Ce qui réveille | Fait une arête d'attente ? |
|---|---|---|
| `finished` | un nouveau message qui m'est adressé | non |
| `waiting_for_run(id)` | ce run se termine | **oui** — vers un run |
| `waiting_for_peer(qui)` | ce participant parle | **oui** — vers un participant |
| `waiting_for_instruction` | n'importe quel message humain | non |
| `waiting_until(quand)` | l'horloge | non |
| `blocked` | **rien** — il faut le dire | non |

Deux commentaires qui justifient la liste :

**`blocked` n'est pas `finished`.** L'un a la forme d'un succès, l'autre d'un
échec, et les confondre cacherait exactement ce qu'on veut voir. Un agent
`blocked` ne se réveille pas tout seul : c'est **le seul genre qui doit
remonter**, tout de suite, à qui peut le débloquer. Se taire parce qu'on a
fini et se taire parce qu'on est coincé, ce n'est pas la même chose.

**`waiting_for_instruction` n'est pas `waiting_for_peer(humain)`.** Le second
attend *quelqu'un en particulier* et entre dans le graphe d'attente ; le
premier attend *une direction, de qui voudra*. Et un humain ne fait jamais
partie d'un cycle : il n'attend pas, il vit sa vie.

### 8.3 Ce que le genre achète, concrètement

- **Le réveil sans invention** : plus de « quand faut-il rappeler cet
  agent ? ». Le genre le dit.
- **Le graphe d'attente exact** (§4) : seuls `waiting_for_run` et
  `waiting_for_peer` créent une arête. Un cycle ne peut donc se former
  qu'entre des attentes réelles — jamais parce que quelqu'un attend un
  humain, ce qui aurait produit de faux blocages tous les quarts d'heure.
- **La liste exacte au modèle**, avec le « vouliez-vous dire » qu'on a déjà :
  `Choices::Fixed` et `GraphToolError::BadChoice` sont écrits depuis hier,
  il n'y a qu'à s'en servir.
- **Un état de session lisible** : « 3 agents, dont 1 `blocked` » se
  répond d'un coup d'œil ; « 3 agents en pause » ne dit rien.

### 8.4 Le piège du réveil manqué

Il faut le nommer parce qu'il est classique et qu'il ne pardonne pas :
`waiting_for_run("#search-3")` posé **après** que le run se soit terminé.
L'événement est passé, la pause ne se réveillera jamais.

Donc l'enregistrement d'une pause **vérifie sa condition tout de suite** :
si le run est déjà fini, on ne se met pas en pause du tout. C'est du niveau,
pas du front — et c'est la seule façon que ce soit juste sans verrou.

## 9. Ce qui attend doit se **voir**, pas se rappeler

Lucie encore : « au pause peut-être dire "attention, l'agent machin attend
toujours votre réponse", ou un flag dans le contexte du système prompt
persistant, indépendant de l'agent ».

Les deux formes sont possibles, et **elles ne valent pas la même chose**.

### 9.1 Un message se perd, un état se lit

Dire une fois « A attend ta réponse » est une **prise de parole** : ça
défile, ça se noie, et le répéter devient du harcèlement. On aurait alors le
choix entre oublier et insister — les deux mauvais.

Un **bloc d'état dans le contexte** est autre chose : il est là à chaque
tour tant que l'attente dure, et il **disparaît tout seul** quand elle cesse.
Ce n'est pas une notification, c'est une lecture.

C'est exactement le remède qu'on applique depuis trois jours à la même
famille de défauts : *rendre visible ce dont l'absence ne se voit pas*. Un
agent qui attend est invisible par nature — il ne fait rien, il ne dit rien.

### 9.2 Il est **dérivé**, jamais écrit

Le bloc n'est pas un texte que quelqu'un compose et range quelque part. Il
est **calculé au moment d'assembler l'invite**, à partir des postures :

```
en attente de vous :
  · rechercheur (#run-12) — waiting_for_peer, « il me faut le chemin exact »
  · indexeur   (#run-9)  — blocked, « aucune racine autorisée pour /tmp »
```

Trois conséquences, toutes bonnes :

- **Il ne peut pas être périmé.** Une projection d'un état ne ment pas ; un
  message archivé, si.
- **Il disparaît sans qu'on y pense.** La pause tombe, la ligne s'en va. Rien
  à nettoyer, donc rien à oublier de nettoyer.
- **Il est le même pour tout le monde** — c'est la même fonction, avec un
  destinataire différent.

### 9.3 Indépendant de l'agent, et c'est le point

Lucie insiste là-dessus et elle a raison : **ce n'est pas à l'agent de dire
qu'on l'attend.** S'il devait le faire, il faudrait qu'il pense à le faire,
et un agent en pause est justement celui qui ne fait rien.

Le bloc appartient donc à la **session** — au graphe qui assemble le tour
(doc [13](../25-aout-2026-18h58/13-la-session-comme-graphe.md), `assemble`).
L'agent est une **donnée** de ce calcul, pas son auteur.

### 9.4 Adressé, et compact

- **Adressé** : chacun voit ce qui attend **de lui**. Montrer à Lucie que
  l'agent A attend l'agent B ne sert qu'à l'encombrer — sauf si c'est un
  cycle (§4), et là c'est justement à elle qu'il faut le dire.
- **Compact** : ça vit dans l'invite système, donc ça se paie à chaque tour.
  Une ligne par attente, et **rien du tout** quand il n'y a rien. Un bloc
  toujours présent, même vide, apprend au modèle à ne plus le lire.

### 9.5 Ce qu'on n'en fait pas

- **Pas un rappel qui se répète.** Le bloc est là parce que l'état est là ;
  il n'insiste pas, il constate.
- **Pas un ordre.** « A attend ta réponse » n'oblige à rien : on peut
  décider que A attendra. Ce qu'on ne peut plus faire, c'est ne pas le
  savoir.
- **Pas un journal.** Ce qui a été attendu puis obtenu appartient à la trace
  du bus, pas à l'invite. L'invite dit **ce qui est**, jamais ce qui fut.
