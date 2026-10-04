# Proposition — l'adresse, les genres de référence, et le classeur

4 octobre 2026, session mémoire longue. Réponse demandée par l'orchestration à
la vision `../4-octobre-2026-00h09/01`. Même méthode que la proposition de la
mémoire longue : je prends ce que le code permet déjà, je **contredis** ce qu'il
dit autrement, et je nomme ce que je ne sais pas.

Six contradictions, toutes vérifiées à la source. Une concerne du code que j'ai
livré il y a deux heures.

## 1. Ce que le code permet déjà, et qu'il ne faut pas réinventer

| Besoin de la vision | La pièce qui existe |
|---|---|
| un enum de genres **qui grandit**, relu à chaque appel | `Choices` (`dataflow/node_registry.rs`) : `Fixed`, `Targets`, `Relations` — et les deux dernières sont **résolues contre le catalogue au moment où la fiche est rendue**. Un genre créé à l'instant paraît dans le schéma de l'appel suivant **sans mécanisme neuf** |
| une règle de genre portée par un script | un backend déclare déjà ses scripts rhai au manifeste : `templates/backends/validated-result/backend.json`, clé `"scripts"` → `hooks/*.rhai`. Plus `RhaiNode` et `ValidationRuleNode` |
| un refus qui dit l'appel exact à refaire | la silhouette vérifiée par la session recherche : `ok: true`, `executed: false`, charge `needs`. Un résultat d'outil n'est une erreur que s'il porte un champ `error` au premier niveau |
| « à revoir » quand une cible bouge | `EntitiesChanged` + `ReactTransitionNode` + la machine à états, livrés aujourd'hui |
| plusieurs mémoires dans une base | la cellule : `_Org`/`_Project`, `scope_columns()`, `multi_cell` — le découpage existe et sert déjà |

**Donc `new` et `newRefType` n'ont pas besoin d'exister.** La vision les garde
en repli de la forme à deux outils ; je propose de les supprimer tout à fait.
`Choices::RefTypes`, résolu contre le catalogue comme `@targets` l'est,
suffit : `add_ref` montre la liste du jour, refuse un genre absent **avec la
liste et l'appel exact** (`create_ref_type(name, description, rule)`), et
l'appel suivant voit le genre neuf. Un champ optionnel qu'on remplit en passant
est précisément ce qui fait entrer le désordre sans qu'on le voie — et c'est
l'argument de Lucie elle-même pour préférer les outils.

## 2. Ce que je contredis

### 2.1 Une adresse n'est pas une relation. C'est une entité d'entrée.

La vision dit « une fiche a des liens, un lien a une adresse, l'adresse a un
genre ». Si « lien » veut dire une relation déclarée, **le moteur refuse**, et
pas par accident :

- **une relation a un couple `(depuis, vers)` fixe** : `register_relation_with`
  refuse durement un ré-enregistrement avec un autre couple. Une seule relation
  `POINTS_TO` ne peut pas désigner neuf genres de cible ;
- **au plus une relation par couple** si l'on veut filtrer d'une entité sur une
  autre : `find_relation` itère une table de hachage et rend la **première** —
  avec deux relations entre les mêmes bouts, laquelle est prise n'est pas
  déterministe ;
- **une propriété d'arête ne se cherche ni ne se filtre** : `FetchRelatedNode`
  traverse une arête anonyme et ne rend pas ses propriétés ; `FilterParser` ne
  connaît que des champs de nœuds. Donc le genre et la valeur **ne peuvent pas**
  vivre sur l'arête ;
- et `RelationBatchNode` **refuse** une relation à propriétés en indiquant le
  remède, mot pour mot : *« snapshot links require a property-free relation; use
  an entry entity for payload »*.

**Le moteur donne donc la réponse dans un message d'erreur** : une entité
d'entrée. Ma proposition :

```
Ref  (entité d'entrée, déclarée comme n'importe quelle autre)
  genre   : string   — contraint par Choices::RefTypes
  valeur  : string   — la forme normale rendue par le harnais du genre
  vu_le   : int      — date serveur
  empreinte : string — ce qui fait vieillir, selon le genre ; vide si rien
  etat    : string   — machine à états : vue → a_revoir → introuvable / retirée

CITE      : Fiche → Ref     (relation nue)
DESIGNE   : Ref  → <ligne>  (relation nue, **seulement pour le genre « ligne »**)
```

