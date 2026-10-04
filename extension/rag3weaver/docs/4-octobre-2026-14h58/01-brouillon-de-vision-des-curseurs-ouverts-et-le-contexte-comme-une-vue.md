# Brouillon de vision — des curseurs ouverts, et le contexte comme une vue

4 octobre 2026. **Brouillon** : rien n'est décidé ni codé. Deux idées de
Lucie, la seconde plus lointaine que la première :

> « Et si les agents avaient des curseurs ouverts sur des fichiers, comme un
> vrai dev, qu'ils peuvent fermer quand ils veulent, et tant qu'ils ne le font
> pas ils voient le curseur actif et les relations dans leur contexte ? »

> « Peut-être que le contexte serait vraiment à manager totalement
> différemment de ce qui se fait actuellement. Il faudrait expérimenter, mais
> peut-être qu'un jour ce ne serait plus un simple fil d'historique d'outils,
> mais une vue optimisée de contexte, en permanence. Il faudrait d'abord
> faire des essais intermédiaires qui ne vont pas directement là-dessus. »

Ce document complète les visions du produit code
(`../3-octobre-2026-20h37/01`), l'exploration des relations
(`../3-octobre-2026-20h16/01`) et la vision des mémoires
(`../4-octobre-2026-00h09/01` et `02`).

## 1. Le trou que cela comble

Un agent qui lit un fichier en garde une **copie** dans son contexte. Cette
copie vieillit sans le dire : un autre agent édite, lui-même édite, la
synchronisation passe, et il raisonne sur un texte qui n'existe plus. Il
relit par précaution, ou il édite à l'aveugle et l'édition échoue.

Un développeur, lui, a des onglets ouverts : ce qu'il regarde est vivant, et
il sait toujours sur quoi il est.

## 2. Le curseur

Un **curseur** est une adresse tenue ouverte par l'agent : un scope (une
fonction, un type), une plage, un fichier. Tant qu'il est ouvert, le harnais
en tient l'état et le montre.

| Ce qu'un curseur montre | D'où ça vient | État |
|---|---|---|
| où il est : chemin, lignes, signature du scope | le catalogue | livré |
| sa fraîcheur : inchangé, ou « a changé depuis l'ouverture », et par qui | l'ingestion émet ce qu'elle change | livré côté événement |
| ses relations : appelants, dépendants, tests — quelques lignes, comptées | `usages`, `impact`, section « Avant d'éditer » | livré, lié à un appel d'outil |
| les fiches de mémoire qui parlent de lui | les liens de la vision des mémoires | proposé |
| qui d'autre a un curseur dessus | un registre des curseurs par session | à faire |

Ce qu'un curseur ne montre **pas** : le corps du fichier. Le corps se lit par
un outil ; le curseur dit s'il faut le relire.

**Les gestes** : ouvrir, déplacer, fermer, lister. Rien d'autre.

**C'est une adresse de la vision des mémoires.** Un curseur est une
référence (`row:Scope/…`, une plage, `path:…`) qui vit le temps de la
session ; une fiche de mémoire est la même référence, durable. À la
fermeture, le harnais peut proposer d'en garder une note — jamais l'imposer.

## 3. Ce que cela donne

- **Une vue vivante au lieu d'une copie morte** : moins de relectures de
  précaution, moins d'éditions sur un contenu périmé.
- **Les relations sans les demander**, là où l'agent travaille, et seulement
  là.
- **Un état qui survit à la compression du contexte** : les curseurs sont
  tenus par le harnais et rendus de nouveau, pas résumés.
- **Un signal pour l'archiviste** : les curseurs ouverts disent sur quoi
  l'agent travaille sans qu'il le déclare — le fil de travail, et les fiches à
  rappeler.
- **La présence entre agents** : un curseur visible des autres est un verrou
  d'intention, qui annonce sans bloquer.

## 4. Les points durs

- **Le coût en contexte.** Le souci permanent : ne pas injecter sans cesse.
  Un bloc à budget fixe, rendu en fin de prompt (le préfixe reste en cache),
  avec l'en-tête et quelques lignes de relations par curseur ; ne rendre de
  nouveau que ce qui a changé.
- **L'agent oublie de fermer.** Un plafond, le coût affiché (« 4 curseurs,
  ~700 jetons »), et le plus ancien fermé d'office en le disant.
