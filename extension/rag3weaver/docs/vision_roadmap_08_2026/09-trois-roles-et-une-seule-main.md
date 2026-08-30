# Trois rôles, et une seule main sur le code

**30 août 2026.** Lucie, en deux temps. D'abord : *« nous on veut des agents qui
se parlent, qui savent prendre des décisions »*, et le mode `auto` en première
classe plutôt qu'en option. Puis, en découpant : *« c'est l'agent qui touche
réellement à l'écriture du code qui est en auto / edit / standard ; l'agent de
contexte ne fait que discuter avec l'agent de code ; l'agent de design sert
d'user »*.

Ce document tient sur une observation, et c'est elle qui décide du reste.

## 1. Ce qu'un humain fait, et que personne ne fait à sa place

Regardons quand un humain intervient vraiment dans une session. Trois moments,
toujours les mêmes :

1. **Il relève le niveau.** *« Oui, mais as-tu pensé à ça ? Je pense qu'on
   pourrait plutôt envisager ceci, ce serait moins naïf, non ? »*
2. **Il valide un design** avant qu'on écrive le code. *« OK, je valide ton
   doc. »*
3. **Il décide de ce qu'on garde.** En fin de session : *« range ça dans un
   md »* — rapport, fiche de progression, ce qui reste, vidage de connaissance.

Le reste — chercher, lire, écrire, tester — un agent le fait déjà. Ces
trois-là, non. **Ce sont donc eux qu'il faut nommer**, et pas « un agent qui
aide ».

## 2. Trois rôles, et un seul touche au code

> **Un quatrième rôle s'est ajouté le même jour** : la vision, qui tient le
> *pourquoi* et parle à l'humain. Voir
> [doc 10](10-l-agent-vision-et-la-distance.md). Le tableau ci-dessous reste
> vrai — il décrit les trois rôles qui travaillent, sous celui qui oriente.

| rôle | ce qu'il fait | ce qu'il ne fait **jamais** |
|---|---|---|
| **design** | propose, valide, refuse ce qui est fait trop simplement. Tient le cap. | écrire du code |
| **contexte** | cherche, se souvient, souffle au bon moment | écrire du code, décider |
| **code** | lit, écrit, exécute | décider de l'objectif |

**Les modes de permission ne concernent que le rôle `code`.** C'est le
découpage que Lucie a fait, et il simplifie tout : `standard`, `approbation` et
`auto` sont des réglages d'une seule main. Les deux autres rôles n'ont pas
besoin de porte parce qu'ils n'ont pas de verbe qui écrit — et c'est une
propriété à faire tenir par les outils qu'on leur donne, pas par leur bonne
volonté.

**`auto` devient le défaut du rôle `code`.** Un garde qui demande toujours est
un garde que personne n'active. Les trois modes restent ; le centre de gravité
se déplace, parce qu'un agent qui doit demander pour lancer les tests ne fera
jamais deux tours de suite.

## 2 bis. Les tempéraments, et pourquoi chaque triplet se tient

Lucie, en trois lignes :

| rôle | tempérament |
|---|---|
| **vision** | ambitieux · visionnaire · **observateur** |
| **design** | abstractionniste · optimiste · **volontaire** |
| **code** | attentif · minutieux · **obéissant** |
| *contexte* | *fidèle · discret · **méfiant*** — proposé ici, pas d'elle |

Ce ne sont pas des adjectifs de présentation : c'est ce qui ira dans chaque
invite système, et donc ce qui décidera de ce que chaque agent voit comme un
problème.

### Chaque triplet porte son propre contrepoids

C'est ce qui les rend tenables, et c'est visible en les lisant à l'envers :

- **vision** : *ambitieux* et *visionnaire* tirent loin du réel ; **observateur**
  ramène — il regarde ce qui *est*, pas ce qui devrait être. Sans lui, le rôle
  ne mesure plus la distance, il la rêve.
- **design** : *abstractionniste* et *optimiste* tirent vers l'élégant jamais
  construit ; **volontaire** oblige à trancher. Un design optimiste qui ne veut
  rien décider produit trois options et aucune décision.
- **code** : *attentif* et *minutieux* peuvent paralyser ; **obéissant** fait
  qu'on avance. Sans lui, l'agent de code débat au lieu d'écrire.
- *contexte* : *fidèle* et *discret* pourraient le rendre inutile à force de se
  taire ; **méfiant** lui donne son unique geste actif — douter de ses propres
  sources, et savoir qu'un index périmé ne se cite pas.

### La chaîne est un dégradé, de l'ambition à l'obéissance

