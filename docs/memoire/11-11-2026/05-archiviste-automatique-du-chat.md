# L’archiviste automatique du chat — clarification de M9

11 novembre 2026. Conception issue de la discussion avec Lucie ; aucun raccordement automatique
n’est livré par ce document. Les comportements ci-dessous fixent le cap du chantier. Les noms
d’outils et d’événements proposés restent à confronter aux contrats du runtime.

Suivi du travail restant : [ticket M9, porté par la session mémoire](../tickets/2026-11-11-raccorder-l-archiviste-automatique-aux-frontieres-du-chat.md). Cette liaison n’ouvre aucun lot d’implémentation.

## 1. L’intention

Pendant que l’agent de code travaille avec Lucie, un archiviste examine les échanges terminés et
décide s’ils contiennent quelque chose de suffisamment significatif pour la mémoire : une
décision, une préférence durable, une correction, une connaissance acquise ou un résultat utile
pour la suite. Il peut consulter les fiches et ranger ces éléments avec leur provenance.

**À l’envoi d’un message utilisateur, il examine aussi le besoin de rappel pour le prochain
tour de l’agent de code.** Il peut chercher du contexte utile, puis décider de l’injecter ou
de s’abstenir. Si le contexte de l’agent paraît déjà suffisant pour la demande, il ne le
surcharge pas pour le seul plaisir d’avoir trouvé une fiche.

**Ne rien ranger est une réussite normale.** Un échange peut ne contenir aucune connaissance
durable, répéter ce qui est déjà connu ou rester trop incertain. Le traitement n’a aucun quota
d’écritures à remplir. Il doit pouvoir finir par un outil terminal, proposé sous la forme
`terminer(raison)`, sans créer ni modifier de fiche.

Cette abstention vaut aussi pour le rappel : il peut terminer sans recherche, ou après une
recherche sans rien injecter. Ranger et rappeler sont deux décisions indépendantes ; un même
passage peut faire les deux, une seule, ou aucune.

L’archiviste est un agent spécialisé avec une continuité par conversation : chaque déclenchement
ouvre un passage de travail, pas une nouvelle mémoire amnésique. Son activité reste consultable.
Il aide à entretenir la mémoire pendant la conversation, sans modifier les fichiers sur lesquels
l’agent de code travaille.

## 2. Quand le déclencher

Trois frontières alimentent le même traitement déclaré :

| Frontière | Ce qui est disponible | Ce qu’il peut examiner |
|---|---|---|
| Message utilisateur envoyé | Le message complet et le contexte disponible à cette frontière | Une décision à retenir et le besoin de contexte à rappeler avant le prochain tour de l’agent |
| Tour de l’agent terminé | L’ensemble du tour, ses appels d’outils et leurs résultats | Ce qui a été effectivement exécuté, obtenu, vérifié ou laissé en échec |
| Compression du contexte | La tranche concernée avant réduction et le repère du dernier passage | Un rattrapage de ce qui mérite d’être conservé avant que cette tranche quitte le contexte courant |

Pour l’utilisateur, l’envoi du message est la frontière : on n’attend pas la réponse de l’agent
de code. Pour l’agent, une réponse intermédiaire ou un appel d’outil ne clôt pas son tour : on
attend la fin du travail de ce tour, outils compris.

Un tour interrompu ou en erreur peut contenir des actions déjà réalisées et des enseignements.
Son état doit être visible de l’archiviste ; une action annoncée n’est jamais une action prouvée,
et un tour interrompu ne devient pas une réussite par le rangement de son résumé.

À la compression, il faut préserver la tranche ou une référence durable permettant de la relire
avant sa réduction. La décision de classement peut ensuite se faire en fond : la sûreté de la
source ne doit pas dépendre de la rapidité de réponse du modèle. Le seuil de compression n’est
pas fixé ici ; les 30 % de la vision historique ne constituent pas un réglage adopté.

## 3. Ce que voit l’archiviste

Il reçoit trois ensembles, sur un instantané identifié de la conversation :

1. **Le contexte disponible à l’agent de code à cette frontière** : échange avec Lucie,
   instructions applicables, résultats d’outils et résumés déjà présents. Le nouveau contenu
   depuis le dernier passage est clairement repéré. Les tranches anciennes se relisent par
   leurs sources lorsqu’elles sont nécessaires ; on ne prétend pas qu’un contexte compressé
   contient encore tout l’historique brut.
2. **Les fiches déjà ouvertes ou rappelées**, leur identité et leur version, avec le contenu
   pertinent pour savoir ce qui est déjà connu. Elles orientent le travail sans limiter le
   classement : l’archiviste peut rechercher et ouvrir une autre fiche pertinente.
