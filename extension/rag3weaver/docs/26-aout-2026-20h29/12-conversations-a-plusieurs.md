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
