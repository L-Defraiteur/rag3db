# 09 — Le terminal à plusieurs : identité, rôle, personnalité

Idée de Lucie, 27 août : un terminal où l'on s'adresse à des personnages —
*« Alma qui est architecte démoniaque, Zed qui est un reviewer de code et
sécu »* — où l'on envoie une tâche et où **plusieurs répondent**, où l'on peut
s'adresser à un en particulier.

Elle la présente comme une ambition naïve. La partie naïve est le décor ; **le
reste est une architecture, et elle est presque construite.**

## 1. La phrase qui tient tout

> « Un prompt devrait être agnostique de personnalité et de rôle, et lié à la
> tâche. Voire même la plupart du temps éviter les prompts systèmes autres que
> ceux builtins du moteur. »

C'est le cœur, et c'est juste pour trois raisons qui ne sont pas des goûts.

**Une personnalité dans une invite est un coût par tour.** Elle est réenvoyée
à chaque appel, elle occupe du contexte, et le modèle s'en écarte à mesure que
la conversation grandit. Une personnalité dans le graphe est lue quand elle
sert.

**Un rôle écrit en prose n'est pas vérifiable.** *« Tu es Zed, expert sécurité »*
ne contraint rien : Zed peut écrire du code applicatif, effacer une entité,
répondre à côté. Un rôle exprimé comme **enveloppe de capacités** — quels
outils il voit, quel domaine il regarde, quels événements il écoute, ce qu'il
a le droit d'écrire — se fait respecter par le moteur.

**Et le costume ne remplace pas la compétence.** C'est le vrai danger du
personnage : il produit un ton assuré sans rien derrière. Le [doc 06 §2.2](06-le-tamagotchi-et-le-compilateur.md)
le dit déjà pour la spécialisation ; ici c'est pire, parce que la théâtralité
rend le vide plus convaincant.

## 2. Trois choses distinctes, souvent confondues

| | Ce que c'est | Où ça vit | Vérifiable ? |
|---|---|---|---|
| **Identité** | qui c'est, à travers les sessions | `Participant` dans le graphe | oui — elle a des runs, des messages, une histoire |
| **Rôle** | ce qu'il a le droit de faire et de voir | `WorkDomain` + outils + politiques | oui — le moteur refuse le reste |
| **Personnalité** | comment il le dit | un peu de texte, et **rien d'autre** | non, et c'est acceptable **si** elle ne touche qu'au ton |

La règle qui découle : **la personnalité n'a le droit de porter que le
registre.** Alma peut être théâtrale sur la *façon* de dire ; jamais sur le
*fait que ce soit vrai*. Le jour où le décor influence un jugement, on a
fabriqué un menteur charismatique.

## 3. Le critère qui rend tout ça falsifiable

Il découle directement de la phrase de Lucie, et il est mesurable :

> **Donnez la même tâche à Alma et à Zed. La différence entre leurs réponses
> doit s'expliquer par leur domaine et leur histoire — jamais par leurs
> adjectifs.**

Si en retirant les deux lignes de personnalité les réponses deviennent
identiques, alors le rôle n'existait pas : il n'y avait qu'un costume. C'est un
test qu'on peut écrire.

Et ça donne le bon ordre de construction : **le rôle d'abord, la personnalité
en dernier et en petit.**

## 4. La spécialisation vient de l'histoire, pas de l'étiquette

Zed n'est pas bon en sécurité parce qu'on le lui a dit. Il l'est parce que ses
runs sont des revues de sécurité, donc sa mémoire en contient, donc sa
recherche y ramène.

**C'est exactement ce qu'on a livré cet après-midi** : `Participant` est
maintenant une identité stable — plus une adresse de run — et
`Participant -PERFORMED-> Run` referme le chemin *ce qui a été dit → le run →
celui qui l'a mené*. La question « qui a travaillé là-dessus » est devenue un
parcours.

