# Une mémoire longue pour des agents — proposition

3 octobre 2026. Réponse à la [vision](extension/rag3weaver/visions/2026-10-03-20h37-memoire-longue.md)
et aux [décisions accrochées au code](extension/rag3weaver/visions/2026-10-03-20h37-le-produit-code.md) §3.
Conception seulement : rien n'est codé, aucune mesure n'est inventée.

Le fil tient en une phrase : **la mémoire longue n'a presque pas besoin de
moteur neuf. Elle a besoin que des pièces déjà livrées servent à autre chose
que ce pour quoi elles ont été faites** — et le gabarit `notebook` montre
qu'une bonne partie se déclare déjà sans code.

> **Deux corrections d'entrée, par honnêteté.** En préparant ceci j'ai cru
> deux choses fausses. La **mise de côté existe** — un module entier,
> `src/catalog/aside.rs`, livré le 3 octobre, avec son annulation ; je l'avais
> vue refusée sur une branche plus ancienne. Et **une entité de note existe
> déjà**, dans `templates/backends/notebook/`, avec sa machine à états. Les
> deux changent cette proposition : elle part de l'existant au lieu de le
> redemander.

---

## 1. L'entité `Memory`

### 1.0 On ne part pas de zéro : `notebook` est le brouillon

`templates/backends/notebook/backend.json` déclare déjà une entité `Note` :
`key` en titre et en `hashsafe`, `text` en contenu, `bm25 + vector`, une
machine à états `working → reviewed → working`, et des champs écrits
automatiquement (`created_at`, `updated_at`, `revision`, `reviewed_at`). Avec
`put_note`, `get_note`, `search_notes`, et une entité `Snapshot` immuable.

C'est la preuve de la thèse : **tout ça s'est déclaré sans une ligne de Rust.**
`Memory` est `Note` qui grandit. Ce qu'elle ajoute, et rien de plus :
l'origine, la portée, l'ancre engendrée, les deux signaux de « à revoir », et
le nœud de décision.

### 1.1 Les champs

| Champ | Type | Rôle |
|---|---|---|
| `claim` | String, **titre**, `hashsafe` | l'affirmation, courte |
| `why` | Text, **contenu** | le pourquoi ; c'est lui qui se découpe et s'embarque |
| `kind` | String, `choices` | `fact` / `preference` / `decision` / `pointer` |
| `origin` | String, `choices` | `said` / `inferred` |
| `said_as` | Text | les mots de la personne, quand `origin = said` |
| `reach` | String, `choices` | `person` / `project` / `team` |
| `state` | String | le champ de la machine à états (§1.3) |
| `at`, `revision` | écrits par `writes` | la date et la révision vue, sans code |

**Ce qui s'embarque : `why`.** Et `claim` l'est **gratuitement** : le titre est
préfixé à chaque chunk pour l'embarquement (256 caractères sur 1 500). Une
affirmation courte entre donc dans le vecteur par le préfixe — inutile de
l'embarquer à part.

**Ce qui se cherche par mots : `claim` et `why`**, par l'index plein texte de
la table **parente**. Donc `bm25 + vector`, comme `Note`.

**Ce qui se filtre :** `kind`, `origin`, `reach`, `state`, `at`. La date passe
par les bornes locales du 27 août (`local_range`), jamais par un préfixe de
chaîne — filtrer une date par préfixe est faux du décalage de fuseau.

**L'identité : `hashsafe: ["claim", "reach"]`.** Une même affirmation dans deux
portées sont deux mémoires ; la même dans la même portée est **la même ligne**.
Donc une redite ré-ingérée ne crée pas de doublon **même si le protocole de §2
est contourné**. C'est le filet sous le filet, et c'est ce que `Note` fait déjà
avec `key`.

**Le coût se demande au lieu de se deviner** (`EntityConfig::cost_for`, 18
septembre) : un `why` d'un paragraphe fait un chunk, donc dix mille mémoires
font dix mille embarquements.

### 1.1 bis. Une dette nommée : la portée `person` n'est pas rappelée

Le crochet après outil (session recherche, 3 octobre) **n'affiche jamais** la
portée `person`, et c'est une décision prise ensemble plutôt qu'un oubli : le
protocole du backend n'a **aucune identité d'appelant**, donc filtrer sur
« la personne » serait un faux filtrage — les mémoires d'une personne
sortiraient à tout le monde.

Mais la décision crée un piège, et il faut l'écrire : une mémoire de portée
`person` est alors **écrite, stockée, cherchable, et ignorée par le rappel
automatique**. Invisible à l'endroit même où elle devait servir. C'est la
famille qu'on traque partout dans ce dépôt — *écrire quelque chose que rien ne
lira* — et elle y a déjà deux occurrences : `_absent_since`, posée par la
synchronisation et relue par personne en production, et le crochet muet sans
compteurs.

**Ce qu'on fait** : on accepte la portée, et **l'écriture le dit** — « cette
portée ne sera pas rappelée automatiquement tant que le protocole n'a pas
d'identité d'appelant ». Plutôt que de la refuser, parce que refuser
interdirait d'enregistrer une préférence que la personne vient d'énoncer, ce
qui est le cas d'usage le plus naturel du produit ; et la mémoire reste
retrouvable par un `recall` adressé.

**La condition de sortie, parce qu'une dette qui ne dit pas comment elle se
solde est une dette oubliée** : le jour où le protocole porte une identité
d'appelant, le crochet **filtre** au lieu d'exclure, l'avertissement
disparaît, et rien d'autre ne change — la mécanique de section est déjà
prévue pour ça.