3. **Son journal de passages antérieurs** : ce qu’il a examiné, retenu, écarté ou remplacé,
   mais aussi cherché, trouvé et effectivement injecté ; les fiches et leurs versions,
   les sources et les raisons courtes de ses décisions. Une recherche restée sans injection
   y figure aussi, pour éviter de refaire continuellement le même travail.

Les outils et traces détaillées de ses passages restent consultables. Son contexte courant
contient surtout des reçus compacts, par exemple : « décision X ajoutée à la fiche Y depuis
le message Z ; remplace l’ancienne décision W ». La raison est une justification courte et
vérifiable, pas une transcription de son raisonnement interne.

Le dépôt, la vue, la conversation et la mémoire cible restent explicites. Le contexte d’un
travail ne doit pas devenir, par défaut, une connaissance valable pour tous les projets.

## 4. Décider, ranger, rappeler ou terminer

Le passage peut consulter les sources et les fiches avant de décider. Il se termine après
les rangements et rappels utiles, ou sans aucun des deux. `terminer(raison)` est la forme proposée pour
clore explicitement ce passage ; le contrat exact et sa composition avec les outils existants
seront arrêtés à l’implémentation.

Exemples de fins sans écriture mémoire ni injection :

- `terminer("Échange de suivi sans nouvelle décision ni connaissance durable.")`
- `terminer("La préférence exprimée est déjà conservée dans la fiche ouverte, sans changement.")`
- `terminer("L’agent propose une cause, mais aucune vérification ne permet encore de la retenir.")`
- `terminer("Le contexte courant contient déjà la décision et le fichier nécessaires ; aucun rappel utile.")`
- `terminer("Recherche effectuée : les fiches trouvées n’apportent rien à la demande actuelle.")`

Cette fin écrit seulement un reçu dans le journal de l’archiviste. Sa raison ne devient pas
elle-même un apport sur une fiche : sinon « ne rien ranger » finirait encore par tout ranger.

Pour les éléments retenus :

- Une parole de Lucie garde son auteur et sa source. Une intention garde son statut d’intention.
- Un résultat de travail s’appuie sur les résultats observés, avec la portée des vérifications.
- Une déduction reste identifiée comme telle ; elle ne se transforme pas en parole de Lucie.
- Une correction peut remplacer une connaissance antérieure tout en conservant sa provenance.
- Une fiche déjà ouverte n’est pas une destination obligatoire ; aucune fiche pertinente peut
  aussi conduire à une abstention ou à une proposition selon la politique de création.

Les capacités de création et de modification dépendent de la politique de la mémoire cible.
Cette conception n’active pas implicitement la création de fiches, éteinte par défaut dans
`archive`, et ne lance aucune écriture dans la mémoire réelle de Lucie.

### Le rappel à la frontière utilisateur : deux portes et le silence possible

**Première porte : faut-il chercher ?** L’archiviste confronte la demande au contexte réellement
disponible à l’agent, au sommaire des fiches, aux fiches ouvertes et à ses reçus précédents.
S’il n’identifie aucun manque utile à combler, il peut appeler `terminer(raison)` immédiatement.
Le fait qu’un sujet ait des fiches n’oblige pas à les ouvrir toutes.

**Deuxième porte : faut-il injecter ce qui a été trouvé ?** Une recherche peut être utile pour
vérifier et finir sans injection. La sélection retient seulement ce qui apporte quelque chose
à la demande présente : une information manquante, une correction, une contradiction pertinente
ou un point à revoir. Une fiche similaire, ou première dans un classement, n’est pas pour
autant nécessaire au tour. Le classement des candidates et la décision d’en injecter sont
deux garanties différentes ; aucun seuil universel de score n’est adopté ici.

Le registre de session tient les recherches et les éléments montrés, avec leurs versions et
la génération du contexte. Une connaissance déjà présente ne revient pas à chaque tour.
Une fiche modifiée, un nouveau besoin ou une compression qui a retiré l’élément du contexte
peuvent justifier un nouveau rappel. Avoir injecté une fiche autrefois ne prouve pas que
l’agent la voit toujours.

Le rendu est court, nommé et sourcé, dans un budget de contexte. Un état L2 pertinent peut
suffire ; un apport L1 précis ou une source L0 restent accessibles pour vérifier ou approfondir.
L’agent de code conserve aussi la possibilité de demander lui-même une recherche mémoire.

Le rappel retenu pour une demande doit rejoindre le contexte à la frontière précédant le tour
de l’agent. Il ne peut pas être compté comme livré si le modèle a déjà commencé sans lui.
La durée de cette phase doit être bornée ; si son résultat arrive trop tard, son éventuelle
présentation relève d’une prochaine frontière explicite, en réévaluant son utilité. Le budget,
la priorité face à un classement de fond en cours et le comportement à expiration restent à
arrêter : un archivage lent ne doit pas imposer une attente sans borne à chaque message.

