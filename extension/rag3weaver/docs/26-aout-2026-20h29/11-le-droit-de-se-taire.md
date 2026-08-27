# 11 — Le droit de se taire, et celui de raccrocher

27 août 2026, suite du [10](10-outils-asynchrones.md). Lucie, deux demandes
qui n'en font qu'une :

> qu'il sache choisir de faire une pause, quand il n'y a rien à dire à part
> attendre réponse ou résultat tool, qu'il se sente pas obligé de parler tout
> le temps
>
> et qu'il sache terminer discussion avec interlocuteur non humain par lui
> même (« au revoir, merci beaucoup… » « merci beaucoup au revoir » « de
> même, au revoir… »)

Les deux sont le même manque : **la boucle n'a pas le droit de ne rien
dire**. Elle doit produire du texte ou des appels d'outils ; il n'existe
aucune troisième issue. D'où un agent qui meuble, et deux agents qui se
disent au revoir jusqu'à la fin des temps.

## 1. Se taire est une action, pas une absence

Le piège serait de traiter le silence comme « le modèle n'a rien produit ».
Un silence par défaillance et un silence choisi se ressemblent en sortie et
n'ont rien à voir : le premier est une panne, le second une décision.

Donc **`pause` est une action explicite**, avec une raison obligatoire :

```
pause(pour: "#search-3")        // j'attends ce résultat
pause(pour: "réponse de lucie") // j'attends un humain
pause(pour: "10s")              // je laisse passer du temps
```

Trois propriétés, chacune contre une manière de se tromper :

- **Une pause sans raison est refusée.** Sinon c'est une porte de sortie
  quand le modèle ne sait pas quoi faire, et on aura des agents qui
  s'endorment au lieu de travailler.
- **Un agent en pause n'est pas un agent arrêté.** C'est un run à l'état
  *en attente*, **visible** dans l'état de la session, avec ce qu'il attend.
  Sinon on refait la famille de défauts de la semaine : l'absence invisible.
- **La pause se réveille toute seule** — un message dans la boîte, un
  résultat d'outil, un délai. C'est exactement le réacteur du 26 août, et il
  n'y a rien à écrire pour ça.

Et la conséquence à assumer : **ne rien dire devient une réponse
acceptable**, y compris au premier tour. Un agent à qui on demande d'attendre
et qui répond « d'accord, j'attends » a mal compris la consigne.

## 2. Raccrocher : le problème des politesses infinies

L'exemple de Lucie est un vrai mode de panne, connu et parfaitement
reproductible :

```
A : « au revoir, merci beaucoup »
B : « merci beaucoup, au revoir »
A : « de même, au revoir »
B : …
```

La cause n'est pas la bêtise du modèle, c'est que **chaque tour se termine
par une forme qui appelle une réponse**, et qu'aucune règle ne dit quand un
échange n'a plus de contenu. Chacun fait ce qu'on lui a appris à faire :
répondre poliment.

### 2.1 Le principe

> **La politesse n'est pas du contenu.** Entre agents, un échange existe pour
> déplacer de l'information ; quand plus rien ne se déplace, il est fini.

Un humain a droit au rituel — dire au revoir a une valeur en soi, et couper
la parole à quelqu'un est une faute. Un agent, non. C'est donc une
**politique par nature d'interlocuteur**, pas une constante du moteur.

### 2.2 Trois mécanismes, du plus explicite au plus bête

1. **`close(raison)`** — la sortie propre. Le modèle décide que l'échange est
   terminé. C'est **unilatéral et engageant pour celui qui ferme** : il se
   désabonne du fil et publie l'événement. L'autre l'apprend et sait que ses
   messages ne vont plus nulle part.
2. **Le pair l'apprend.** Un `ConversationClosed` sur le fil, comme n'importe
   quel événement du bus. Fermer sans le dire produirait un agent qui parle
   à un mur — la même absence invisible, encore.
3. **Un plafond de tours sans progrès.** Bête, prévisible, et impossible à
   contourner : entre pairs non humains, *N* tours sans un seul appel
   d'outil ni une seule donnée nouvelle, et le fil se ferme tout seul.

Le troisième existe parce que les deux premiers demandent au modèle de bien
juger. Le plafond, lui, ne demande rien à personne — et c'est précisément ce
qu'on veut d'un garde-fou contre une boucle infinie. Une politesse de plus
n'est pas un progrès ; deux agents peuvent en produire indéfiniment.

### 2.3 Ce qu'on ne fait pas

- **Détecter la politesse par le sens.** « Est-ce que ce message apporte
  quelque chose ? » est un jugement, donc faillible, donc une boucle infinie
  un jour de malchance. Le plafond compte des tours : il ne se trompe pas.
- **Fermer d'autorité une conversation avec un humain.** Le plafond ne vaut
  qu'entre pairs non humains. Un humain qui ne répond pas n'est pas une
  boucle, c'est quelqu'un qui fait autre chose.
- **Rouvrir un fil fermé sur un « au revoir » de plus.** Une fermeture est
  définitive pour celui qui l'a prononcée. Sinon on a réinventé le problème.

## 3. Comment on sait que le pair n'est pas humain

Ça se lit, ça ne se devine pas. Un message porte son émetteur : un `run`
publie sur `run.<id>`, un humain arrive par un canal d'interface. La nature
de l'interlocuteur est donc **une donnée de l'enveloppe**, pas une intuition
sur le style du texte.

C'est important : deviner « ça ressemble à un bot » au contenu serait faux un
jour sur dix, et ce jour-là on raccrocherait au nez de quelqu'un.

## 4. Ce que ça change dans la boucle

| Aujourd'hui | Avec |
|---|---|
| texte, ou appels d'outils | texte, appels d'outils, **`pause`**, **`close`** |
| un tour finit quand le modèle a parlé | un tour peut finir sans un mot |
| une conversation finit quand l'appelant s'arrête | une conversation peut se fermer des deux côtés |
| attendre = bloquer | attendre = un état visible qui se réveille |

## 5. L'ordre

1. **`pause(pour:)`** et l'état *en attente* visible — c'est le complément
   direct du [doc 10](10-outils-asynchrones.md) : sans pause, l'accusé de
   réception oblige quand même à meubler.
2. **Le plafond de tours sans progrès** entre pairs non humains. Bête,
   court, et il supprime la boucle infinie tout de suite.
3. **`close(raison)`** et `ConversationClosed` sur le fil.
4. La politique par nature d'interlocuteur, quand on aura plus d'un genre de
   pair.

Comme au doc 10 : aucune pièce nouvelle. Les runs, la boîte aux lettres, le
réacteur et les sujets du bus font déjà tout ça — il manque les **verbes**.