### 1.2 Les relations : comment une ancre vise n'importe quoi

Le moteur est net : `register_relation_with` exige un couple `(depuis, vers)`,
et ré-enregistrer le même nom avec un autre couple est une **erreur dure**. Une
ancre « vers n'importe quelle entité » n'est donc pas **une** relation.

**Mais la traversée, elle, est déjà polymorphe** : `FetchRelatedNode` fait un
`MATCH` sur des nœuds **non étiquetés** et lit `label(m)` à l'exécution. Le
moteur sait donc rendre une cible de type variable ; c'est la *déclaration* qui
ne le permet pas. Tout le problème est là, et il est plus petit qu'il n'y
paraît.

| Voie | Gain | Perte |
|---|---|---|
| Une relation par type de cible, à la main | des arêtes vraies | N déclarations à tenir |
| Une entité pivot `Anchor { entité, uuid }` | une seule déclaration | **ce n'est plus une arête** : plus de parcours, plus de synchronisation de lien, une jointure sur une chaîne |
| **`ANCHORED_TO` engendrée à l'enregistrement** | des arêtes vraies, une déclaration à écrire | du code au registre |

**Je retiens la troisième** : une entité déclare `anchorable: true`, et
`register_entity` enregistre alors `ANCHORED_TO: Memory → <elle>`. Les N
relations existent dans le schéma — donc parcours, synchronisation de liens et
« à revoir » fonctionnent — mais personne ne les écrit, et la généricité vit
dans la **déclaration**. C'est la règle de la maison : une organisation
nouvelle se décrit dans `EntityConfig`, jamais en dur.

Le pivot est la fausse bonne idée du lot : il a l'air plus générique et il jette
exactement ce pour quoi on a un graphe. Le code a d'ailleurs déjà choisi comme
moi deux fois — `Symbol` est une entité de rendez-vous, pas un pivot à champs.

**Une contrainte que le moteur impose, et qu'il faut écrire** : `find_relation`
rend la **première** relation reliant deux entités, en itérant une table de
hachage — l'ordre n'est pas déterministe. Donc **au plus une relation par
couple `(depuis, vers)`** tant qu'on veut filtrer d'une entité sur les champs
d'une autre. Concrètement : pas de `MENTIONS: Memory → Subject` à côté
d'`ANCHORED_TO: Memory → Subject`. Si les deux sont voulues un jour, la
seconde devra passer par une entité intermédiaire.

### 1.3 L'état est une machine à états déclarée

```
field: "state", initial: "current"
transitions:
  review     current   -> to_review     (l'ancre a changé ou a disparu)
  confirm    to_review -> current       (regardée, encore vraie)
  supersede  current   -> superseded     (une neuve la remplace)
  supersede_reviewed  to_review -> superseded
  set_aside  current   -> set_aside
  set_aside_reviewed  to_review -> set_aside
```

Quatre choses gratuites :

- une transition non déclarée **ne passe pas**, aux **deux** chemins
  d'écriture — la mise à jour et l'ingestion ;
- un état inatteignable est refusé **à la déclaration**, avant la première
  ligne ;
- `superseded` et `set_aside` sont terminaux **par construction** : aucun champ
  « final » à tenir en accord, l'absence de transition sortante le dit ;
- le refus **nomme ce qui aurait été permis**, donc un agent n'a pas à relire
  la déclaration pour comprendre un mur.

**Les ancres sont nues, et ce n'est pas un choix de style.** La vision propose
de mettre l'origine et la date en propriétés d'arête. Trois faits du moteur
l'interdisent :

1. une propriété d'arête **ne se cherche ni ne se filtre** — ni
   `FetchRelatedNode`, qui traverse une arête anonyme sans rendre ses
   propriétés, ni `FilterParser`, qui n'adresse que des champs de nœuds ;
2. `RelationBatchNode` **refuse** une relation à propriétés, en disant quoi
   faire : *« snapshot links require a property-free relation; use an entry
   entity for payload »* — donc une ancre à propriétés ne se synchronise pas,
   et c'est d'elle que dépend le « à revoir » ;
3. le seul lecteur de propriété d'arête du crate est du Cypher écrit à la
   main, hors du dataflow.

Et de toute façon origine et date sont des propriétés de **la mémoire** : elle
n'en a qu'une, quel que soit le nombre de choses qu'elle touche. La seule
propriété vraiment par lien serait la révision de *cette* ancre-là ; je
préfère dire qu'une mémoire qui a besoin de deux ancres à deux révisions est
**deux mémoires**.

`REPLACES: Memory → Memory` est nue aussi : la raison vit sur la neuve.

---

## 2. Les quatre verbes

### 2.1 Ce qui s'écrit avec l'existant

Les cinquante-huit nœuds visibles à un graphe-outil suffisent, sauf pour un
point.

