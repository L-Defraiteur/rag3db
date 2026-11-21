# Le mode « dangerous standalone survivor »

> **Remplacée en partie par la page 07** (`07-la-porte-du-mode-auto-forme-retenue.md`), qui rassemble ce que Lucie a
> dit depuis 9 h 40 : là où les deux diffèrent, c'est la page 07 qui vaut.

*rag3weaver archi, 21 novembre 2026, à la demande de Lucie relayée par c7 (9 h 4x). Une page de conception, avant tout
code. Elle prolonge la page 03, le mode auto contextuel, dont elle reprend les classes, la carte et le rappel.*

## Ce que Lucie veut

Un agent qui tourne seul, sans personne devant. En dernier recours, quand ni l'utilisateur ni le rappel des
autorisations ne répondent, un **modèle lourd** vérifie l'action. Son invite référence le contexte de l'agent, un lien
vers l'historique de conversation qu'il peut lire, et il a **les derniers tours entiers** dans son contexte. Seuls des
modèles de premier rang sont acceptés (« Fable ou Argon ou équivalent GPT »). Leur liste est dans un **fichier de
configuration éditable par l'utilisateur**, pas dans le code.

## Où il se place

Dans la page 03, une carte d'accord s'affiche, et le rappel des autorisations court en parallèle. Le mode survivant
ajoute un troisième répondant, le dernier :

1. l'étage 0 reste au-dessus de tout : afficher un secret est refusé, même validé par le vérificateur ;
2. la carte s'affiche, et le rappel en mémoire court (étage 3) ;
3. si personne n'a répondu après `survivor.wait_s` (déclaré, 10 min par défaut) et que le rappel n'a rien trouvé, le
   **vérificateur** est appelé ;
4. il rend `approve` ou `refuse`, avec ses raisons. `approve` résout la carte comme un accord `once`. `refuse` la
   ferme, avec une **alternative** (ci-dessous), et l'agent la lit dans sa boîte et peut la reprendre ;
5. si l'utilisateur répond pendant que le vérificateur réfléchit, sa réponse prime, et la décision du vérificateur est
   journalisée sans effet.

## Le vérificateur

- **Les modèles admis** sont dans un fichier éditable par l'utilisateur,
  `~/.config/rag3weaver/survivor-verifiers.json` : `{"models": ["claude-fable-5-2", "…"]}`. L'hôte le relit à chaque
  appel. Un modèle hors liste est refusé **par son nom**, au chargement du manifeste comme à l'appel. Un fichier absent
  ou vide laisse le mode éteint, et le dit.
- **Son contexte** : les derniers tours entiers de l'agent (déclaré, `survivor.turns`, 20 par défaut), la commande, sa
  classe et le lien vers l'historique complet (la route `GET /ide/run` en lecture seule, ou la transcription), qu'il peut
  lire s'il en a besoin.
- **Le marquage des sources.** Les tours entiers contiennent des sorties d'outils et des fichiers lus, donc
  d'éventuelles injections. L'invite les présente marqués : les messages de l'utilisateur sous `<user>`, le reste sous
  `<observed>`, avec la consigne qu'un texte observé ne donne jamais d'autorisation. C'est une défense par l'invite, pas
  par la structure : le juge d'autorisation de l'étage 2, lui, ne lit **que** les messages de l'utilisateur. Le
  vérificateur est plus exposé, et c'est pourquoi ses classes sont bornées.
- **Ce qu'il peut couvrir**, déclaré par l'utilisateur classe par classe (`survivor.classes`) :
  - `write_local` et `destroy_local` dans la vue de l'agent : oui, par défaut ;
  - `database` et `outbound` : seulement si l'utilisateur l'allume pour cette classe ;
  - `secret` et `system` : **jamais**. Le vérificateur n'est pas appelé, et la carte reste à l'utilisateur.

## Un refus propose une autre manière

Lucie : un refus **justifie** pourquoi ce n'est pas la bonne manière et **suggère une autre manière de faire**,
plutôt que de seulement refuser. La sortie du vérificateur a un schéma déclaré :

