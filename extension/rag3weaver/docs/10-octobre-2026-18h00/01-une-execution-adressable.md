# Une exécution adressable — ce qu'il faudrait, et ce qui est déjà là

10 octobre 2026, session mémoire longue. Demandé par l'orchestration : ce qu'il
faudrait pour qu'un run soit une adresse (`run:…`) citable par une mémoire ou
une fiche, **en passant par les verbes de l'IR (`Write`) et pas par son
Cypher**. Une page, pas de code.

Je contredis la consigne sur deux points, et je le dis d'emblée parce que ça
change l'ordre du travail.

## Ce qui existe, lu et non supposé

`dataflow/record.rs` écrit déjà une exécution en base : `_DataflowExecution`,
et sous elle `_DataflowNodeRun` / `_DataflowEdgeRun`. Quatre faits comptent.

**Son identité est déjà une adresse — mais irrécupérable.**
`hashsafe_uuid("_DataflowExecution", [pipeline_name, now_ms])` : déterministe à
partir du nom et de la **milliseconde**. Donc personne ne peut la recalculer, et
`record()` rend `Result<(), RecordError>` — il ne la **rend pas**. L'adresse
existe, elle est fabriquée, et elle est jetée dans la même fonction.

**Elle écrit par Cypher brut**, `conn.execute_with_params`, pas par les verbes
du catalogue. Elle contourne donc tout ce qui va avec eux : pas de garde de
cycle de vie, pas de `CatalogEvent::EntitiesChanged` — donc aucune réaction ne
peut se déclencher sur un run —, pas de signaux, pas d'`EntityConfig`.

**Ses tables commencent par `_`** : des tables système, hors du catalogue
déclaré. Une ancre (`ANCHORED_TO`) ou une citation (`CITE`) ne peut pas les
viser : la traversée passe par les entités déclarées.

**Et personne ne l'appelle en production.** Les deux seuls appelants de
`DataflowRecorder` sont dans `tests/e2e_dataflow_observe.rs`. Aucun site dans
`src/`. C'est, une fois de plus, le défaut de la semaine sous un nouveau
visage : l'instrument est écrit, complet, testé — et rien ne l'allume.

## Premier désaccord : `Write` n'existe pas

`ir/src/form.rs:6` le dit lui-même : « Aujourd'hui : `Hop`. Viendront `Count`,
`Select`, `Write` et `Tx`. » **L'IR est en lecture seule.** Passer par `Write`
ne serait donc pas emprunter un chemin existant, ce serait **ouvrir le premier
verbe d'écriture de l'IR** pour y faire passer la journalisation d'un run.

C'est un gros premier client pour une petite question. Et un mauvais premier
client : la forme d'écriture de l'IR sera jugée sur les cas du produit —
ingestion, transition, relation —, pas sur une table système à trois champs
qu'aucun dialecte sauf le nôtre n'a de raison de porter. Je recommande
l'inverse : que `Write` naisse de l'ingestion, et que le run **y passe ensuite**
comme n'importe quelle écriture.

## Second désaccord : l'adresse ne demande aucun schéma neuf

C'est la découverte de cette lecture, et elle rend la question beaucoup moins
chère que posée. **`Ref` est déjà l'adressage, et il est déjà déterministe.**

Dans le gabarit `memory`, `Ref` déclare `hashsafe: ["genre", "valeur"]` : l'uuid
d'une référence **se recalcule** depuis son genre et sa valeur. Donc
`Ref{genre: "run", valeur: "<uuid de l'exécution>"}` est une adresse citable
**aujourd'hui**, sans table neuve, sans champ neuf, sans `Write` :

- deux mémoires qui citent le même run convergent sur le **même nœud** `Ref` —
  c'est la propriété que `hashsafe` achète, et c'est exactement ce qu'on veut
  d'une adresse ;
- `CITE` existe, `add_ref` existe, `create_ref_type` existe. Un genre `run`
  s'apprend par le verbe prévu, avec sa description et, si on veut, un harnais
  rhai qui valide la forme de la valeur ;
- et une référence dont la cible a disparu est déjà prévue : `Ref` porte un
  `etat` avec un cycle de vie, et une référence non vérifiée est « citée, non
  vérifiée » plutôt que perdue.

Ce qui manque n'est donc pas un modèle. Ce sont **trois pas**, tous petits :

1. **que `record()` rende l'adresse** au lieu de la jeter —
   `Result<String, RecordError>`, l'uuid de l'exécution ;
2. **que l'adresse soit recalculable** : aujourd'hui elle dépend d'une
   milliseconde. Il faut la dériver de ce qui caractérise le run — à mon avis
   `graph_hash` (déjà stocké), les entrées, et l'instant **arrondi** ou un
   compteur de session. Sans quoi « citer le run d'hier » exige de l'avoir noté
   quelque part au moment où il tournait ;
3. **que quelque chose l'allume.** Aucun appelant dans `src/` : tant que le
   backend ne construit pas de `DataflowRecorder`, il n'y a pas de run à citer.

## Ce que `Write` apporterait quand il viendra, et ce qu'il n'apporte pas

Il apporterait une chose réelle et une seule : que l'écriture d'un run **passe
par les mêmes gardes que le reste** — cycle de vie, événements, donc une
réaction déclenchable sur « un run a fini ». C'est ce qui rendrait un run
observable par le réacteur, et ce n'est pas rien.

Il n'apporte **pas** l'adressabilité : elle vient de `hashsafe`, pas du verbe
d'écriture. Confondre les deux ferait attendre un chantier d'IR pour une
propriété qu'on a déjà.

## Ce que je ferais, dans cet ordre

1. les trois pas ci-dessus, dont le troisième est une décision de produit : dans
   quel cas un backend enregistre-t-il ses exécutions ? Je proposerais : jamais
   par défaut, une clé de manifeste pour l'activer, et la rétention déjà écrite
   (`RecordRetention`) pour que ça ne grossisse pas sans fin ;
2. un genre `run` déclaré dans le gabarit `memory`, avec sa description — un
   genre de trop se paie longtemps, celui-là se justifie ;
3. **la question qui décide, et que je ne tranche pas** : faut-il que
   `_DataflowExecution` devienne une entité **déclarée** ? Si oui, on peut
   ancrer une mémoire dessus directement (`ANCHORED_TO`) et chercher dans les
   runs comme dans le reste ; si non, on y arrive par un `Ref`, et le run reste
   une chose du moteur qu'on cite sans l'indexer. La première réponse est plus
   puissante et plus coûteuse — elle met l'historique d'exécution dans le
   catalogue du produit, avec ses signaux et son embarquement.

## Ce que je ne sais pas

- **Ce qu'une mémoire veut vraiment citer d'un run.** « Le run » ou « ce que le
  run a rendu » ? Une recherche qui a donné un résultat faux se cite par son
  exécution ; une correction se cite par le fichier qu'elle a changé. Les deux
  visions se croisent ici et aucune ne le dit.
- **Si une adresse de run doit survivre à la rétention.** `RecordRetention`
  supprime au bout de 100 runs ou 30 jours. Une mémoire qui cite un run
  supprimé se retrouve avec une référence pendante — ce que `Ref.etat` sait
  dire, mais il faudrait que quelqu'un le mette à jour. Supprimer un run cité
  est peut-être simplement interdit.