Il faut être honnête sur ce que ça produit : une **recherche qui trouve des
choses de sécurité**, pas une compétence. C'est déjà beaucoup, et il ne faut
pas promettre l'autre chose.

## 5. Le droit de se taire est ce qui rend le terminal utilisable

Cinq personnages qui répondent à tout est injouable au troisième message.

Or c'est fait : `pause_dialogue`, `confirm_pause`, `PauseKind`, et la détection
d'attente circulaire des [docs 11 et 12](../26-aout-2026-20h29/12-conversations-a-plusieurs.md).
Un agent qui n'a rien à dire se tait, et **ça se voit** — le bloc d'attentes
est dérivé des postures au moment d'assembler.

On a construit ça hier soir pour une autre raison. C'est la pièce qui manquait
à cette idée-ci, et personne ne l'avait vue venir.

## 5 bis. Le tour de parole : couper, et s'en souvenir

Idée de Lucie, dans la foulée : un **classifieur d'après-coup** qui, voyant
l'historique et les premiers mots d'un tour, décide qu'un agent vient de couper
la parole à un autre — et qui **garde dans l'historique du coupé qu'il l'a
été**, pour qu'il le voie au tour suivant.

Ça complète le silence sur le trou qu'il ne bouche pas : se taire règle *« je
n'ai rien à ajouter »*, pas *« nous sommes trois à avoir quelque chose de
légitime à dire maintenant »*.

### Ce n'est pas une métaphore : le mécanisme existe

`TokenSink::on_token(delta) -> Flow` voit **chaque fragment au fil de la
génération**, et `Flow::Stop` doit être honoré immédiatement — c'est dans le
contrat du trait, pas une option. Et `a_cancellation_without_calls_keeps_the_partial_text`
vérifie déjà qu'une annulation **garde le texte partiel**.

Donc un agent peut réellement être coupé au milieu de sa phrase, et garder ce
qu'il avait commencé à dire. Rien à inventer : un puits qui regarde les
premiers fragments, consulte l'arbitre, et rend `Flow::Stop`.

### Le piège : « poids social » ne doit rien stocker

C'est le mot dangereux de l'idée, et il faut le désamorcer avant d'écrire une
ligne. Un **score de préséance rangé quelque part** est exactement l'inertie du
[doc 05 §1.1](05-la-reputation-des-abstractions.md) : celui qui a pris la
parole une fois la prendra toujours, et plus rien ne le déloge.

Le droit de parole se **dérive du contexte**, il ne se stocke pas :

| Signal | D'où il vient |
|---|---|
| à qui le message était adressé | l'enveloppe |
| dans quel domaine tombe le sujet | `WorkDomain` |
| qui attend déjà une réponse | `Postures` |
| qui vient de parler | le fil |

*« Zed a été coupé parce que le sujet est dans le domaine d'Alma et que la
question lui était adressée »* se vérifie. *« Zed avait moins de poids »* ne se
vérifie pas.

### Une interruption est un fait de la conversation, pas un attribut

Corollaire du point précédent, et il compte : si l'on accumule les
interruptions sur l'individu, on obtient *« Zed se fait souvent couper »* comme
réputation — et on a reconstruit l'inertie **à l'intérieur du dialogue**. Le
fait appartient au fil ; l'agent y a participé, il n'en est pas la propriété.

### Ce que le coupé voit, et qui décide

Même forme que le bloc d'attentes (§6) : **dérivé, présent tant que ça dure,
disparu dès qu'il reparle.**

```
vous avez été interrompu par alma, alors que vous disiez :
  « …et le schéma nullable n'est pas garanti par »
```

Et comme le partiel est gardé, **c'est lui qui décide** de reprendre ou de
lâcher — plutôt que nous à sa place. C'est aussi ce qui rend l'interruption
supportable : rien n'est perdu, seulement différé.

### Deux réserves

