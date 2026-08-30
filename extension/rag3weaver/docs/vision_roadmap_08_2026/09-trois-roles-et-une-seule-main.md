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
| Le souffleur : quand parler | manque |
| Les gabarits de fin de session | manque |

La machinerie des agents qui se parlent est là depuis le 26 août. Ce qui manque
n'est pas de la plomberie : ce sont **les rôles, et ce que chacun possède**.