Chaque étage est plus contraint que celui du dessus, exactement comme le
*pourquoi* est plus large que le *quoi*, plus large que le *comment*. Le
tempérament suit la forme de la chaîne au lieu de la contredire.

### Les frictions sont voulues, pas subies

- **vision ambitieux ↔ design volontaire** : le design doit découper l'ambition
  en quelque chose de décidable. C'est là que l'irréalisable devient un ticket.
- **design optimiste ↔ code attentif** : le code trouve ce que l'optimisme a
  manqué. C'est la friction la plus productive des trois, et la seule qui
  produise des faits.
- **code obéissant ↔ tout le reste** : il n'argumente pas sur le but.

### Et `obéissant` est exactement pourquoi la règle de mémoire compte

Un agent obéissant dont l'objectif a dérivé **obéit à la dérive**, fidèlement,
sans jamais lever la main. C'est pire qu'un agent têtu : un têtu résiste et se
fait voir, un obéissant exécute proprement la mauvaise chose.

La borne du §4 — ne couper la mémoire qu'à une manche de design — n'est donc
pas une optimisation de contexte. C'est **ce qui rend l'obéissance sûre**.

### La nuance qui manque à `obéissant`, et qu'il faut écrire

Obéissant **sur le but, pas sur les faits**. Un agent de code qui n'a pas le
droit de dire « ce que tu demandes ne marche pas, voici la mesure » est un
agent qui écrira du code cassé pour rester poli. Il ne discute pas l'objectif ;
il rapporte ce qu'il a constaté, et le design en fait ce qu'il veut.

C'est la même distinction que dans la porte des commandes : on ne confond pas
*ce qui est permis* avec *ce qui est vrai*.

## 3. L'agent de contexte est un souffleur, pas un bibliothécaire

Lucie : *« il fait constamment quelques recherches dans les sessions, rappelle
des choses relevantes à l'agent de code, ou fait des recherches dans le code
indexé si c'est prêt, ou même deux ou trois grep, écoute l'agent parler et
injecte au bon moment »*.

Ce qui compte dans cette phrase, c'est **écoute** et **au bon moment**. Un
agent qui répond quand on l'appelle est un outil de plus ; celui-là suit la
conversation et parle sans qu'on lui demande. C'est un rôle, pas une fonction.

**Ce qui existe déjà pour ça**, et qu'il n'y a pas à inventer :

- La **boîte de réception** (`Agent::read_inbox`) : un message arrive **entre
  deux tours, jamais au milieu d'un appel d'outil**. C'est exactement le canal
  d'injection, et sa frontière est déjà la bonne.
- Le **`Trace`** du catalogue, cherchable comme un document : « ce qu'on a déjà
  essayé » est une recherche, pas un mécanisme à part.
- La **fraîcheur de l'index par fichier** (`code_tools`, champ `stale`) : le
  souffleur peut savoir qu'il ne doit **pas** répondre depuis l'index, et faire
  un `grep` à la place. Sans ça, il affirmerait du périmé avec assurance — la
  faute la plus coûteuse pour un souffleur.

### `agentCanAnswer` — la porte avant la réponse

Lucie : *« il pourrait jauger si le contexte semble suffisant pour répondre, et
préférer mettre en pause l'agent de code pour relire / chercher l'historique
des conversations ou chercher dans le code. Un outil `agentCanAnswer`,
true/false + reason ; si false on le relance lui-même avec l'objectif de
chercher du contexte. »*

C'est le geste actif du souffleur, et c'est exactement là que son tempérament
**méfiant** sert : il doute d'abord de ce que l'autre a sous les yeux.

#### La sortie doit dire *quoi chercher*, pas seulement « non »

Un `false` sans cible relance le souffleur sans objectif, et il cherchera ce
qui lui vient. La même discipline que le verdict de commande s'applique :

```rust
struct PeutRepondre {
    oui: bool,
    raison: String,
    /// Ce qui manque, **et où le chercher**. Vide quand `oui`.
    manques: Vec<Manque>,
}

enum Manque {
    /// Dit plus tôt dans cette session, ou dans une autre.
    DejaDit { quoi: String },
    /// Dans le code — mais seulement si l'index est frais pour ces fichiers.
    DansLeCode { quoi: String, chemins: Vec<String> },
    /// Déjà tenté, et le résultat compte : `search(target="Trace")`.
    DejaTente { quoi: String },
    /// Hors d'ici. **Nouvelle dépendance** : on n'a aucun outil de recherche
    /// web aujourd'hui.
    Dehors { quoi: String },
}
```

