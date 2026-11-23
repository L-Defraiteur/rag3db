# Les tickets intégrés : conception

23 novembre 2026, au soir. La présentation de rag3weaver annonce des tickets intégrés (« Built-in Tickets ») ; rien
n'en est codé. Ce document part des décisions de Lucie du même jour, dit sur quoi on bâtit, et propose les lots. Il
est soumis avant tout code.

## Les décisions de Lucie (23 novembre)

1. **Des données de l'hôte, avec un miroir en `.md` parsable**, « comme la mémoire » ; le miroir sert aussi à déboguer.
2. **Les champs** : `title`, `epic`, `type` libre (`fix` et `feature` livrés, avec des alias), `state`, `assignee`
   (vide permis). **Chaque titre `##` du `.md` est un champ.**
3. **Des liens pour ce qui vit ailleurs, des pièces jointes (livrées) pour ce qui peut disparaître.**
4. **Un outil `my_tickets`** : l'action rapide qui liste les tickets assignés à moi.
5. **Une liste de tickets partagée dans un swarm**, lisible par le super-orchestrateur, et par les autres swarms
   **sur demande, par l'outil**.
6. À côté (le lot des swarms) : le titre d'un swarm est une base plus un suffixe ; un agent n'édite que le suffixe ;
   le refus propose « demande à l'utilisatrice `/commande` » ou « demande au super-orchestrateur, avec justification
   et paramètres ».

## Le principe : une ressource de l'hôte, trois clients

Un ticket est une ressource de l'hôte. Les agents la lisent et l'écrivent par des outils déclarés en gabarits, l'IDE
par ses routes, et le service sans écran par `/v1/tickets` (la façade, voir la conception du service du même jour).
Rien n'est propre à un client : un agent local sans service a ses tickets, et le service les sert sans code à part.

## Sur quoi on bâtit

- **Un backend déclaré.** Une organisation nouvelle se décrit dans le catalogue d'entités d'un manifeste, pas dans le
  moteur. Les tickets sont un backend livré, `builtin:backends/tickets`, comme `memory` ou `notebook` : aucune ligne
  du moteur ne connaît le mot « ticket ».
- **Le miroir en fichiers de la mémoire** (`src/fichiers_entite.rs`) : un fichier Markdown par ligne, un en-tête YAML
  canonique qui porte l'uuid, le format et les champs déclarés, le format versionné avec ses migrations, et le
  fichier composé (un gabarit, relu par son bloc machine) réécrit par la réaction de miroir quel que soit l'écrivain.
- **La clé d'agent** (`agent:<clé>`), stable à travers les relances : c'est elle qu'on assigne, et elle signe
  l'auteur, comme dans la mémoire.
- **Le bus** : chaque écriture publie son événement, que l'IDE, le flux de `/v1` et les réveils lisent.

## 1. Les entités

Déclarées dans le manifeste livré, modifiables par l'utilisatrice dans sa copie.