- **Le grain.** Le scope plutôt que le fichier : c'est là que sont les
  relations. Un fichier sans scope (un md, une configuration) reste un
  curseur de fichier.
- **Le bruit des relations.** Un curseur montre en permanence ce que la
  section « Liens » montre une fois : les fausses arêtes (variables locales
  Rust, conteneur `mod tests`) y pèseraient plus lourd. À corriger d'abord.
- **Hors de notre chat.** Dans un harnais qui ne laisse qu'ajouter du
  contexte (Claude Code, `../4-octobre-2026-00h09/02`), pas de bloc épinglé :
  on n'aurait qu'une approximation, une injection quand un curseur change.

## 5. L'idée plus lointaine : le contexte comme une vue

Aujourd'hui, le contexte d'un agent est un **journal** : tout ce qui a été
dit et fait, dans l'ordre, jusqu'à ce qu'il déborde et qu'on le résume. Ce
que l'agent sait de l'état du monde est éparpillé dans ce journal, en copies
de dates différentes.

L'autre forme : le contexte est une **vue**, recalculée à chaque tour à
partir d'un état tenu hors du modèle —

| Zone de la vue | Ce qu'elle porte | Ce qui la tient |
|---|---|---|
| la tâche | le but, ce qui reste, ce qui est décidé | le fil de travail (mémoire) |
| l'espace de travail | les curseurs ouverts, leur fraîcheur, leurs relations | le registre des curseurs |
| ce qui vient de se passer | les derniers tours, tels quels | le journal, sur une fenêtre courte |
| ce qui a été appris | des fiches, rappelées par ce qu'on touche | l'archiviste |
| l'historique | accessible par un outil, pas présent | le journal complet |

Le journal ne disparaît pas : il devient une source qu'on interroge, pas le
contexte lui-même. Les curseurs sont la première zone de cette vue ;
l'archiviste (L1, L2, L3) en est une autre. Les deux visions se rejoignent
ici.

Ce qu'on ne sait pas, et que seuls des essais diront :

- si un modèle travaille aussi bien sur une vue recalculée que sur son propre
  fil — il est entraîné sur des fils ;
- ce que coûte une vue qui change à chaque tour (le cache de préfixe est
  perdu sur ce qui bouge) ;
- ce qu'on perd en ôtant un résultat d'outil que l'agent croyait avoir sous
  les yeux.

## 6. Les essais intermédiaires

Du plus petit au plus engageant ; chacun se juge seul, et aucun n'oblige au
suivant.

| # | Essai | Ce qu'il apprend | Ce qu'il demande |
|---|---|---|---|
| 1 | **La fraîcheur seule** : après un outil, dire « ce que tu as lu de X a changé depuis » | si les copies périmées sont un vrai coût | le crochet après outil (livré) et un registre de ce qui a été lu |
| 2 | **Les curseurs** : `open` / `close`, un bloc en fin de prompt, relations et fraîcheur | si un état épinglé évite relectures et éditions ratées | un registre par session, le rendu, un plafond |
| 3 | **Un résultat d'outil périmé se replie** : la lecture d'un fichier rouvert depuis est remplacée par une ligne (« lu au tour 12, remplacé au tour 20 ») | si l'on peut ôter du fil sans perdre l'agent | la maîtrise de l'assemblage du prompt (notre chat) |
| 4 | **La zone « tâche »** : but et reste à faire tenus hors du fil, rendus à chaque tour | si l'agent tient mieux un travail long | le gabarit *fil de travail* de la mémoire |
| 5 | **La vue** : fenêtre courte de tours + zones, le reste par outil | si la vue remplace le fil | tout ce qui précède |

**Comment juger** : des passes d'agent réelles (Gemini, comme pour le produit
code), sur les mêmes tâches avec et sans. Compter les relectures du même
fichier, les éditions ratées sur un contenu périmé, les appels à `usages` et
`impact`, les jetons par tâche, et la réussite. Une tâche longue au moins,
qui traverse une compression : c'est là que la différence doit se voir.

## 7. Le débat du 4 octobre : replier, ou réunir

Le débat n'est pas clos ; cette section garde les positions, pas une
décision.

