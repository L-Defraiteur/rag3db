# L’archiviste automatique du chat — clarification de M9

11 novembre 2026. Conception issue de la discussion avec Lucie ; aucun raccordement automatique
n’est livré par ce document. Les comportements ci-dessous fixent le cap du chantier. Les noms
d’outils et d’événements proposés restent à confronter aux contrats du runtime.

## 1. L’intention

Pendant que l’agent de code travaille avec Lucie, un archiviste examine les échanges terminés et
décide s’ils contiennent quelque chose de suffisamment significatif pour la mémoire : une
décision, une préférence durable, une correction, une connaissance acquise ou un résultat utile
pour la suite. Il peut consulter les fiches et ranger ces éléments avec leur provenance.

**Ne rien ranger est une réussite normale.** Un échange peut ne contenir aucune connaissance
durable, répéter ce qui est déjà connu ou rester trop incertain. Le traitement n’a aucun quota
d’écritures à remplir. Il doit pouvoir finir par un outil terminal, proposé sous la forme
`terminer(raison)`, sans créer ni modifier de fiche.

L’archiviste est un agent spécialisé avec une continuité par conversation : chaque déclenchement
ouvre un passage de travail, pas une nouvelle mémoire amnésique. Son activité reste consultable.
Il aide à entretenir la mémoire pendant la conversation, sans modifier les fichiers sur lesquels
l’agent de code travaille.

## 2. Quand le déclencher

Trois frontières alimentent le même traitement déclaré :

| Frontière | Ce qui est disponible | Ce qu’il peut examiner |
|---|---|---|
| Message utilisateur envoyé | Le message complet et le contexte disponible à cette frontière | Une décision, une préférence, une correction ou une intention exprimée par Lucie |
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
3. **Son journal de rangement antérieur** : ce qu’il a examiné, retenu, écarté ou remplacé,
   les fiches concernées, les sources et les raisons courtes de ses décisions.

Les outils et traces détaillées de ses passages restent consultables. Son contexte courant
contient surtout des reçus compacts, par exemple : « décision X ajoutée à la fiche Y depuis
le message Z ; remplace l’ancienne décision W ». La raison est une justification courte et
vérifiable, pas une transcription de son raisonnement interne.

Le dépôt, la vue, la conversation et la mémoire cible restent explicites. Le contexte d’un
travail ne doit pas devenir, par défaut, une connaissance valable pour tous les projets.

## 4. Décider, ranger ou terminer

Le passage peut consulter les sources et les fiches avant de décider. Il se termine soit après
les rangements utiles, soit sans aucun rangement. `terminer(raison)` est la forme proposée pour
clore explicitement ce passage ; le contrat exact et sa composition avec les outils existants
seront arrêtés à l’implémentation.

Exemples de fins sans écriture mémoire :

- `terminer("Échange de suivi sans nouvelle décision ni connaissance durable.")`
- `terminer("La préférence exprimée est déjà conservée dans la fiche ouverte, sans changement.")`
- `terminer("L’agent propose une cause, mais aucune vérification ne permet encore de la retenir.")`

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

## 5. Un travail de fond ordonné et reprenable

Les passages d’une même conversation sont ordonnés. Si un message arrive pendant un passage,
sa frontière est conservée pour le suivant. L’archiviste travaille sur l’instantané annoncé,
sans relire silencieusement un état devenu différent au milieu de sa décision. Les échanges du
chat peuvent continuer pendant ce travail de fond.

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
de pertinence et son registre de ce qui a déjà été montré.

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

Le vocabulaire courant reste **Source = L0**, **Apport = L1**, **État de fiche = L2**. Un résumé
produit à la compression n’est pas automatiquement un apport L1 : son rôle et sa provenance
doivent être explicités avant son classement.

## 7. État, ordre proposé et preuves attendues

`archive(document)` et son correctif d’alias sont livrés. Le suivi automatique du chat décrit
ici, l’outil terminal proposé et son journal de passages ne sont pas livrés par cette note.
Le recalcul automatique des L2 et la consolidation ne sont pas acquis non plus.

Un ordre d’implémentation proposé, à confirmer lors de l’ouverture du lot :

1. La capture durable à la compression et le passage de rattrapage, avec reçu et reprise.
2. Les frontières de message utilisateur et de tour complet de l’agent, avec le même état
   d’archiviste et la fin sans écriture mémoire.
3. Le rappel automatique selon ses deux portes, avec une mesure séparée de son utilité.

Avant de déclarer ce parcours livré, il faudra notamment prouver :

- Un échange sans information significative termine sans écriture de fiche, garde sa raison
  et n’est pas retraité au passage suivant.
- Une décision de Lucie est retenue dès son message ; un résultat de code attend les outils
  et garde leurs preuves. Une annonce ou une erreur ne devient pas un succès.
- Une fiche ouverte déjà à jour évite un doublon ; une fiche pertinente non ouverte peut
  être retrouvée et recevoir l’apport avec la bonne source.
- Un nouveau message pendant l’archivage est traité ensuite dans l’ordre ; deux conversations
  conservent leurs contextes et repères distincts.
- Une compression et un redémarrage conservent la source, les reçus et les rangements acquis ;
  une écriture partielle ne provoque ni perte silencieuse ni doublon lors de la reprise.

Les réglages encore à arrêter comprennent le budget par passage et conversation, le modèle,
les outils effectivement accessibles, la politique de regroupement en cas de retard et le
contrat de récupération des effets partiels. Aucun chiffre de coût ou de latence n’est acquis.
