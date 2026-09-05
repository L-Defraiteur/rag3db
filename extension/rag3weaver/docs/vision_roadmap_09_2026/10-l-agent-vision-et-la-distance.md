# L'agent vision, et la distance qu'il mesure

**30 août 2026**, dans la foulée du [doc 09](09-trois-roles-et-une-seule-main.md).
Lucie ajoute un quatrième rôle, et c'est celui qui ferme la boucle :

> *« Un agent vision : il classe, crée des issues vision, et il stocke
> l'intention première de l'utilisateur. L'utilisateur dit un truc, vision
> stocke ça comme vision initiale — irréalisable honnêtement direct par l'agent
> de code, car souvent les utilisateurs veulent des trucs compliqués. Il perd
> jamais la vision initiale utilisateur. Il écoute les conversations et il note
> ce qui commence à répondre à la vision, et comment. Il découpe les features en
> tickets pour l'agent design. Il note aussi les nouvelles bonnes idées à
> proposer à l'utilisateur — « et si on faisait carrément ça ? » — au retour de
> tous les agents. »*

## 1. On le fait déjà à la main, et c'est l'argument

Ce dépôt contient `docs/vision_roadmap_09_2026/` et `docs/issues/<date>/`. Cette
session-ci a produit cinq issues et deux documents de vision. **Le rôle existe
donc déjà** — il est tenu par un humain, à la main, et il produit exactement ces
artefacts.

Ce n'est pas une idée en l'air à essayer : c'est une pratique établie dont on
propose d'automatiser le porteur. C'est la meilleure sorte d'automatisation à
tenter, et la seule qu'on puisse évaluer — on sait à quoi ressemble le travail
bien fait, il est sur le disque.

## 2. Quatre rôles, une chaîne qui se rétrécit

| rôle | ce qu'il tient | son artefact |
|---|---|---|
| **vision** | le **pourquoi** | la vision initiale, les issues, les tickets |
| **design** | le **quoi** | le document de design, et « je valide » |
| **contexte** | ce dont on se souvient | les résumés, le rappel |
| **code** | le **comment** | le code, les tests, le journal |

Chaque étage **ancre celui du dessous**. Le doc 09 posait que la parole de
design borne la mémoire du code ; la vision joue le même rôle un cran plus
haut, et à une autre échelle de temps.

**Et une seule chose change vraiment** : *seul le rôle vision parle à
l'humain*. Le design ne demande pas, le contexte ne demande pas, le code ne
demande pas — sauf la porte des commandes, qui est une question de sûreté et
pas de direction. L'humain a un interlocuteur, pas quatre.

## 3. Deux ancres, deux échelles

C'est la symétrie qui rend l'ensemble cohérent :

| ancre | ce qu'elle empêche | portée |
|---|---|---|
| les deux dernières **manches** de design | que le code redérive son objectif de ses propres traces | quelques tours |
| la **vision initiale**, jamais réduite | que la suite des designs s'éloigne de ce qui a été demandé | toute la session, et au-delà |

Sans la seconde, chaque parole de design peut être raisonnable *par rapport à
la précédente*, et l'ensemble s'éloigner. C'est le même glissement qu'au doc 09,
d'un cran plus haut : il ne se voit jamais d'un tour à l'autre, seulement de
loin.

## 4. Ce que la vision **mesure**, et pas seulement ce qu'elle garde

C'est le point qui empêche ce rôle d'être décoratif. Lucie : *« il note ce qui
commence à répondre à la vision, et comment »*.

L'artefact n'est donc pas « un document de vision ». C'est un **registre de la
distance** :

```
vision initiale (verbatim, jamais réécrite)
  ├── ce qui y répond aujourd'hui, et par quoi
  ├── ce qui n'y répond pas encore
  └── ce qui s'en est éloigné, et quand
```

Cette dernière ligne est celle qui coûte à écrire et qui vaut le plus. Un
registre qui n'enregistre que les progrès est un registre qui ment par
omission — et on passe nos journées à débusquer cette famille-là.

**Le verbatim est essentiel.** La vision initiale se stocke *telle qu'elle a été
dite*, y compris quand elle est irréalisable. La reformuler en quelque chose de
faisable, c'est déjà décider à la place de l'utilisateur — et perdre ce qu'on
prétendait garder.

## 5. Le découpage en tickets, et pourquoi c'est le bon niveau

Vision découpe pour design ; design découpe pour code. Le même geste, deux fois,
et chaque fois la même règle : **celui qui découpe ne fait pas le travail**.

Un ticket de vision ne dit pas comment. Il dit *ce qui manquerait encore si on
ne le faisait pas* — c'est-à-dire une portion de la distance. Ça se vérifie :
un ticket qui ne réduit aucune ligne du registre n'a rien à faire là.

## 6. Les trois façons dont ça peut mal tourner

### Un cliquet qui refuse tout changement de cap

*« Il perd jamais la vision initiale »* peut devenir *« il empêche d'en
changer »*. Les utilisateurs changent d'avis, et ils ont le droit.

La règle : la vision initiale n'est **jamais perdue**, mais elle peut être
**dépassée** — et un dépassement est un **acte explicite de l'utilisateur,
enregistré comme tel**, jamais déduit de la conversation. Les deux visions
restent, datées, avec ce qui a motivé le passage. Sans ça, l'agent fige le
projet ou réécrit en douce ce qu'on lui a demandé, et les deux sont pires que
l'oubli.

### Des propositions qui deviennent du bruit

*« Et si on faisait carrément ça ? »* est la meilleure chose que fasse un humain
et la plus facile à rendre insupportable. Un agent qui propose à chaque retour
apprend à l'utilisateur à ne plus lire.

La borne est la même que pour le souffleur : **ne proposer que ce qui change la
suite**. Et le moment est déjà donné par Lucie — *au retour de tous les
agents*, pas pendant. Une proposition au milieu d'une manche interrompt ; à la
fin, elle oriente.

### Un super-orchestrateur qui décide de tout

Un rôle qui tient le pourquoi, découpe le travail, mesure le progrès et parle à
l'humain concentre beaucoup. Le contrepoids doit être écrit :

**la vision ne valide pas le code et n'écrit pas les designs.** Elle découpe et
elle mesure. Le design garde son droit de refus — y compris envers un ticket
mal découpé, et c'est ce refus-là qui empêche la vision de devenir naïve sans
que personne le voie.

## 7. Ce qui existe déjà pour ça

| | |
|---|---|
| Les artefacts, et leur forme | **existent** — `docs/vision_roadmap_*`, `docs/issues/<date>/` |
| Écouter les conversations sans les interrompre | **existe** — le bus d'événements, les curseurs par sujet |
| Chercher ce qui a été tenté | **existe** — `search(target="Trace")` |
| Les gabarits d'artefacts | **existent** — le catalogue, `place` et `adopt` |
| Une notion de rôle | manque (doc 09) |
| Le registre de la distance | manque |
| Le dépassement explicite d'une vision | manque |

Comme au doc 09 : la plomberie est là. Ce qui manque, ce sont les rôles — et
ici, l'artefact qu'ils tiennent.