Le relancement a alors un but vérifiable : chaque `Manque` est comblé ou ne
l'est pas, et le second tour de `agentCanAnswer` peut le dire.

#### Au moment de juger, il n'a **que** cet outil

Lucie : *« au moment où il décide, peut-être qu'il n'a que ce tool-là — sinon
il va déjà chercher le contexte pour répondre, il le cherchera deux fois »*.

Exact, et c'est une propriété de la **surface**, pas de la consigne. Un agent à
qui on donne `search`, `grep` et `read` s'en servira pour répondre
honnêtement à « peut-il répondre ? » — c'est même la façon sérieuse de le
savoir. Il trouve alors le contexte, rend `false`, se fait relancer, et
**cherche une seconde fois ce qu'il vient de trouver**.

On ne corrige pas ça par une instruction. On le corrige en ne donnant pas
l'outil. C'est le même principe que le `rerank=0` par défaut de `search` —
**ce qu'on offre décide de ce qui arrive**, pas ce qu'on demande.

#### Et même pas un outil : une sortie structurée

Lucie, en précisant : *« peut-être même pas un tool à ce moment-là, il a juste
une sortie structurée possible ; c'est après, quand on le relance, qu'il a tous
les tools de recherche »*.

C'est plus juste, et notre propre code le dit déjà. En-tête de `tools.rs` :

> *« La même donnée sert deux fois : mise dans le prompt et compilée en
> grammaire pour contraindre le décodage — **un appel d'outil n'est qu'une
> sortie structurée dont le schéma est fixé**. »*

Si tout ce qu'on veut est le verdict, il n'y a aucune raison de l'envelopper
dans un outil. Un appel d'outil est quelque chose que le modèle **choisit**
d'émettre : il peut aussi choisir de disserter, ou de ne rien appeler. Une
sortie structurée est la **forme de la réponse elle-même** — il n'y a rien
d'autre à produire.

Les deux phases deviennent donc franchement différentes :

| | outils | sortie |
|---|---|---|
| **juger** | *aucun* | `response_format` = le schéma de `PeutRepondre` |
| **chercher** | toute la lecture seule | libre, avec les manques pour objectif |

Le mécanisme existe : `GenOptions::with_response_format`, envoyé tel quel par
le client cloud (`openai_llm.rs`).

Et une troisième conséquence, qu'on n'avait pas vue : **la phase de jugement
n'est pas une boucle d'agent.** Sans outil, il n'y a pas de tour à enchaîner —
c'est un seul `generate`, avec un schéma minuscule. L'impôt par tour tombe à
une complétion courte, ce qui était la dernière objection sérieuse à mettre une
porte là.

#### Alors comment juger sans rien appeler ? En comparant

C'est la question qu'on se pose aussitôt, et elle a une réponse nette : le
souffleur n'a pas besoin de chercher, parce qu'il **voit déjà les deux côtés**.

- Ce qu'il sait, lui : toutes les conversations (§ privilège).
- Ce que l'agent de code voit : deux manches, bornées (§4).

Juger, c'est faire la **différence entre deux vues** — « une chose pertinente a
été dite, et elle n'est pas dans sa fenêtre ». Ce n'est pas une recherche, c'est
une comparaison, et elle ne coûte aucun aller-retour d'outil.

Il peut même produire un manque sans en connaître le contenu : *savoir qu'il y
a eu une longue conversation sur X il y a trois sessions* suffit à écrire
`Manque::DejaDit { quoi: "X" }`. Connaître l'existence, pas le contenu — c'est
précisément ce qu'un objectif de recherche demande.

Deux conséquences, et les deux sont bonnes :

1. **On ne cherche qu'une fois**, et seulement ce qui manque vraiment.
2. **La phase de jugement est bon marché** — pas d'appel d'outil, pas d'attente.
   L'impôt sur chaque tour redevient supportable, ce qui était la principale
   objection à mettre une porte là.

Et si le souffleur n'a rien dans sa propre vue qui suggère un manque, il répond
`oui`. Il ne « va pas voir au cas où » : ce serait rétablir par la porte de
derrière la double recherche qu'on vient de supprimer, et violer la règle du
défaut.

#### La pause existe déjà, et son pire cas est déjà attrapé

`PauseKind::WaitingForPeer` **fait une arête** dans le graphe d'attente, et
`Postures::deadlocks` trouve les cycles. Donc :

- « mettre en pause l'agent de code » = le souffleur pose une posture ;
- et si les deux s'attendent l'un l'autre, **c'est détecté**, sans plafond de
  tours et sans heuristique. Le mécanisme a été écrit le 26 août pour
  exactement cette situation.