| Verbe | Forme | Nœud neuf ? |
|---|---|---|
| `recall` | `SearchSourceNode` + les trois signaux + `FuseResultsNode` + filtre `state` + `RenderResultsNode` ; par ancre, `FetchRelatedNode` | non — c'est `search_structured.mmd` avec un filtre |
| `revise` | `InsertRecordNode` (la neuve) + `LinkRecordNode` (`REPLACES`) + `UpdateRecordNode` (l'ancienne) | non |
| `forget` | `UpdateRecordNode` vers `set_aside`, et `aside.rs` fait le reste | non |
| `remember` | chercher, montrer les proches, **exiger le choix** | **oui** : le nœud de décision (§7.5) |

Et la mise de côté est **déjà** ce qu'il faut : `set_aside` copie la ligne *et
les vecteurs de ses chunks*, `take_from_aside` les rend sans réembarquer si
l'identité et l'empreinte n'ont pas bougé, la purge est bornée par lots, et
`undo_snapshot_finish` annule en bloc tant que les copies vivent. `forget`
n'a rien à inventer.

### 2.2 `remember` : un seul verbe, et le premier appel est un refus qui montre

J'ai d'abord proposé deux verbes — `remember` qui propose, `apply_memory` qui
écrit — sur le modèle de la coupure plan / application de la synchronisation.
**L'inventaire du crate me fait changer d'avis, et pour une raison mesurée.**

> `search_expand` a été fusionné dans `search` le 28 août : **zéro appel sur
> quarante**, parce qu'« un agent qui hésite entre deux outils proches en
> prend zéro ».

C'est une mesure, pas une opinion, et elle vise exactement ce que j'allais
faire. S'y ajoute une doctrine écrite dans `place.mmd` : *« un outil qui
confirme sans montrer oblige son appelant à une seconde requête »*. Et un
précédent qui refroidit : le `needs_confirmation` d'`estimate` est **purement
informatif**, aucun code ne bloque dessus.

Donc : **un seul verbe, appelé deux fois, et le premier appel est un refus qui
porte le contenu.**

```
remember(claim, why, kind, reach, anchor?)          -- sans « choice »
  -> REFUS, qui contient :
     les proches (affirmation, date, état, distance),
     un identifiant de proposition,
     la liste fermée des suites, et l'appel exact à refaire

remember(…, proposal, choice)                        -- « choice » dans une liste fermée
  -> écrit une fois
```

Pourquoi un modèle faible suivra :

1. **Un seul nom d'outil.** Aucune hésitation, donc le défaut mesuré de
   `search_expand` ne s'applique pas.
2. **La liste des choix est fermée** (`%% choices: choice = complete |
   replace | create`). Le moteur refuse une valeur hors liste **avec la liste
   dans l'erreur** (`check_choices`, `kind = "bad_choice"`), et un décodage
   contraint ne peut pas en inventer.
3. **`choice` est obligatoire** (`%% param: choice string!`), donc son absence
   est un refus du moteur, pas un oubli silencieux.
4. **Le refus montre.** Il ne dit pas « confirme », il donne les proches et
   l'appel à refaire, arguments remplis. C'est la doctrine de `place.mmd`
   respectée, pas contournée.
5. **Une proposition se périme et ne s'applique qu'une fois** — elle retient
   l'état des proches ; l'appliquer après qu'ils ont bougé est refusé en
   disant de recommencer. Même garde que le plan périmé de la synchronisation,
   qui est éprouvée.
6. **Le premier appel n'écrit rien.** Un modèle qui s'arrête là ne crée
   **aucun désordre** : l'échec est le silence, pas un doublon.

Le sixième est le choix de conception, et son prix doit être nommé : un modèle
qui ne confirme jamais laisse la mémoire vide, et ça ne se voit pas. **Donc
« propositions jamais appliquées » est une des mesures du banc** (§4.2). Un
échec silencieux qu'on mesure cesse d'être silencieux.

---

## 3. « À revoir » : deux signaux, pas un

La vision attribue le « à revoir » à la synchronisation. **C'est juste à
moitié**, et la moitié manquante est la plus fréquente.

> La synchronisation sait ce qui **manque**. C'est l'**ingestion** qui sait ce
> qui a **changé**.

- **Une ancre qui disparaît** : la fin de session la connaît — son rapport
  liste `removed`, `transitioned`, `kept`, et `onMissing: {transition}` existe.
- **Une ancre qui change sans disparaître** — un fichier édité, une fonction
  réécrite, le cas courant : la fin ne la voit pas. Mais `split_unchanged` la
  connaît : séparer ce qui a changé de ce qui n'a pas changé est littéralement
  son travail, et il rend déjà les deux ensembles.

La mécanique, petite :

1. l'ingestion et la fin **émettent** ce qui a bougé (uuids, entité) — la fin
   le sait déjà, l'ingestion doit le dire ;
2. un **réacteur** (`each` / `batch`, livrés) écoute, remonte les
   `ANCHORED_TO` entrantes par `FetchRelatedNode` en direction `Incoming`, et
   transitionne par `review` ;
3. la transition passe par la garde, donc elle ne peut pas sauter un état.

Deux comportements qui tombent juste sans rien coder :

- **`review` n'écrase pas `superseded` ni `set_aside`** : ces états sont
  terminaux, la transition n'existe pas depuis eux, la garde refuse et la ligne
  est **nommée** dans le rapport ;
- une mémoire déjà `to_review` dont l'ancre rechange reste `to_review` :
  `depuis == vers` n'est pas un passage, la garde le laisse passer sans rien
  écrire.

**Et une pièce à ne pas confondre avec celle-là.** `_absent_since` existe
(schéma v8), est posée sur les lignes transitionnées par une fin, et effacée
dès qu'une ligne reparaît — par trois chemins. Mais **personne ne la lit en
production**, et comme elle est préfixée `_` elle n'est pas un champ déclaré,
donc **`FilterParser` ne peut pas l'adresser**. Elle ne peut donc pas servir
de critère à `recall` tant qu'elle reste interne. C'est une information utile
telle quelle — « cet état vient d'une absence, pas de quelqu'un » — et c'est
la mémoire qui lui donnerait son premier lecteur, à condition de la déclarer.

---

## 4. Le banc

### 4.1 Le scénario, cinq sessions, sans modèle d'abord

« L'agent » est un script qui appelle les verbes avec des choix fixés. La
vérité est donc connue **par construction** — ce qui sert deux fois (§7.5).

| Session | Ce qui arrive | Ce qu'on attend |
|---|---|---|
| 1 | trois faits, deux ancrés, un global | trois mémoires, zéro doublon |
| 2 | une correction d'un fait | la neuve `current`, l'ancienne `superseded`, le lien `REPLACES` |
| 3 | une redite, dite autrement | le proche est trouvé ; `complete` ou refus, **pas** une quatrième ligne |
| 4 | l'ancre du fait 1 est **éditée** ; celle du fait 2 **disparaît** | les deux passent `to_review`, par les **deux** signaux de §3 |
| 5 | une question qui touche le fait corrigé | la `current` revient, la `superseded` **non** |

La session 4 est la seule qui vaille vraiment : elle est le seul endroit où les
deux signaux se distinguent, et c'est le point que la vision confond.

### 4.2 Les mesures

Les quatre de la vision, plus une que la conception rend nécessaire :

1. **doublons créés** ;
2. **rappels utiles** — la mémoire attendue est dans ce qui revient ;
3. **périmés servis comme vrais** — une `superseded` ou une `to_review` rendue
   sans sa marque. **La plus importante** : c'est le défaut qui fait perdre
   confiance, et une mémoire en qui on n'a pas confiance ne sert à rien ;
4. **contradictions vues** ;
5. **propositions jamais appliquées** — l'échec silencieux de §2.2.

Puis le même scénario avec un modèle, les mêmes mesures : la tuyauterie étant
déjà prouvée, l'écart mesure le modèle et rien d'autre. C'est la règle de
Lucie, appliquée telle quelle.

**Le scénario est séquentiel exprès** : les écritures parallèles ne sont pas
finies, et un banc ne doit pas reposer sur ce qui n'est pas livré.

---

## 5. Ce qui manque au moteur, dans l'ordre

| # | Ce qu'il faut | Pourquoi là | Estimation |
|---|---|---|---|
| 1 | `Memory` **en partant de `notebook`** : les champs, `anchorable`, la machine à états, les gabarits des quatre verbes | c'est de la déclaration, pas du moteur | ½ jour |
| 2 | `ANCHORED_TO` engendrée à l'enregistrement d'une entité `anchorable` | la seule pièce de moteur de §1.2 | 2 heures |
| 3 | l'ingestion **émet** son ensemble changé | la moitié manquante du « à revoir » | ½ jour |
| 4 | le réacteur qui transitionne les mémoires ancrées | il sert les deux signaux | 2 heures après 3 |
| 5 | le **nœud de décision**, étage classement d'abord (§7.5) | `remember` et les sujets en dépendent | 1 jour |
| 6 | déclarer `_absent_since` pour qu'elle soit filtrable, si `recall` doit s'en servir | sinon elle reste invisible | 2 heures |
| 7 | le crochet après outil | **pas à moi** : session recherche, `backend.rs` | — |
| 8 | le jardinier, la détection de contradiction | ils vivent de tout ce qui précède | après |

**Ce qui ne figure pas dans cette liste, et que je croyais devoir y mettre :
la mise de côté.** Elle est livrée, avec son annulation.

---

## 6. Mes désaccords avec la vision

**a. L'origine et la date ne sont pas des propriétés d'arête** (§1.3). Trois
faits du moteur le disent, dont un refus explicite de `RelationBatchNode` qui
indique même le remède.

**b. `session` n'est pas une portée.** La règle 6 la liste, puis dit que ce qui
ne vaut que pour la conversation en cours ne s'écrit pas. Une portée qu'on
refuse d'écrire n'est pas une portée : trois niveaux, pas quatre.

**c. Le « à revoir » a deux sources, pas une** (§3). C'est le désaccord qui
change le plus de choses dans le travail.

**d. La règle 7 est mon problème d'août qui revient par la fenêtre.** « Le
rappel est une recherche, pas un chargement, avec un budget » — mais un
crochet qui déverse les `why` dans chaque résultat d'outil, c'est « tout est
chargé tout le temps » sous un autre nom. La forme est déjà livrée : le crochet
rend **les affirmations seules**, et le `why` se demande par un renvoi. C'est
`absorb` et `recall` de la session.

**e. « Plusieurs agents qui notent » n'est pas dans ce que le moteur donne
déjà.** La vision le range là en renvoyant aux écritures parallèles « en
cours ». Elles ne le sont pas. Tant qu'elles ne le sont pas, deux agents qui
notent en même temps ne sont pas un cas soutenu.

**f. Et un désaccord avec moi-même, pour mémoire.** J'avais écrit que la mise
de côté n'existait pas. Elle existe depuis le 3 octobre, avec plus que ce que
la vision promettait — les vecteurs sont gardés et rendus sans réembarquement.
J'avais vérifié sur une branche antérieure et conclu une absence d'un endroit
où elle n'était pas encore. La règle que j'en retire, et qui vaut pour tout ce
document : **avant d'annoncer qu'une chose manque, vérifier à la source la plus
récente.**

---

## 7. Les sujets abstraits

Demande de Lucie : une note sur une stratégie, un principe, une préférence, qui
n'a **aucun** fichier ni entité où s'accrocher.

### 7.1 Le sujet est une entité de rendez-vous — mais pas la config de `Symbol`

La piste est juste et le précédent est le bon : `Symbol` est le point de
rendez-vous des noms, et c'est exactement pour permettre à n'importe quel scope
de viser une cible que la relation ne connaît pas encore.

**Mais sa configuration ne se copie pas.** `Symbol` est `BM25` et
`chunked: false` parce qu'un nom de vingt caractères n'a rien à gagner d'un
vecteur — et le défaut `HYBRID` lui avait fait calculer 3 275 embarquements que
personne n'avait voulus. Un sujet, lui, porte une **description** : il a donc
quelque chose à embarquer, donc il lui faut des chunks, car une entité qui
déclare un signal vecteur **sans** chunks est refusée à la déclaration.

`Subject` est donc une petite config à la `Memory`, pas à la `Symbol` :
`name` en titre cherché par mots, `about` en contenu embarqué, `at` et `state`.
Et `Subject` déclare `anchorable: true` : l'ancre d'une note abstraite est
alors **une ancre comme les autres**. C'est le test que le mécanisme de §1.2
est vraiment générique — s'il fallait un cas particulier pour les sujets, il ne
le serait pas.

`RELATES_TO: Subject → Subject` et les ancres vers du concret viennent plus
tard sans migration : `register_relation_with` fait un `ALTER` additif.

### 7.2 Le bordel des étiquettes : le même protocole, pas un second

Un sujet se choisit parmi les proches à l'écriture — c'est-à-dire **le
protocole de §2.2 avec un autre critère**. Je ne propose pas un second
dispositif anti-doublon : la force de l'idée de Lucie est que c'est la même
décision. Un seul nœud, un seul protocole, deux critères.

**La découpe de cette décision n'est pas arrêtée, et elle ne doit pas l'être
par ce document** (voir §7.7). La mesure dit que « variante » est l'étiquette
que tout le monde rate ; elle ne dit pas quelle découpe la remplace. Un choix
à deux, deux oui/non enchaînés, un « sous-sujet de » : trois formes possibles,
et on n'a pas de quoi trancher.

Donc la seule chose que la conception fixe, c'est que **le critère et la liste
fermée sont des données du graphe**, pas du code. Changer la découpe doit être
une ligne de gabarit, pas un correctif.

### 7.3 Ce qui fait vieillir une note sans ancre concrète

- **La date**, déjà là.
- **Une note plus récente sur le même sujet qui la contredit** — mais
  « contredit » demande la détection, la pièce la plus dure du produit. Un
  substitut mécanique, disponible tout de suite : *sur le même sujet, même
  `kind`, même portée, une note plus récente fait passer l'ancienne en
  `to_review`* — pas en `superseded`, puisqu'on n'a rien prouvé. Aucun modèle,
  et l'erreur penche vers « regarde ça » plutôt que « j'ai tranché ».

### 7.4 Des sujets peuvent-ils émerger ?

Oui, et le moteur a la pièce : des grappes de notes proches. **Mais je ne les
nommerais pas automatiquement.** Une grappe sans nom est une grappe, pas un
sujet ; le jardinier **propose** un nom, quelqu'un le valide. C'est encore
« proposer, ne pas fusionner ».

### 7.5 Le nœud de décision générique

**La distinction qui décide de tout : un reranker donne un ordre, pas un
verdict.** Il note des paires (question, candidat), donc il répond très bien à
« lequel est le plus proche » et **pas du tout** à « est-ce la même chose ». Or
`même / complète / contredit / neuve` est une classification. Confondre les
deux est le piège de ce genre de nœud.

Donc **deux étages** — et c'est ce qui permet de livrer utile tout de suite :

| Étage | Ce qu'il fait | Ce qu'on a |
|---|---|---|
| 1. classement | ordonner les candidats par proximité au critère | le trait `Reranker`, son mock, trois rerankers burn, et `RerankNode` déjà dans le dataflow |
| 2. verdict | un choix dans une liste fermée, avec un score | **rien** : c'est l'étage à brancher |

L'étage 1 existe **et il est déjà un nœud**. Le nœud de décision peut donc
naître avec un étage 2 réglé sur « demander toujours » — le défaut sûr — et
gagner son modèle ensuite sans que la forme bouge.

**Ce que le nœud exige du modèle.** Je ne choisis pas le modèle ; je dis les
conditions. Les faits vérifiés par la session optimiseur
([son tableau](../optimiseur/3-octobre-2026-20h55/01-les-modeles-de-decision-verifies-a-la-source.md),
aucune mesure encore) les rendent plus précises qu'à ma première écriture, et
font apparaître une tension qu'il faut nommer.

1. **Un choix parmi des options nommées, pas un oui/non.** Les trois primitives
   communes aux candidats sont le oui/non, le choix parmi N options décrites, et
   un score ordonné — critère et options donnés **à l'appel**, rien n'est figé à
   l'entraînement. On prend la deuxième, et pour une raison que l'un des auteurs
   écrit lui-même : son oui/non « peut suivre ses étiquettes d'options plutôt que
   l'état », et son score ordonné est sa primitive la plus faible. Des options
   nommées — « même sujet », « variante », « sujet nouveau » — sont donc le
   terrain le plus sûr.
2. **Une liste fermée, tenue hors de sa bonne volonté** : décodage contraint,
   lecture des logits des choix, ou un schéma d'énumération côté API. Jamais du
   texte libre analysé après coup. Le moteur a déjà la forme —
   `Choices::Fixed` devient un `enum` dans le schéma JSON, et une valeur hors
   liste est refusée **avec la liste**.
3. **Un budget d'options déclaré par le modèle, pas découvert par l'échec.**
   Les limites relevées vont de 16 à 255 par passe, et les petits encodeurs
   recommandent d'en rester à une dizaine. C'est précisément le travail de
   l'étage 1 : le classement **tronque** la liste au budget du modèle de
   verdict. Le nœud doit donc lire ce budget comme une propriété du modèle
   branché.
4. **Un nombre d'options fixe, et c'est une conséquence qu'on ne voit pas tout
   de suite.** La probabilité est partout un softmax sur les options, divisé par
   une température. Un softmax dont le **dénominateur change** avec le nombre de
   candidats ne rend pas des scores comparables d'un appel à l'autre : deux
   seuils posés sur trois candidats ne veulent plus rien dire sur sept. Donc
   l'étage 1 ne rend pas « les proches », il rend **toujours le même nombre** de
   candidats, complété si besoin. Sans ça, les zones se déplacent sous nos pieds.
5. **L'abstention comme sortie de première classe**, pas comme un score bas.
   Un modèle qui sait dire « je ne sais pas » vaut mieux qu'un seuil sur une
   assurance. S'il ne sait pas s'abstenir, la zone incertaine *est* son
   abstention.
6. **Déterminisme** : même entrée, même sortie, sinon les mesures du banc ne
   veulent rien dire.
7. **Le français est une exigence, pas un confort.** Un seul des six candidats
   se déclare multilingue. Nos mémoires et nos documents sont en français : un
   modèle « English only » ne lit pas la matière du produit.
8. **Atteignable depuis notre exécution** : ONNX pour la voie encodeur, ou
   `llama-server` pour la voie à quelques milliards de paramètres. Un candidat
   livré avec un runtime Go seul n'est pas branchable chez nous — c'est un
   critère de sélection, pas un détail d'emballage.

### 7.6 La tension à nommer, et comment on en sort

Les deux exigences les plus dures — un score comparable (4) et le français
(7) — sont aujourd'hui satisfaites par des **ensembles disjoints** de
candidats : le seul qui se déclare multilingue est aussi celui qui annonce ses
températures laissées à 1,0, donc **non calibré**, et un autre dit que sa
calibration n'a jamais été rapportée.

Conclusion, et elle change la conception plutôt que d'attendre un modèle
parfait :

> **Le nœud n'exige pas un modèle calibré. Il exige que la calibration nous
> appartienne.**

Les deux seuils ne sont pas des constantes lues dans une carte de modèle : ce
sont des **réglages par critère et par modèle**, ajustés sur un jeu de paires
dont on connaît la réponse. Et ce jeu, le banc scripté de §4 le produit
gratuitement, puisque dans un scénario écrit on sait lesquelles paires sont la
même chose.

D'où une mesure de plus, propre au nœud et facile : **l'invariance aux
étiquettes.** On permute l'ordre et la formulation des options et le verdict
ne doit pas changer. C'est exactement le défaut qu'un auteur avoue — suivre
les étiquettes au lieu de l'état — et il se mesure en une passe. Un modèle qui
échoue là ne peut pas porter la décision, quelle que soit son exactitude
annoncée.

Dernier point, opérationnel plutôt que technique : la voie à quelques
milliards de paramètres passe par `llama-server` et **partage la carte** avec
le service d'embarquement et le modèle de dialogue. Ce n'est pas qu'une
latence, c'est une contention — et sur ce poste, un démon sur la carte
intégrée fige l'écran. La voie encodeur, à 150–420 millions de paramètres en
une passe, n'a pas ce défaut. C'est un argument qui ne figure dans aucun
tableau de performances.

### 7.7 Ce que la mesure a tranché

La session optimiseur a mesuré
([son document](../optimiseur/3-octobre-2026-20h55/02-les-modeles-de-decision-mesures.md)).
Quatre choses sont désormais établies, et deux changent la conception.

**« Un ordre n'est pas un verdict » est vérifié sur nos phrases, avec un
chiffre.** Le cosinus et le reranker trouvent le bon candidat parmi dix à
chaque fois ; mais le cosinus note en moyenne une **contradiction à 0,85** et
une **redite à 0,83**. Deux relations opposées, deux notes qu'un seuil ne
sépare pas. Les deux étages ne sont donc pas une précaution d'architecte,
c'est la seule forme qui marche.

**Les seuils par critère passent de prudents à nécessaires.** Un seul modèle
remplit la condition de score comparable, et chez lui le seuil sans fausse
fusion vaut **0,50 pour les sujets et 0,07 pour les mémoires**. Un seuil
global aurait fusionné à tort sur l'un ou refusé tout sur l'autre.

**L'abstention n'existe pas, il faut donc que la zone la fabrique.** Aucun
modèle ne s'abstient quand on lui en offre l'option. Ma condition 5 traitait
ça comme un repli ; c'est le seul chemin. Le nœud **possède** l'abstention,
il ne la demande pas.

**Un signal fort sur « variante », et une conclusion que j'ai tirée trop
vite.** Le verdict à quatre options de `remember` tient à 85 % ; celui des
sujets en *même / variante / sans rapport* tombe à 64 %, et c'est « variante »
que tout le monde rate — la frontière est discutable **même entre nous**.
J'en ai conclu qu'il fallait deux options nettes. **C'était prématuré**, et
Lucie l'a arrêté : la mesure dit qu'une découpe particulière est difficile,
elle ne dit pas laquelle la remplace. Un choix à deux, deux oui/non enchaînés,
un « sous-sujet de » — trois formes au moins, et rien pour choisir.

Ce qui reste vrai, et qui suffit : **une option sur laquelle des humains ne
s'accordent pas n'a rien à faire dans une liste fermée offerte à un modèle.**
C'est une contrainte sur la façon de valider une découpe, pas la désignation
d'une découpe.

**Donc rien n'est figé, et le nœud est écrit pour ça** : l'étage 2 reste sur
« demander toujours », aucun modèle n'est branché, et le critère comme la liste
de choix sont des **données du graphe**. Un diagnostic de formulation est en
cours ; quand il rendra, changer la découpe devra coûter une ligne de gabarit.
Le mot de Lucie : « ça reste une expérimentation, faut pas courir avec avant
d'avoir trouvé si une bonne manière de s'en servir existe. »

**Le coût, et une conséquence pour le nombre fixe de candidats.** Le seul
modèle qui décide coûte 2 s par décision sur processeur, **4,4 s à dix
candidats**, et passe par `llama-server`. Les petits encodeurs répondent en
54–117 ms mais ne décident pas sur nos critères (45 % et 31 % pour trois
choix, le hasard étant à 33 %). Donc le nombre fixe d'options du §7.5 point 4
est aussi un **réglage de latence**, et les deux exigences poussent dans le
même sens : **peu de candidats**. Trois vaut mieux que dix, pour la
comparabilité comme pour le temps.

**Les réserves de la mesure, et ce qui les a levées.** 42 et 20 paires,
étiquetées par une seule personne, et **les seuils réglés sur les paires mêmes
qui servent à juger** — donc les AUC annoncés étaient une borne haute.

Le banc de §4 a bouché ce trou, et ce n'était pas prévu pour ça : son scénario
est **écrit**, donc il produit des paires dont la réponse est connue sans que
personne les ait étiquetées.

### 7.8 Et le contrôle a défait la conclusion

Les 48 paires du banc, jouées avec le critère, les options et le seuil du
document 02 **sans rien régler** :

| | sur les paires jugées | sur les 48 du banc |
|---|---|---|
| AUC de P(même) | 0,97 | **0,82** |
| au seuil 0,07 | aucune fausse fusion | **4 redites sur 6** passent, et **6 « différent » sur 40** passent aussi |
| verdict à quatre choix | 85 % | **« même » pour aucune des six redites** |

Les deux distributions se recouvrent : P(même) va de 0,03 à 0,31 pour les
vraies et monte à 0,13 pour une paire sans rapport. **« Le seuil tient »
reposait sur six paires ; il ne tient pas sur quarante-huit.**

Ce qui tient quand même, et qui n'est pas rien : les deux contradictions
sortent à 0,03 et 0,05 — **aucune n'est prise pour une redite**. C'est la
confusion la plus coûteuse, et elle ne se produit pas.

**Et une réserve sur mon propre jeu, qu'il faut écrire avec le reste.** Mes
redites étaient des paraphrases lointaines de titres très courts, et le modèle
ne voyait que le titre. C'est **plus dur que la vraie tâche**, où un
`remember` apporte toujours son pourquoi. Donc ni 0,97 ni 0,82 n'est « le »
chiffre : entre les deux mesures, la définition de la tâche a bougé. Les
paires portent désormais l'affirmation **et** le pourquoi des deux côtés, pour
qu'une prochaine mesure sépare le modèle de ce qu'on lui montre.

### 7.9 Ce que je retire — une mesure est un fait sur son jeu

J'avais adopté cinq « trouvailles » de la forme de la question. Rejouées sur un
autre jeu — les entrées d'un journal rangées sous leurs sections — la forme qui
faisait 15 sur 16 rejoint **13 fois sur 32**. Deux survivent, deux non :

| Trouvaille | Ailleurs |
|---|---|
| la décision se pose sur le texte, pas sur deux noms | **survit** |
| l'ordre des options déplace les verdicts | **survit** |
| quatre candidats plutôt que deux ou dix | **non confirmé** |
| nom et description suffisent, le contexte nuit | **non confirmé** |
| un seul choix dont le neuf est une option | à revoir |

Je retire donc « quatre candidats » et « nom et description suffisent » de ce
que la conception fixe. Ce qui reste du §7.5 point 4 — **un nombre fixe** de
candidats — tient toujours, parce qu'il vient du softmax et non d'une mesure :
c'est *le nombre* qui n'est pas connu, pas la nécessité qu'il soit fixe.

**La leçon, et c'est la deuxième fois dans la même journée que je la reçois :**
une mesure est un fait sur le jeu qui l'a produite. J'avais déjà tranché trop
vite sur « variante » après un seul chiffre ; j'ai recommencé avec cinq. Ce
qu'une mesure autorise, c'est d'éliminer — pas de choisir.

Conséquence pratique, et elle est rassurante : **l'étage 2 sur « demander
toujours » est le seul réglage que la mesure soutienne aujourd'hui.** C'est
déjà ce que la conception prévoit, et Lucie l'avait demandé avant que ces
chiffres existent.

### 7.10 Trois jeux plus tard : ce qui est acquis, et ce que ça change

Mesuré sur trois jeux à la fois — celui de l'auteur, le journal des chantiers,
et un jeu public à étiquettes humaines.

**1. Ranger est un problème d'ordre, et on l'a déjà.** Le cosinus fait aussi
bien que le modèle de décision : 14 sur 16, 19 sur 32, 22 sur 30. Donc le
premier appel de `remember`, celui qui montre les proches, **n'a besoin
d'aucun modèle de plus**.

**Mais à une condition que ma déclaration ne remplit pas, et c'est ma
correction.** Le bon résultat s'obtient en comparant le texte au **texte
complet du sujet** — sa description *et ce qu'il porte*. Avec les noms seuls :
8, 7, 15. Or mon `Subject.about` est une description courte, écrite une fois,
qui **ne contient pas** ce qu'on a rangé dessous. Elle est donc plus près du
cas « nom seul » que du bon.

> **Un sujet doit être une entité dérivée.** Son texte cherchable n'est pas
> écrit, il est **rendu** depuis sa racine et ses voisines — c'est-à-dire
> depuis les mémoires qui s'y ancrent.

Et la pièce existe, livrée par le repli des bases de connaissances :
`DerivedConfig`, les règles `gather`, `DeriveNode`, et le court-circuit par
`_render_hash` qui réinvalide le rendu quand l'ensemble ancré change. Un sujet
grossit donc tout seul à mesure qu'on lui accroche des notes, et son vecteur
suit — sans un nœud de plus.

C'est aussi, accessoirement, la réponse à ce qui fait **vieillir** un sujet :
un sujet dont plus rien ne dépend cesse d'être rendu.

**2. Décider qu'il faut créer ne marche par aucune piste, sur aucun jeu** : ni
l'option « aucun de ces sujets » (3 sur 8, 2 sur 15, 3 sur 12), ni la faiblesse
du meilleur score (AUC de 0,53 à 0,80), ni un seuil qui se transporte. **Rien
de mesuré ne fait mieux que « demander toujours » avec création réparable.**

Donc le protocole à verbe unique dont le premier appel refuse en montrant les
proches n'est **pas un pis-aller** : c'est la meilleure réponse connue, et
c'est l'appelant — agent ou personne — qui tranche entre rejoindre un proche et
créer.

**3. Et une distinction que je n'avais pas : le texte du juge n'est pas le
texte du vecteur.** Sur mes paires, le modèle de décision passe de 0,80 à
**0,92** d'AUC quand on lui donne le pourquoi en plus du titre, et reconnaît
cinq redites sur six au lieu de trois. **Le cosinus fait l'inverse** : 0,99 sur
les titres, 0,83 avec le pourquoi.

Le nœud doit donc nourrir ses deux étages avec **deux textes différents** :

| Étage | Ce qu'il reçoit | Pourquoi |
|---|---|---|
| 1. classement | le titre, court | le vecteur se dilue dans le paragraphe |
| 2. verdict | le titre **et** le pourquoi | juger « même chose » demande la raison |

Ce n'est pas un réglage, c'est la forme du nœud. Et ça n'a rien d'évident : la
tentation est de passer le même objet aux deux, puisque c'est la même mémoire.

**4. L'ordre des lots change**, et la raison est au journal des chantiers : le
nœud de décision **à modèle** descend derrière le réacteur, le crochet après
outil et le jardinier. Rien de mesuré ne le rend utile aujourd'hui, et les
trois autres servent sans lui.

**Les trois zones, et leur asymétrie assumée.** Les coûts ne sont pas
symétriques, donc les zones ne doivent pas l'être :

- **fusionner** est difficilement réversible → zone haute seulement ;
- **créer** est bon marché et réparable → le jardinier fusionnera ;
- donc **la zone incertaine se résout en « créer »**, et la question remonte
  **sans bloquer** : les outils asynchrones et la pause existent depuis août,
  donc « la question remonte à la personne » est une poignée résolue plus tard,
  pas un agent figé.

Jamais de fusion silencieuse à faible confiance : c'est la demande de Lucie, et
la forme ci-dessus la rend **structurelle** au lieu de la confier à un réglage
qu'on peut desserrer.

**Les mesures propres au nœud**, à part des cinq de §4.2 : sujets doublonnés
créés, sujets fusionnés à tort, **taux d'abstention**, et l'accord avec un
jugement humain sur un échantillon.

Et une conséquence à dire : **« fusionnés à tort » n'est mesurable que contre
une vérité.** Le banc scripté de §4 la donne gratuitement — dans un scénario
écrit, on sait lesquelles paires sont la même chose. Sans lui, cette mesure
n'existe pas ; c'est une raison de plus de l'écrire avant le reste.

**Le choix du modèle reste ouvert.** Des alternatives en poids ouverts m'ont
été signalées ; elles ne sont **pas vérifiées** et ce n'est pas à cette
proposition de les retenir. La session optimiseur vérifie dépôts, licences et
architectures, puis mesure deux candidats sur des paires de sujets. Les cinq
conditions ci-dessus sont ce que son tableau doit permettre de trancher — en
particulier la deuxième, qui est celle qu'un tableau de performances ne montre
jamais.

---

## 8. Ce qui attend un choix de Lucie

Les trois de la vision — premier usage visé, qui écrit sans confirmation, ce
que la personne voit — et un quatrième que cette proposition ajoute :

4. **Que fait la zone incertaine du nœud de décision.** Créer et demander
   après (ce que je propose) : aucun agent n'est bloqué, des doublons existent
   et le jardinier les traite. Ou demander avant et attendre : aucun doublon,
   l'agent s'arrête. C'est un choix de produit, pas de technique.