- **`Ticket`** : `uuid`, `number` (un numéro court par magasin, pour dire « T-42 »), `title`, `epic` (vide permis),
  `type`, `state`, `assignee` (une clé d'agent, `user`, ou vide), `swarm` (vide permis), `author`, `created_at`,
  `updated_at`, puis les **champs de sections** (ci-dessous).
- **`TicketLink`** : `ticket`, `kind` (`relates`, `blocks`, `duplicates`, `parent`), `target` : une adresse
  (une PR, un ticket d'un autre outil), un autre ticket, un commit, un `chemin:ligne@commit`, une fiche de la mémoire.
- **`Attachment`** : `ticket`, `name`, `media_type`, `sha256`, `size`. Le contenu est copié dans le magasin, rangé par
  son empreinte, jamais un chemin vers l'original : une pièce jointe sert justement ce qui peut disparaître (une
  capture, un journal, un fichier de `/tmp`).
- **`TicketNote`** : `ticket`, `author`, `at`, `text`. C'est le journal du ticket, en ajout seul : ce qu'un agent a
  essayé, ce que l'hôte a fait atterrir, une relecture. Proposition de ma part : Lucie n'en a pas parlé, mais un
  ticket sans journal oblige à réécrire ses champs pour dire ce qui s'est passé.

**`type` et `state` viennent du gabarit, pas du code.** Le manifeste livré déclare les types `fix` et `feature` avec
leurs alias (`bug` → `fix`, `feat` → `feature`), et accepte tout autre type, puisque le type est libre. Il déclare
aussi les états `open`, `in_progress`, `blocked`, `done` et `dropped`, `open` par défaut. Une utilisatrice change ces
listes dans sa copie du gabarit, sans bâtir.

## 2. Le miroir `.md`

Un fichier par ticket, `<number>-<slug du titre>.md` :

```markdown
---
uuid: 0f3c…
format: 1
number: 42
title: La recherche rend un fichier supprimé
type: fix
state: in_progress
assignee: agent:7c1e…
epic: recherche
links:
  - relates https://github.com/…/pull/612
attachments:
  - capture.png sha256:9ab1… 48 Kio
---

## Description
…

## Reproduction
…

## Fini quand
…

## Journal
- 2026-11-23 21:04, agent:7c1e… : …
```

- **Chaque titre `##` est un champ**, ce que le miroir ne sait pas faire aujourd'hui : un fichier simple y a un seul
  corps, et un fichier composé se relit par son bloc machine, pas par ses sections. C'est un mode de corps de plus,
  `sections`, ajouté à `fichiers_entite.rs` pour toute entité (la mémoire pourra s'en servir). Le gabarit déclare les
  sections connues, chacune avec sa clé et son libellé (`description` sous « Description », `done_when` sous « Fini
  quand »). Un titre inconnu devient un champ libre, gardé tel quel et relu à l'identique. Le `## Journal` est rendu
  depuis les `TicketNote`.
- **Où vit le miroir** : par défaut dans les données de l'hôte (`<data>/tickets/<magasin>/`). Un projet peut déclarer
  la racine de ses fichiers dans son dépôt (`docs/tickets/`) : ses tickets voyagent alors avec git et se relisent dans
  une PR, comme les tickets d'aujourd'hui.
- **Qui fait foi** : la base, et le fichier la suit après chaque écriture (la réaction de miroir). Un fichier modifié
  à la main, par l'utilisatrice ou par un agent avec ses outils de fichier, est relu par la surveillance et entre dans
  la base comme une écriture de l'auteur `file`. Si la base a changé entre-temps, la dernière écriture gagne, et une
  note du journal le dit avec les deux valeurs. La base se rebâtit depuis les fichiers, comme celle de la mémoire.

## 3. Les outils de l'agent

Des gabarits d'outils comme les autres, avec leur description pour le modèle :

| outil | ce qu'il fait |
|---|---|
| `my_tickets` | les tickets assignés à ma clé, ni `done` ni `dropped`, rangés par état puis par date |
| `tickets` | une recherche : texte par le sens et le plein texte, filtres `epic`, `type`, `state`, `assignee`, `swarm` |
| `ticket` | un ticket entier, avec ses liens, la liste de ses pièces jointes et son journal |
| `ticket_open` | ouvrir un ticket (titre, type, sections, épopée, assignation) ; rend son numéro |
| `ticket_update` | changer des champs ou l'état ; un état inconnu du gabarit est refusé, la liste dite |
| `ticket_note` | ajouter au journal |
| `ticket_link` / `ticket_attach` | lier, joindre un fichier (copié, avec son empreinte) |

Une pièce jointe passe le même contrôle que les racines accordées : rien qui vienne d'un dossier protégé (le jeton de
l'hôte, un profil de navigateur, `.vault`, `~/.ssh`), et une taille maximale déclarée.

## 4. Le partage : le swarm, le super-orchestrateur, les autres

- **Avant que le swarm soit un objet**, un magasin par hôte : la liste de l'utilisatrice. Le champ `swarm` y est
  vide. Ce lot n'attend donc pas celui des swarms.
- **Quand le swarm existe** : un magasin par swarm, lu et écrit par ses membres. Le super-orchestrateur lit tous les
  magasins.
- **Un autre swarm lit sur demande, par l'outil** : `tickets {swarm: "<titre>"}`, rendu en lecture seule. La lecture
  est notée dans le journal du magasin lu (qui, quand, quelle recherche), sans carte. Écrire dans la liste d'un autre
  swarm est refusé, avec ce que le refus propose (décision 6) : ouvrir le ticket chez soi et le lier, ou demander au
  super-orchestrateur avec une justification et des paramètres.

## 5. L'IDE et `/v1`

- **`/v1/tickets`** : `GET`, `POST` et `PATCH /v1/tickets/{id}`, les mêmes champs que les outils.
- **`sessions.start({ticket})`** : une session part d'un ticket. Son premier tour reçoit le ticket rendu par un
  gabarit, le ticket est assigné à la clé de la session et passe en `in_progress`. À chaque fin de tour, l'hôte ajoute
  au journal le code de fin, et les commits atterris s'il y en a.
- **Les événements** `ticket.opened`, `ticket.changed` et `ticket.closed`, sur le bus, dans le flux de l'IDE et dans
  le SSE de `/v1`.
- **La page de l'IDE** (liste, filtres, un ticket) : un lot à part, pour qui tient l'IDE.

## 6. Les événements du dehors, plus tard

Ils viennent après les tickets et s'appuient sur eux : un événement (« une PR touche à ton code », « conflit avec ta
branche ») arrive dans la boîte de l'agent concerné, routé par l'index (`impact` sur le diff de la PR) ou par un
`merge-tree`, et peut ouvrir un ticket. Cela fera sa propre conception.

## Les lots, chacun avec ses témoins rouges d'abord

1. **Les sections comme champs** dans `fichiers_entite.rs` (générique) : un fichier écrit, relu à l'identique ; un
   titre inconnu gardé ; un libellé ou une clé reconnus.
2. **Le backend livré et le magasin de l'hôte**, avec `my_tickets`, `tickets`, `ticket`, `ticket_open`,
   `ticket_update` et `ticket_note`. Le témoin : un agent ouvre un ticket, un second le trouve par `my_tickets` après
   assignation, et le miroir se relit en la même ligne.
3. **Les liens et les pièces jointes** : une pièce jointe survit à l'effacement de l'original, et un dossier protégé
   est refusé.
4. **`/v1/tickets` et `sessions.start({ticket})`**, après la façade `/v1/sessions`.
5. **Le partage par swarm**, après l'objet swarm.

Le lot 1 peut partir dès l'accord. Le lot 2 ne dépend que de lui.

## Ce que j'ai tranché, et que Lucie peut défaire

- La base fait foi, et un fichier modifié à la main est relu : la dernière écriture gagne, et le journal le dit.
- Le journal (`TicketNote`) : une entité de plus, proposée.
- Les états livrés : `open`, `in_progress`, `blocked`, `done`, `dropped`.
- Les tickets d'aujourd'hui (`docs/<session>/tickets/*.md`) ne sont pas importés dans ces lots. Un importeur, s'il est
  voulu, sera un lot à part : leur forme est proche, mais pas identique.