**L'arbitre coûte un appel** s'il est un modèle — et sur chaque tour de chaque
agent, ça se voit. Le compteur ([doc 08](08-le-compteur.md)) le rendra visible ;
et il y a de bonnes chances que ce soit un travail de **règles** plutôt que de
modèle, puisque les quatre signaux ci-dessus sont tous lisibles sans en appeler
un.

**Une interruption doit rester rare pour porter du sens.** Si tout le monde est
coupé tout le temps, c'est un filtre avec des étapes en plus. Ce qui vaut, ce
n'est pas d'avoir écarté du bruit — c'est l'information transportée : *tu
parlais hors de ton tour, à propos de ceci*.

## 6. L'invite système devient une projection du graphe

C'est la conséquence la plus intéressante de la phrase de Lucie, et elle est à
portée.

`Agent::refresh_waiting_block` fait déjà exactement ça : un bloc **calculé à
chaque tour** depuis l'état, jamais rangé, vide quand il n'y a rien, placé en
fin d'historique pour ne pas casser le cache du fournisseur.

La même mécanique porte le reste : qui je suis, ce que je peux faire ici, qui
d'autre est dans le fil. Donc :

> **L'invite système n'est pas un document. C'est une vue, recalculée, sur ce
> que le graphe sait au moment d'assembler.**

Ce qui explique pourquoi *« éviter les prompts systèmes autres que les builtins »*
est le bon réflexe : les builtins portent le **contrat** — le protocole, les
outils, l'interdiction d'inventer. Tout le reste est de l'état, et l'état se
projette.

## 7. Ce qui existe déjà, et ce qui manque

**Déjà là** — et c'est ce qui rend l'idée sérieuse plutôt que rêveuse :

| Pièce | État |
|---|---|
| identités persistantes | `Participant`, stable depuis aujourd'hui |
| qui a fait quoi | `PERFORMED`, aujourd'hui |
| fils, avec dates lisibles | `Conversation`, `IN_CONVERSATION`, fuseau réel |
| nature d'un participant **par fil** | sur l'arête `PARTICIPATES_IN` |
| adressage et boîtes | `run.<id>.inbox`, `SendMessage` |
| se taire, raccrocher, blocages | `Postures`, `PauseKind`, cycles |
| ce que chacun voit | `WorkDomain` + `Selector` |
| outils comme graphes versionnés | `GraphTool`, gabarits |
| ce que ça coûte | le compteur, aujourd'hui |

**Ce qui manque, et c'est court :**

1. **Un fil à plus de deux.** `conversation_id` dérive d'une paire ; il faut un
   identifiant explicite dans l'enveloppe. Petit, et déjà noté.
2. **La diffusion.** Poser une tâche *dans un fil* plutôt qu'à un destinataire,
   et laisser chacun décider s'il a quelque chose à dire.
3. **L'attribution du rôle à une identité** — aujourd'hui `WorkDomain` n'est pas
   attaché à un `Participant`.
4. **Les gabarits builtin** (`builtin/reviewer`, `builtin/architect`), qui sont
   le vrai travail de produit : le moteur reste universel, **les builtins
   donnent une opinion forte** ([doc 06 §1.6](06-le-tamagotchi-et-le-compilateur.md)).

## 8. Le garde-fou, nommé

Un terminal de personnages a un mode de défaillance propre : **le charme
remplace la preuve**. Cinq voix distinctes donnent l'impression d'un débat
contradictoire alors qu'elles partagent le même modèle, le même contexte et les
mêmes angles morts.

La protection est celle qu'on applique partout ici : **ce qui est affirmé doit
pointer vers quelque chose qu'on peut aller relire** — un run, un commit, une
mesure. Alma a le droit d'être théâtrale. Elle n'a pas le droit d'être la seule
source de ce qu'elle avance.

Et cinq agents qui se trompent ensemble avec cinq styles différents restent une
seule erreur. Ce n'est pas une raison de ne pas le faire ; c'est une raison de
ne pas confondre la polyphonie avec la vérification.