**L'idée de départ (Lucie)** : une lecture ou une édition lointaine se
réduit — plutôt que tout son retour, l'agent voit la fiche (ou la docstring
plutôt qu'un résumé) et les relations.

**Les objections** :

- la fiche dit ce qu'est une chose, pas ce que l'agent y cherchait : le
  détail pour lequel il a lu le corps n'est ni dans la docstring ni dans les
  relations ;
- une édition n'est pas une lecture : ce qui compte plus tard, c'est ce qui a
  été changé et pourquoi ;
- un modèle qui ne voit plus le corps peut le réinventer depuis la docstring
  et éditer de mémoire ;
- réécrire un vieux message coûte le cache de tout ce qui suit.

**Où Lucie arrive** : « Ce ne serait plus vraiment l'idée de lui cacher les
retours exacts, mais de les réunir proprement par fichier : ce serait une
belle première étape. Et rendre les anciennes lectures vivantes. »

**Ce que cela donne — la première étape** :

- le contenu exact d'un fichier lu vit **une seule fois**, dans une zone par
  fichier : les plages lues, fusionnées, au contenu **du jour**, éditions
  comprises ;
- le fil garde les **événements** (« lu `X` l. 10-80 », « modifié `X` »), plus
  le contenu : chaque événement pointe vers la zone du fichier ;
- quand le fichier change, la zone change, et dit ce qui a changé depuis
  (« l. 34-40 modifiées au tour 20, par toi ») — sans cela l'agent relit son
  propre raisonnement sur un texte qui n'est plus là ;
- rien n'est caché : ce qui se gagne, ce sont les doublons (relectures) et
  les copies périmées.

C'est le curseur du §2 avec le corps : un fichier lu **est** un curseur
ouvert sur les plages lues.

**Ce qui reste à débattre** :

- quand la zone déborde, que fait-on ? C'est là que l'idée de départ revient,
  comme politique de débordement et non comme défaut : le fichier le moins
  touché se replie en fiche + relations, marqué « corps hors contexte ».
- la ligne « ce que j'en ai retenu » : écrite par l'agent, par l'archiviste,
  ou pas du tout tant que rien n'est replié ?
- le fil pointe vers un contenu qui a pu changer : faut-il garder, par
  événement, la trace de ce qui était lu alors (un diff), ou seulement l'état
  du jour ?
- le coût de cache d'une zone qui bouge à chaque édition.

**L'objection de Lucie à sa propre étape** : « L'agent peut oublier pourquoi
il avait édité telle chose, vu qu'il ne voit plus le problème dans son
historique. Peut-être une clarification dans la zone : il voit les diffs et
l'état final, de la manière la plus concise possible. »

Pistes, non tranchées :

- **le diff porte l'avant** : les lignes retirées sont le code du problème ;
  état final + diff ne perd donc pas le texte, seulement la raison ;
- **la raison se donne à l'édition** : un argument `reason` d'une ligne sur
  l'outil d'édition, comme un message de commit — écrit au moment où l'agent
  la connaît le mieux, sans appel de plus. Obligatoire ou facultatif : à
  débattre ;
- **par fichier, un diff net et un journal de raisons** : le diff net depuis
  la première lecture (plusieurs éditions de la même plage n'en font qu'une),
  et une ligne par édition (« tour 20, l. 34-40 : la garde manquait le cas
  vide ») ;
- **les essais défaits restent au journal** : un diff net ne montre pas ce
  qui a été tenté puis retiré, et c'est ce qu'il ne faut pas retenter ;
- **au-delà d'une taille**, le diff se réduit à son compte (+12 −3) et se
  rouvre par un outil ;
- ce journal de raisons est de la matière toute prête pour l'archiviste (L1)
  et pour un message de commit.

Ce qui ne bouge pas : les retours qui ne sont pas du contenu de fichier (une
sortie de test, une erreur de compilation) restent dans le fil ; le problème
qui a motivé l'édition y est donc encore, tant que le fil n'est pas compressé.

Dans la liste du §6, cette étape se place entre les essais 1 et 2 : elle
demande la maîtrise de l'assemblage du prompt (notre chat), pas encore les
gestes `open` / `close` — l'ouverture est la lecture elle-même.

### Suite du débat : le pourquoi voyage avec le diff