## 5. Un travail de fond ordonné et reprenable

Les passages d’une même conversation sont ordonnés. Si un message arrive pendant un passage,
sa frontière est conservée pour le suivant. L’archiviste travaille sur l’instantané annoncé,
sans relire silencieusement un état devenu différent au milieu de sa décision. Les échanges du
chat peuvent continuer pendant le classement de fond ; la phase de rappel avant le prochain
tour est coordonnée séparément avec le lancement de ce tour.

Le repère de progression distingue **ce qui a été examiné** de **ce qui a été mémorisé** :
une fin sans rangement avance aussi le premier repère. Sinon les mêmes échanges seraient
réexaminés indéfiniment. Un reçu relie le passage à sa frontière, aux sources examinées et,
le cas échéant, aux écritures effectuées.

La reprise doit éviter les apports en double et distinguer passage terminé, passage échoué
et effets partiellement réalisés. Une erreur de modèle ou d’écriture n’est pas une décision
« rien de pertinent » et n’avance pas le repère comme une réussite complète. Le mécanisme
exact d’atomicité ou de réconciliation est à établir avec les primitives existantes ; ce
document ne présume pas qu’une transaction englobe déjà fichiers, catalogue et journal.

Le classement automatique n’injecte pas rétrospectivement de contenu dans un tour de code en
cours. Le rappel est un geste distinct, à une frontière explicite, avec sa propre décision
de pertinence et son registre de ce qui a déjà été montré. Une abstention de rangement ne
supprime pas le besoin éventuel de rappel, et une abstention de rappel ne marque pas comme
rangées des connaissances qui ne l’ont pas été.

## 6. Composition avec le projet et les conceptions antérieures

Le chemin visé est : **frontière de conversation → traitement déclaré → agent spécialisé et
outils mémoire → reçu durable**. Le runtime porte les garanties nécessaires à l’exécution,
aux instantanés, à l’annulation et à la reprise. Les déclencheurs, les consignes et les outils
se composent par les contrats et graphes du système.

Lucie autorise l’agent du chat de l’atelier à lancer des sous-agents identiques ou spécialisés.
Elle réserve à une conception ultérieure un nouveau mécanisme intégré d’orchestration
multiagents. Le raccordement de cet archiviste ne doit pas servir de prétexte à développer
cet orchestrateur général.

Les documents à conserver ensemble sont :

- [Vision du 4 octobre, §9 et §9.2](../../../extension/rag3weaver/visions/2026-10-04-00h09-memoires-par-gabarit-et-fiches-qui-pointent-vers-tout.md) :
  les deux rythmes, le classement et les deux portes du rappel (« chercher ? », « injecter ? »),
  le budget et la lutte contre les répétitions. La présente clarification précise un passage
  de l’agent archiviste à chaque frontière utilisateur et agent ; elle ne suppose pas un appel
  de modèle supplémentaire après chaque résultat d’outil.
- [Étude du raccordement externe](../../../extension/rag3weaver/visions/2026-10-04-00h09-l-archiviste-branche-sur-claude-code.md) :
  les adaptateurs entre journaux de conversation, crochets et traitement mémoire.
- [Conception du 18 octobre, §3](../../../extension/rag3weaver/docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md) :
  la porte commune au rangement volontaire et automatique, le repère par session et le travail
  de fond dans l’hôte.
- [Découpage M4/M9 du 25 octobre](../../../extension/rag3weaver/docs/25-octobre-2026-memoire-longue/06-m4-l-archiviste.md) :
  M4 fournit `archive(document)` ; la porte commune, le repère et les déclencheurs sont M9.
- [Classeur local dans le rappel, 9 novembre](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md) :
  le contexte de conversation pour juger les candidates, le budget et la distinction entre
  leur classement et un seuil de pertinence. Cette étude vise les crochets externes ; ses
  choix de service et de modèle ne sont pas imposés au chat par la présente conception.

Le vocabulaire courant reste **Source = L0**, **Apport = L1**, **État de fiche = L2**. Un résumé
produit à la compression n’est pas automatiquement un apport L1 : son rôle et sa provenance
doivent être explicités avant son classement.

### La hiérarchie mémoire à préserver

Le raccordement automatique doit conserver les rôles et les garanties déjà conçus :