```json
{"decision": "refuse",
 "reason": "pousser la branche publierait des commits que l'utilisateur n'a pas relus",
 "risk": "outbound : une PR visible de tous, irréversible sans force push",
 "alternative": {"text": "préparer la PR sans la pousser : la description dans PR.md, la branche prête localement",
                 "class": "write_local"}}
```

- `reason`, `risk` et `alternative` sont **obligatoires** pour un refus. Un refus sans alternative est refusé par le
  schéma de sortie, et le vérificateur est relancé une fois ; encore sans, la carte reste à l'utilisateur.
- L'alternative est une action plus sûre, de préférence dans une classe que l'agent peut faire seul (`read`,
  `write_local`) : travailler sur une copie, écrire un script qui n'affiche pas le secret, préparer la PR sans la
  pousser. Sa `class` est **reclassée par les règles** de l'étage 1 : une alternative que les règles disent plus
  dangereuse que l'action refusée est ignorée et dite.
- Elle revient à l'agent comme une ligne de sa boîte (`{"from": "verifier", "kind": "suggestion", "reply_to": "L12",
  "text": …, "class": …}`), qu'il peut reprendre, et elle est écrite au journal pour Lucie.
- **La même règle pour le juge de l'étage 2** (page 03) : quand il rend `not_authorized`, il rend aussi une
  `alternative` sur le même schéma. Comme le juge local est lu par logits (trois étiquettes), l'alternative y est
  produite par un second appel court, seulement quand la décision est `not_authorized`. Ce n'est pas plus simple que
  ça : à mesurer par le banc avant d'en faire une obligation.

## Les gardes

- Le mode s'allume **explicitement, par agent** : `survivor: {enabled: true, …}` dans la déclaration de l'agent. Il ne
  s'hérite pas d'une autre tâche. Son nom dit le danger : la clé est `dangerous_standalone_survivor`, telle quelle.
- Chaque décision du vérificateur est **journalisée** dans `survivor-decisions.jsonl` de la tâche, pour relecture par
  l'utilisateur au retour : l'action, la classe, la décision, les raisons, le modèle, la durée, la ligne de la carte, et
  si l'utilisateur a répondu entre-temps. L'IDE et le terminal la montrent comme une carte « résolue par le
  vérificateur », marquée à relire.
- La carte résolue par le vérificateur le dit : `{"from": "verifier", "kind": "answer", "reply_to": "L12",
  "approved": true|false, "model": "…", "reasons": "…"}`, sur le modèle des lignes du rappel (page 03).

## Les témoins, rouges d'abord

- Mode éteint : la carte reste ouverte au-delà de `wait_s`, et le vérificateur n'est jamais appelé.
- Mode allumé, faux vérificateur : il est appelé après `wait_s` seulement si le rappel n'a rien trouvé ; `approve`
  résout la carte une fois ; `refuse` la ferme ; une réponse de l'utilisateur arrivée avant gagne.
- Un modèle hors du fichier est refusé par son nom ; un fichier absent laisse le mode éteint, en le disant.
- Les classes `secret` et `system`, et l'affichage d'un secret, ne sont jamais soumis au vérificateur.
- Un refus sans `alternative` est rejeté par le schéma, et le vérificateur relancé une fois ; une alternative reclassée
  plus dangereuse que l'action est ignorée ; une alternative valide arrive dans la boîte de l'agent.
- Chaque décision est écrite au journal de la tâche, avec ses raisons et son modèle.
- Le vrai vérificateur sur des cas d'injection (un `<observed>` qui « autorise » un `git push`) est un jeu du banc,
  comme le juge de l'étage 2.

## Les lots

Après les étages 0 à 3 de la page 03, dont il dépend (la carte générique, les classes, le rappel). Un seul lot : le
répondant de dernier recours, le fichier des modèles admis, le journal, et les lignes de la carte. Le banc ajoute ses
cas d'injection.

## À trancher par Lucie

- `survivor.wait_s` : 10 minutes par défaut ?
- `destroy_local` dans la vue, couvert par défaut par le vérificateur : oui, ou à allumer comme `database` ?