**Lucie** : « L'agent peut oublier pourquoi il avait édité telle chose,
puisqu'il ne voit plus le problème dans son historique. Il voit les diffs et
l'état final, de la manière la plus concise possible, et on colle la
réflexion de l'agent et la question de l'utilisateur aux diffs qu'il aura
dans son contexte. »

Ce que cela donne : dans la zone d'un fichier, l'état du jour, puis ses
modifications, chacune avec sa raison. C'est un micro-commit : un diff et son
message.

```
src/catalog/sync.rs — lu l. 580-640, état du jour
  [demande 3 : « l'annulation d'un instantané laisse des lignes »]
    l. 603-611 (+6 −2) — apply_snapshot_finish ne purgeait pas le cas annulé
    l. 620 (+1 −1) — même cause, second appel
```

Points en débat :

- **D'où vient la raison.** Le texte de l'agent juste avant l'appel coûte
  zéro, mais il peut être long, absent, ou parler d'autre chose. Un champ
  « pourquoi » d'une ligne à l'outil d'édition est voulu et court, mais
  demande de la discipline. Piste : le champ, et le texte précédent en repli.
- **La question de l'utilisateur ne se répète pas** : une demande amène vingt
  éditions ; elle se met en tête de groupe, une fois.
- **Le déclencheur n'est souvent ni la question ni la réflexion**, mais un
  résultat d'outil (une erreur de compilation, un test rouge). La raison doit
  pouvoir le citer en une ligne.
- **Diff net ou diffs dans l'ordre.** L'état final plus le diff net contre
  l'état d'ouverture est le plus concis ; les essais abandonnés (« essayé A,
  retiré ») y disparaissent, alors qu'ils disent ce qu'il ne faut pas
  refaire. Piste : diff net, et une ligne par essai abandonné.
- **Quand cela sert vraiment.** Tant que le fil n'est pas compressé, la
  réflexion y est encore. Cette structure est donc surtout ce qui **survit à
  la compression** — et c'est le L1 de l'archiviste tout fait : (demande,
  raison, diff, adresse), sans appel à un modèle.

### Ce que Lucie retient (4 octobre)

> « On voit chaque diff, c'est sûr, une par une, pas le diff à l'état final.
> On voit des commits, en gros, même si ce ne sont pas ceux de git, entre
> deux appels d'outils. Une raison optionnelle quand on fait une édition, et
> puis basta, en précisant dans la description que ça aide aux fiches
> contextuelles. »

Donc :

- **chaque édition est un commit** de la zone du fichier, dans l'ordre : pas
  de diff net. Les essais abandonnés restent visibles d'eux-mêmes, comme un
  commit et celui qui le défait ;
- **la raison est un champ optionnel** de l'outil d'édition, sans autre
  mécanisme (pas de repli sur le texte du fil, pas d'obligation) ; la
  description de l'outil dit à quoi elle sert : elle nourrit les fiches
  contextuelles ;
- la question de l'utilisateur reste attachée aux commits qu'elle a amenés.

**Quand la zone d'un fichier déborde** (Lucie) : « un fork de l'archiviste
en cours a ce boulot-là, de faire un petit résumé sur la fiche de contexte du
fichier, et on comprime les diffs en un seul ensuite. »

Donc, au débordement seulement :

- un fork de l'archiviste lit les commits anciens du fichier (diffs, raisons,
  demandes) et écrit quelques lignes sur la fiche de contexte du fichier ;
- ces commits sont ensuite fondus en un seul diff ; les commits récents
  restent un par un ;
- les originaux restent au journal, joignables par un outil.

Points en débat :

- le fork tourne à côté, il ne bloque pas le tour : la fusion s'applique
  quand le résumé est prêt, en une fois (le cache n'est perdu qu'à ce
  moment) ;
- le résumé peut être faux : les raisons données par l'agent y sont reprises
  telles quelles quand elles existent, le fork ne rédige que ce qui manque ;
- un essai abandonné disparaît du diff fondu : le résumé doit le dire en une
  ligne ;

**Retenu par Lucie** : le résumé **devient une fiche durable** liée au
fichier (`path:…`) à la fin de la session. Et : « un agent peut marquer une
fiche *done* pour obtenir juste un résumé de la fiche dans ses prochains
contextes, sans avoir besoin d'attendre la fin de session. »