Rien à inventer : le rôle se branche sur ce qui existe.

#### Le privilège, et son contrepoids

*« Il a accès à toutes les conversations de chaque agent, connaît toute la
hiérarchie, peut faire les outils de lecture qu'il veut. »* C'est le seul rôle
avec une vue **globale**.

C'est précisément pourquoi il ne décide rien. **Il lit tout, il ne tranche
rien.** Le jour où le souffleur commencerait à choisir l'objectif parce qu'il
en sait plus que les autres, il deviendrait un second design — sans en avoir le
tempérament ni l'artefact.

#### Le défaut doit être « oui », et c'est un argument d'asymétrie

Une porte qui s'exécute avant chaque tour est un impôt sur chaque tour. Et un
souffleur trouvera toujours quelque chose de plus à chercher : rien ne l'arrête
de l'intérieur.

**`agentCanAnswer` répond donc `oui` par défaut, et la charge de la preuve est
sur le blocage.** La raison n'est pas le confort :

- un **faux négatif** — on bloque alors qu'on pouvait répondre — est
  *invisible*, et il se compose : chaque pause en rend la suivante plus
  plausible ;
- un **faux positif** — on répond avec trop peu — se voit dans le résultat, et
  se rattrape au tour suivant.

Entre une erreur qui se voit et une erreur qui s'accumule en silence, on
choisit celle qui se voit. C'est la même règle que partout ici.

#### Trois pauses valent un signal de design

Un plafond par manche, et une escalade plutôt qu'un abandon : si le souffleur a
déjà cherché deux fois et que l'agent de code ne peut toujours pas répondre,
**le problème n'est probablement pas le contexte, c'est le ticket**. On remonte
au design, dont c'est le travail.

L'incapacité répétée à répondre est une information sur l'objectif, pas sur la
mémoire — et sans cette règle, le souffleur boucle en croyant bien faire.

## 4. La mémoire de l'agent de code, et la règle qui la borne

Lucie : *« l'agent de code n'a que peu de vrais tours complets en mémoire, les
anciens il voit un résumé assez court qu'il peut quand même consulter sur
commande »*.

**Ça existe.** `Absorb::Stale { max_chars, after_turns }` réduit ce qui est
vieux plutôt que ce qui est gros, `Session::recall(handle)` rend le contenu
entier sur demande, et le contenu complet n'est jamais perdu — *« réduire
l'historique n'est pas oublier »*.

Ce qui manque est la borne, et c'est l'idée la plus fine de la journée :

> **Les tours ne disparaissent jamais entre une parole de design et le code.**
> Pour qu'il y ait remise à zéro des tours trop vieux, il faut une parole du
> design — *un tour design*. Sinon l'agent de code cherche dans ses tours
> précédents et écrase l'objectif que le design lui avait donné.

**Pourquoi c'est juste.** L'objectif vit dans une parole de design. Si on coupe
ailleurs, l'agent de code reconstruit un objectif à partir de son propre
raisonnement — et un agent qui redérive son but de ses propres traces dérive.
Il ne se trompe pas d'un coup : il glisse, et chaque tour rend le glissement
plus cohérent avec le précédent.

### L'unité n'est pas le tour, c'est la manche

Lucie, en précisant : *« on peut se permettre les deux dernières paroles design
— enfin les deux derniers vrais tours où il y a eu : parole design, vraies
actions entreprises par l'agent code (donc tool calls), et réponse à nouveau
design »*.

Une **manche** est donc bornée par deux paroles de design et **contient au moins
une action réelle**. Ce dernier point n'est pas un détail : deux paroles de
design qui se suivent sans que rien ne soit tenté ne forment pas une manche, et
ne consomment donc pas de mémoire. Ce qui compte n'est pas qu'on ait parlé,
c'est qu'on ait essayé.

```rust
Absorb::DernieresManches { n: usize }  // n = 2 par défaut
```

### Pourquoi deux, et pas une

Avec une seule manche, l'agent de code voit l'objectif courant et rien d'autre.
Il ne sait pas ce qui vient d'être tenté et écarté — donc il le repropose. Avec
deux, il voit **l'objectif précédent et comment il a bougé** : c'est la
trajectoire qui rend un objectif stable, pas sa simple présence.

Trois coûteraient plus pour n'ajouter qu'un écho.

### Borner par la structure, pas par la taille

C'est là que la règle se distingue de ce que font les agents répandus, et
Lucie le dit sans détour : *« ça reste BEAUCOUP plus léger que ce que font
actuellement les agents populaires — attendre 1 M de contexte par exemple »*.

