# `provider` et `protocol` : les mots d'un modèle déclaré sont à l'envers de l'usage

- **État** : ouvert (15 novembre 2026). Demande de Lucie : « il faut être cohérent sur les wording ». Après l'atterrissage de `models.llm` en liste (2b3340520), et avec lui si le lot n'est pas encore poussé.
- **Famille** : produit
- **Gravité** : planification. Aucune panne : un vocabulaire qui trompe celui qui écrit un manifeste et celui qui lit `/model`.
- **Atteignable en service** : non, c'est un choix de noms.
- **Touche rag3weaver** : oui (`src/model_source.rs` : `ModelSource`, `Provider` ; les manifestes de backend et de tâche qui déclarent `models` ; la lecture par `ChatConfig` ; le README et `docs/` qui montrent un manifeste).
- **Pour** : everything declarative
- **Signalé par** : c7 (orchestration), sur la question de Lucie du 15 novembre 2026 (page `docs/orchestration/14-11-2026/03-la-file-en-pile.md`).


## Tranché par Lucie (15 novembre 2026, par c7)

- **`access`** pour la manière d'accéder : `local`, `service`, `compatible`.
- **`provider`** pour le fournisseur : `openai`, `llama_server`, `vertex`, `anthropic`…
- **Les alias** : les anciens mots restent lus **deux semaines**, jusqu'au **29 novembre 2026**, date dite
  dans la ligne de nouveautés. Le retrait se fait en lot à part.
- La transition, conçue avec c7 :
  - un `provider` qui vaut `local`, `service` ou `compatible` se lit avec l'ancien sens ;
  - `protocol` est l'alias du nouveau `provider` ;
  - un mélange des deux formes dans un même modèle est refusé, en le nommant.
- Un seul vocabulaire pour tous les `models.*` (embed, sparse, rerank, ocr, decide, llm), et les
  gabarits livrés réécrits dans la forme neuve.
- **Sa place** : après l'écrivain de 2a′-2, la mesure (3a)/(3a′) et la page de 2a′-3.

## Ce que c'est

Un modèle déclaré dit trois choses : **la manière d'accéder** (notre moteur dans le processus, un démon à nous, un service tiers), **le fournisseur tiers** (OpenAI, llama-server, Vertex, Anthropic) et **le nom du modèle**. Dans `ModelSource`, la manière d'accéder s'appelle `provider` (`local`, `service`, `compatible`) et le fournisseur tiers s'appelle `protocol` (`openai`, `llama_server`, `vertex`, `anthropic`). Partout ailleurs, « provider » désigne le fournisseur. Qui lit `provider: compatible` croit lire un fournisseur, et cherche le vrai sous un mot qui promet un protocole réseau.

## La demande

Choisir un seul vocabulaire et le tenir dans le code, les manifestes, le terminal et les docs :

- la manière d'accéder prend un mot qui le dit (par exemple `access`, ou `where` : `local`, `service`, `compatible`) ;
- le fournisseur tiers prend `provider` (`openai`, `llama_server`, `vertex`, `anthropic`), puisque c'est ce que le mot veut dire pour tout le monde ;
- `model` reste le nom du modèle ; `name` reste la clé stable de la liste.

Le même changement vaut pour les modèles d'embarquement et de reclassement, qui passent par le même `ModelSource` : un seul vocabulaire pour `models.*`.

## Ce qui ne doit pas casser

Les manifestes qui existent (`.rag3weaver/`, bases de Lucie, exemples des docs, `tests/`) lisent encore. Un alias serde sur les anciens mots pendant une période dite, puis un refus nommé qui donne le nouveau mot. Le code suit la règle des identifiants en anglais ; les docs et le terminal disent la même chose en français.

## Le témoin

Un test de désérialisation par forme : l'ancienne (`provider: compatible`, `protocol: vertex`) et la nouvelle lisent le même `ModelSource` ; un manifeste qui mélange les deux est refusé par un message qui cite les deux mots. Le README montre la nouvelle forme seule.

## Ce qu'il faut pour le fermer

Le renommage, ses alias, le témoin, les manifestes et docs du dépôt réécrits, et la ligne de nouveautés.