Donc la fiche de contexte d'un fichier a des états :

| État | Ce que l'agent voit | Comment on y entre |
|---|---|---|
| ouverte | les plages lues au contenu du jour, les commits un par un | une lecture ou une édition |
| terminée (*done*) | le résumé seul, et les relations | l'agent la marque ; ou la fin de session |
| à revoir | le résumé, avec la mention de ce qui a changé | le fichier change après coup, par un autre |

- *done* est le geste « fermer » du curseur (§2) : l'ouverture est la
  lecture, la fermeture est *done*. Deux gestes suffisent.
- Marquer *done* lance le même fork de l'archiviste que le débordement ; tant
  que le résumé n'est pas prêt, la fiche reste entière.
- Relire ou éditer le fichier rouvre la fiche ; le résumé reste en tête.
- **Retenu par Lucie** : une fiche non touchée depuis longtemps reçoit un
  indice qui suggère à l'agent de la fermer s'il ne s'en sert plus. C'est une
  suggestion, jamais une fermeture : le harnais ne ferme de lui-même qu'au
  débordement, et le dit. À régler : l'indice se montre une fois, avec ce que
  la fiche coûte (« non touchée depuis 15 tours, ~900 jetons »), pas à chaque
  tour.
- C'est une machine à états comme celles des gabarits de mémoire : la fiche
  de contexte de fichier peut être un gabarit, pas du code à part.

### La personne dans les fiches, et l'heure partout

**Lucie** : « Les utilisateurs peuvent explorer dans l'interface les fiches
ouvertes et y glisser des notes ; les agents voient la date d'édition. Chacun
des tours de l'agent est marqué d'une heure et minute exactes, comme ça quand
on dit "tout à l'heure" il sait de quoi il s'agit. Et pour read et edit, il
ne voit que les paramètres et la référence de la fiche. »

Trois choses :

1. **Les fiches ouvertes se voient et s'annotent dans l'interface.** La
   personne voit ce que l'agent a ouvert, et pose une note sur une fiche : un
   canal de la personne vers l'agent accroché à un fichier, pas au fil de la
   conversation. Une note est une entrée de la fiche comme un commit, avec
   son auteur et son heure.
2. **L'heure partout.** Chaque tour, chaque commit, chaque note porte sa date
   et son heure, sur la même horloge : « tout à l'heure, vers 14 h » désigne
   la même chose dans le fil et dans les fiches.
3. **Dans le fil, une lecture ou une édition ne laisse que ses paramètres et
   la référence de la fiche.** Le contenu vit dans la fiche. Lucie précise :
   les paramètres eux-mêmes sont **rognés**, puisque la fiche les porte — une
   édition ne garde pas l'ancien et le nouveau texte (c'est le commit de la
   fiche), seulement le chemin, les lignes, la raison et la référence.

Points en débat :

- une note de la personne n'est jamais fondue dans un résumé de l'archiviste :
  ce sont ses mots, ils restent tels quels, y compris sur la fiche terminée
  et sur la fiche durable ;
- une note posée sur une fiche ouverte doit se signaler à l'agent au tour
  suivant (un événement « note de Lucie sur `X`, 14 h 32 »), sinon il ne la
  voit pas ; posée sur une fiche terminée, elle la fait passer « à revoir » ;
- l'auteur d'une entrée est toujours dit (la personne, cet agent, un autre
  agent) : l'agent ne doit pas prendre une note d'un autre agent pour une
  consigne de la personne ;
- l'heure seule ne suffit pas sur une session de plusieurs jours : date et
  heure, et « maintenant » redonné à chaque tour, en fin de prompt (l'heure
  des tours passés ne bouge pas, le cache est gardé) ;
- une édition qui échoue garde son erreur dans le fil : elle n'a rien écrit
  dans la fiche. Les résultats qui ne sont pas des fichiers (une commande,
  une recherche) restent dans le fil comme aujourd'hui.

## 8. Ce qui attend un choix de Lucie

1. Lancer l'essai 1 (petit, sans risque) dès qu'une session est libre, ou
   attendre que les fausses arêtes soient corrigées.
2. Le grain du curseur : scope d'abord (recommandé), ou fichier d'abord.
3. Les curseurs visibles entre agents dès le début, ou plus tard avec les
   écritures parallèles.