| Étage courant | Rôle | Ce qu’il faut préserver |
|---|---|---|
| **L0 — Source** | Le document reçu ou la source de la tranche examinée | Contenu immuable, origine et adresse ; un résumé produit reste distingué des événements bruts dont il provient |
| **L1 — Apport** | Une connaissance ou une section rangée sur une fiche | Date, auteur, distinction dit/déduit et liens vers ses sources ; plusieurs apports peuvent expliquer l’évolution d’un sujet |
| **L1bis — Apport consolidé** | Une consolidation de vieux L1 pour borner le travail ultérieur | Les liens `CONSOLIDE` vers les L1 d’origine, qui restent conservés ; pas de destruction des preuves |
| **L2 — État de fiche** | La synthèse du sujet à partir de ses apports | Les liens `LIT` vers les apports utilisés et `ETAT_DE` vers la fiche ; les décisions actuelles et leurs changements restent compréhensibles |

Le L2 est recalculé depuis les L1 pertinents, ou les L1bis qui les consolident, **jamais depuis
le seul L2 précédent**. Répéter un résumé de résumé ne doit pas devenir une source indépendante
qui finit par effacer les preuves. Une correction garde l’histoire de ce qu’elle remplace et
permet de redescendre jusqu’aux sources. Le reçu d’un passage reste un journal de traitement,
il n’est pas automatiquement un nouvel apport métier.

La vision du 4 octobre, §9.3, portait aussi une distinction temporelle : résumé de tranche
nommé L1, article du jour par sujet nommé L2, article principal intemporel nommé L3, refait
au-delà d’un budget par une synthèse au contexte limité aux articles sources. **Cette idée
reste une référence de conception.** Elle n’a pas la même nomenclature que le schéma courant
du 18 octobre : on ne doit ni l’effacer ni prétendre qu’un niveau L3 est déjà livré. La place
d’un article du jour et de sa synthèse globale dans les entités actuelles reste à réconcilier
avant implémentation, sans renommer silencieusement les étages.

Restent également à préserver le sommaire compact des sujets, les références structurées vers
les fichiers, symboles ou documents, le signal « à revoir » quand une source change, et la
possibilité de rechercher par ces références. Les budgets peuvent déclencher une consolidation
ou une proposition de scission d’un sujet, sans autoriser une réorganisation arbitraire.
Les modèles produisent des connaissances et synthèses persistées : une reconstruction d’index
relit ces artefacts ; elle ne les régénère pas en rappelant le modèle.

## 7. État, ordre proposé et preuves attendues

`archive(document)` et son correctif d’alias sont livrés. Le suivi automatique du chat décrit
ici, l’outil terminal proposé et son journal de passages ne sont pas livrés par cette note.
Le recalcul automatique des L2 et la consolidation ne sont pas acquis non plus.

Un ordre d’implémentation proposé, à confirmer lors de l’ouverture du lot :

1. La capture durable à la compression et le passage de rattrapage, avec reçu et reprise.
2. Les frontières de message utilisateur et de tour complet de l’agent, avec le même état
   d’archiviste, la fin sans écriture mémoire et les deux portes du rappel avant le tour agent.
3. Les traitements de synthèse L2 et de consolidation, dans leurs lots propres, en conservant
   la hiérarchie et les sources. L’utilité du rappel se mesure séparément du classement.

Avant de déclarer ce parcours livré, il faudra notamment prouver :

- Un échange sans information significative termine sans écriture de fiche, garde sa raison
  et n’est pas retraité au passage suivant.
- Une décision de Lucie est retenue dès son message ; un résultat de code attend les outils
  et garde leurs preuves. Une annonce ou une erreur ne devient pas un succès.
- Une fiche ouverte déjà à jour évite un doublon ; une fiche pertinente non ouverte peut
  être retrouvée et recevoir l’apport avec la bonne source.
- Un agent ayant déjà suffisamment de contexte ne reçoit aucune injection, avec une raison
  courte conservée. Une recherche qui ne trouve rien d’utile peut aussi finir sans injection.
- Une information manquante et utile arrive avant le tour visé, sourcée et sans doublon ; une
  fiche corrigée ou devenue absente après compression peut être rappelée de nouveau.
- Un L2 pertinent permet un rappel compact ; la lecture de ses L1 puis des sources restitue
  les preuves et les changements de décision. Une consolidation garde les L1 d’origine.
- Un nouveau message pendant l’archivage est traité ensuite dans l’ordre ; deux conversations
  conservent leurs contextes et repères distincts.
- Une compression et un redémarrage conservent la source, les reçus et les rangements acquis ;
  une écriture partielle ne provoque ni perte silencieuse ni doublon lors de la reprise.

Les réglages encore à arrêter comprennent le budget par passage et conversation, le modèle,
les outils effectivement accessibles, la politique de regroupement en cas de retard et le
contrat de récupération des effets partiels, ainsi que le budget de rappel avant le tour et
la place de la synthèse temporelle historique. Aucun chiffre de coût ou de latence n’est acquis.