Un plafond en jetons compacte **quand on a mal** : il attend la douleur, et le
moment de la coupe est décidé par un accident — quel outil a rendu quoi. Une
borne en manches coupe **là où le sens le permet**, et le poids d'un résultat
d'outil ne déplace jamais la frontière.

Les deux mécanismes ne se remplacent pas, ils se composent :

- `Absorb::Stale` réduit **à l'intérieur** d'une manche — un gros résultat
  ancien devient une ligne, sans que la manche bouge.
- `DernieresManches` borne **entre** les manches.

Sans le premier, une manche où l'agent fait deux cents appels d'outils pèserait
autant qu'un contexte d'un million. Sans le second, on couperait au hasard.

### La parole de design ne se réduit jamais

C'est ce qui rend le reste sûr. Une manche peut être compactée agressivement à
l'intérieur — mais les **paroles de design elles-mêmes sont épinglées**, jamais
réduites, même quand leur manche sort de la fenêtre. L'objectif ne peut donc
pas disparaître par accumulation, quel que soit le silence du design.

C'est aussi ce qui protège du cas dégénéré : si le design se tait longtemps, il
n'y a qu'une manche ouverte et la mémoire grossirait — mais ce qui grossit est
réductible, et ce qui ne l'est pas tient en quelques lignes.

### L'effet de bord qu'on veut

**Le design a intérêt à parler.** Une session sans parole de design ne peut pas
clore de manche, donc ne peut pas oublier, donc coûte. Le cadran pousse dans le
bon sens sans qu'on ait à l'imposer.

Et la règle reste **mécaniquement vérifiable** : on connaît le rôle de chaque
tour et on sait lesquels portent des appels d'outils, donc on sait où on a le
droit de couper. Pas de jugement, pas de modèle — une borne.

## 5. Ce que chaque rôle **possède**

Un rôle sans artefact est un avis. Chacun tient donc quelque chose :

| rôle | ce qu'il tient |
|---|---|
| design | le **document de design**, et le verdict « je valide » |
| contexte | les **résumés** : quoi garder, sous quelle forme, et le rappel |
| code | le **code**, les tests, et le journal de ce qu'il a lancé |

La fin de session — le troisième moment humain — appartient au **contexte** :
c'est lui qui décide quoi ranger et où. Rapport de session, fiche de
progression, ce qui reste, vidage de connaissance sur les scripts et les tests,
vidage sur l'architecture. Ce sont des gabarits, donc du ressort du catalogue
(doc 08) : `place` un gabarit `rapport-de-session`, remplis-le, `adopt` ce qui
a marché.

## 6. Les deux façons dont ça peut être creux

À écrire tant qu'on peut encore décider :

**Un agent de design qui refuse sans raison vérifiable est un tampon à
l'envers.** « C'est naïf » n'est pas un refus, c'est un avis. Un refus doit
nommer ce qui manque — un cas non traité, une hypothèse non dite, un mécanisme
déjà présent qu'on réinvente. Le contrôle honnête, c'est que **le refus soit
plus long que l'approbation** : si refuser coûte moins cher que valider,
l'agent refusera tout.

**Un souffleur qui parle au mauvais moment est du bruit**, et le bruit finit
par se faire ignorer — par un agent comme par un humain. La borne à tenir n'est
pas « injecte ce qui est pertinent » mais « **ne parle que si ça change le tour
suivant** ». Ça se mesure : on peut regarder si le tour d'après cite ce qui a
été injecté.

## 7. Ce qui existe, et ce qui manque

| | |
|---|---|
| Boîte de réception entre les tours | **existe** (`Agent::read_inbox`) |
| Postures : qui se tait envers qui, interblocage | **existe** (`postures.rs`) |
| Réduction de l'historique, rappel sur commande | **existe** (`Absorb`, `Session::recall`) |
| Fraîcheur de l'index par fichier | **existe** (`stale`) |
| Traces cherchables comme un document | **existe** (`search(target="Trace")`) |
| La porte des commandes, et ses modes | **existe** (`commande.rs`, 30 août) |
| Une notion de **rôle** | manque |
| `Absorb::DernieresManches` — la coupe à la manche de design | manque |
| Le souffleur : quand parler, et `agentCanAnswer` | manque |
| Un outil de recherche web | manque — nouvelle dépendance |
| Les gabarits de fin de session | manque |

La machinerie des agents qui se parlent est là depuis le 26 août. Ce qui manque
n'est pas de la plomberie : ce sont **les rôles, et ce que chacun possède**.