Deux conséquences qui tombent bien :

- **le lien se lit dans les deux sens sans rien de neuf** : depuis la fiche par
  `CITE`, depuis la cible par la traversée entrante — et `FetchRelatedNode` est
  **déjà** polymorphe (il fait un `MATCH` sur des nœuds non étiquetés et lit
  `label(m)` à l'exécution). C'est la déclaration qui borne, pas le stockage ;
- **le genre et la valeur sont des champs de nœud**, donc cherchables et
  filtrables : « que sait-on de `path:docs/users.md` ? » est un filtre sur deux
  champs, pas une traversée à inventer.

Et une réserve que je ne cache pas : `DESIGNE` retombe sur le couple fixe. Pour
que `Ref` désigne une ligne de n'importe quelle entité, il faut soit une
relation `DESIGNE_<Entité>` engendrée à l'enregistrement d'une entité
`adressable` (ce que la proposition de la mémoire longue faisait déjà pour
`ANCHORED_TO`), soit se passer de relation et garder l'uuid dans `valeur`. Je
penche pour **la relation engendrée** : elle seule donne la traversée, donc le
rappel par ce qu'on touche.

### 2.2 Le réacteur que je viens de livrer ne sait pas suivre une adresse

`ReactTransitionNode` (fusionné aujourd'hui) remonte **une relation déclarée**,
nommée dans son gabarit. Il fait donc marcher « un sujet change → les mémoires
qui l'ancrent repassent à revoir », et **il ne fera pas** « un fichier change →
les fiches qui le citent repassent à revoir » : il n'y a pas de relation vers un
fichier, seulement une `Ref` dont `valeur` vaut `path:docs/users.md`.

Ce qui manque est petit mais réel : un second mode, où la remontée se fait par
**égalité de champs** sur `Ref(genre, valeur)` au lieu d'une relation. Je le
dis maintenant parce que c'est mon code et que je connais son coût — pas parce
que la vision l'a demandé. Et je ne l'écrirai pas avant que l'adresse soit
tranchée : un second mode écrit pour une forme d'adresse qui change serait à
réécrire.

### 2.3 La première porte « sans modèle si possible » est exactement ce que mes mesures déconseillent

La vision veut une porte à deux temps, dont le premier « sans modèle si
possible » : le tour comparé au sommaire du classeur, un modèle seulement en cas
de doute. **Mes mesures disent qu'un score de similarité ordonne bien et juge
mal** :

| Mesure | Chiffre | Ce qu'elle dit |
|---|---|---|
| ranger par le cosinus, texte complet du sujet | 14/16, 19/32, 22/30 | **ordonner marche** |
| sur les noms seuls | 8/7/15 | et le texte compte |
| décider par un seuil, sur mes 72 paires de contrôle | AUC 0,82 | **juger marche mal** : les deux distributions se recouvrent |
| le même seuil transporté sur un autre jeu | 15/16 → 13/32 | **un seuil ne se transporte pas** |

Une porte oui/non sur un score se trompera donc dans les deux sens, et ses deux
erreurs ne coûtent pas la même chose : un **faux non est un silence** — le
défaut que toute cette mémoire existe pour combattre — tandis qu'un faux oui
coûte une recherche bornée.

**Ma proposition : que la première porte ne juge pas la pertinence du tout.**
Son vrai travail est le dédoublonnage, que la vision nomme déjà (« ne pas
chercher deux fois la même chose ») : elle dit **oui par défaut**, et **non
seulement** quand le registre de la session montre qu'on a déjà cherché ce
sujet-là et que rien n'a changé depuis. C'est une condition **vérifiable**, pas
un score — et elle n'a pas de seuil à transporter.

Le coût est borné par la recherche elle-même, qui est un graphe borné. Le score,
lui, garde son emploi juste : **ordonner** les sujets du classeur une fois
qu'on a décidé de chercher, ce que mes chiffres soutiennent.

Et la seconde porte — injecter ou non — est une vraie décision fermée : là, un
modèle se justifie, avec les trois refus de la vision comme critères. C'est
aussi la seule des deux dont une erreur se voit à la lecture.

### 2.4 Le schéma et le script sont un seul harnais

La vision hésite (« si le schéma et le script sont deux harnais ou un seul »).
Je propose **un seul** : la description obligatoire, et un script rhai
facultatif. Raisons :

- le dépôt a déjà le précédent et l'outillage — un manifeste déclare ses
  scripts, `RhaiNode` et `ValidationRuleNode` existent ; un schéma de validation
  de valeur n'a aucun précédent de ce côté ;
- deux harnais pour une seule question (« cette valeur est-elle de ce genre ? »)
  doublent la surface à tenir pour un seul gain, la lisibilité — et un script de
  dix lignes avec son nom de fonction se lit ;
- et **la règle de trois notions appelées « racine »** s'applique : un niveau de
  plus ne se justifie pas, il se fond.

Les trois fonctions facultatives de la vision (`validate`, `resolve`,
`fingerprint`) sont la bonne découpe et je les garde telles quelles. Le niveau 0
(description seule) reste, avec sa conséquence : ses références sont dites **non
vérifiées**, jamais tenues pour sûres.

### 2.5 « Le L3 dit ce qui a changé » est déjà une machine à états, pas une consigne de rédaction

La vision demande qu'une décision renversée reste lisible comme renversée, pour
que l'article principal n'efface pas ce qu'une mémoire doit garder. C'est juste,
et **le gabarit `memory` le modélise déjà** : `superseded` est un état terminal,
`REPLACES: Memory → Memory` est déclarée, et la garde de cycle de vie refuse de
ramener une ligne `superseded` à `current`.

Donc : un L2 ou un L3 refait **ne réécrit pas** l'ancien ; il naît, et l'ancien
passe `superseded` avec un `REPLACES` vers le neuf. Le « on faisait X jusqu'au
3 octobre » n'est alors pas une tournure que l'archiviste doit penser à écrire —
c'est une traversée. Une consigne de rédaction se perd au premier résumé
distrait ; une machine à états ne se perd pas.

### 2.6 Une seule base pour toutes les mémoires d'une personne

C'est le troisième choix du §8, et je recommande **une seule base**, pour une
raison de moteur et non de goût : une relation ne traverse pas deux bases. Des
mémoires séparées rendraient `CITE` et `REPLACES` impossibles entre elles, donc
la boucle étrange — une fiche qui pointe vers une autre mémoire — ne serait plus
qu'un texte.

Et le découpage existe déjà pour ça : la cellule (`_Org`/`_Project`,
`scope_columns()`, `multi_cell`) sert aujourd'hui à isoler des locataires dans
une base. Une mémoire est une cellule de plus, ou un champ `memoire` sur la
fiche — à mesurer, mais dans les deux cas **sans base de plus**.

La réserve : une base unique concentre le risque, et nous savons qu'un arrêt
brutal casse le journal (`docs/tickets/`). Un instantané par reflink avant toute
grosse écriture reste la parade, et elle ne change pas.

## 3. L'ordre que je propose

Il diffère du §7 sur un point : l'entité `Ref` passe devant tout, parce que le
moteur l'impose et que son dessin décide du reste.

1. **`Ref`, `CITE`, et les genres de niveau 0** — l'entité d'entrée, la forme
   normale, la machine à états de la référence. Aucun script, aucun registre :
   des fiches qui citent des choses, et qui se retrouvent par `genre` et
   `valeur`. **Mesurable tout de suite** au banc.
2. **`add_ref` / `create_ref_type` avec `Choices::RefTypes`** — l'enum qui
   grandit, le refus qui dit l'appel exact. Le harnais rhai vient ici, en
   facultatif.
3. **Le second mode du réacteur** (remontée par `Ref(genre, valeur)`) : « à
   revoir » devient vrai pour les fichiers, les commits, les tickets.
4. **La relation engendrée pour le genre « ligne »**, qui donne le rappel par ce
   qu'on touche sur les lignes du catalogue.
5. **Le classeur L1/L2/L3**, avec `REPLACES` entre étages.
6. **Le registre des objets du système**, et les genres `schema`, `graph`,
   `template` : la boucle. En dernier, parce que c'est le seul morceau qui
   demande du Rust dans le catalogue, et que les cinq premiers s'en passent.

## 4. Ce que je mesurerais, et dans quel ordre

Le banc de la mémoire longue sert tel quel, avec trois chiffres de plus :

1. **genres créés à tort** — deux genres pour une même chose (« référence
   d'article » et « DOI »). C'est le désordre que `create_ref_type` peut
   introduire, et la seule mesure qui juge le remède plutôt que le défaut ;
2. **références non résolues** — « cité, introuvable » est une information ;
   son taux dit si les harnais valent quelque chose ;
3. **la première porte** : sur un banc de tours réels étiquetés « il fallait
   chercher » ou non. Ma proposition §2.3 la rend presque triviale à mesurer —
   une porte qui dit toujours oui sauf dédoublonnage a un taux de faux non égal
   au taux de dédoublonnages à tort, et rien d'autre.

Et une mesure que je ne sais pas faire aujourd'hui, à nommer plutôt qu'à
oublier : **l'abandon**. Après une demande de précision, l'agent rappelle-t-il
ou renonce-t-il ? La session recherche et moi avons la forme des quatre issues
(rappel, reprise déviée, rappel en texte, abandon) et sa matière dans
`events.jsonl` ; l'instrument n'est pas écrit.

## 5. Les trois choix du §8, et ma recommandation

| Choix | Ma recommandation | Pourquoi |
|---|---|---|
| l'ordre, ou le registre d'abord | **l'ordre, avec `Ref` devant** | les cinq premiers morceaux ne demandent aucun Rust dans le catalogue ; le registre en demande |
| qui peut concevoir un gabarit | **tout agent** | la garde est dans le verbe — montrer les proches, demander lequel on retient —, pas dans une permission. Une permission se contourne en demandant à la personne, qui dira oui sans savoir |
| une base, ou une par mémoire | **une seule base** | une relation ne traverse pas deux bases ; la cellule existe déjà pour isoler |

## 6. Ce que je ne sais pas

- ~~**Qui écrit le L1.**~~ **Répondu** par la session recherche, qui tient le
  chat, et la réponse change la conception en mieux : **le chat ne compresse
  rien aujourd'hui.** Ni résumé ni élagage ; les seules bornes sont
  `AgentLimits.token_budget` (qui **arrête** le tour), `max_iterations`, et la
  troncature de la sortie d'un outil à l'entrée. Le contexte grandit tel quel.

  Donc il n'y a pas de résumé du harnais à lire, et **l'archiviste écrit le
  sien** — processus séparé, qui lit `journal/<session>.jsonl`, écrit **au fil
  de l'eau** et lisible **pendant** le tour, avec des frontières de tours où une
  tranche se découpe proprement. La session JSON n'est écrite qu'en fin de tour :
  le journal est la bonne source.

  Et « 30 % » est mesurable sans rien ajouter : `AgentRun.usage` cumule les
  jetons, le `prompt_tokens` du **dernier** appel est la taille réelle du
  contexte envoyé, le modèle déclare sa fenêtre (`ModelSource.context_tokens`),
  et l'événement `turn_end` porte déjà le cumul.

  Il manque **un seul** crochet, et il a un site net : la **fermeture de
  session** — la sortie de boucle de `rag3weaver-chat`, avant `drop(tools)`,
  où le backend est encore vivant et le journal joint. Trois lignes, même motif
  que l'`index_status` d'après-tour. Le crochet « à la compression », lui, n'a
  aucun site aujourd'hui — et ma conception n'en a pas besoin.

  **Ce que ça simplifie** : pas de négociation avec le harnais, pas de
  dépendance à un résumé dont la forme ne nous appartient pas, et le même
  chemin pour un agent hébergé ailleurs — on lit ce qu'il émet, pas ce qu'il
  pense.
- **« 30 % du contexte »** : du total ou depuis le dernier L1. Mesurable une
  fois le classeur écrit, pas avant.
- **Si un L1 touchant plusieurs sujets se range sous chacun ou se découpe.** Mon
  intuition dit « sous chacun », parce que découper un résumé le rend faux par
  les bords ; je n'ai rien pour l'appuyer.
- **Le budget qui déclenche le L3.** Un seuil de plus, donc à ne pas poser au
  jugé : une donnée du gabarit, avec un défaut assumé et dit.
