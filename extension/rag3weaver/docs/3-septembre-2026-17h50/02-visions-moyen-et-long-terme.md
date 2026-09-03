# Ce qu'on veut faire, au-delà de la semaine

Rappel de la vision moyen et long terme, pour qu'une session compressée ne
confonde pas *la prochaine tâche* avec *la direction*. La feuille de route
détaillée est
[`docs/vision_roadmap_08_2026/`](../vision_roadmap_08_2026/00-index.md), quatorze
documents. Celui-ci en donne l'état au 3 septembre 2026.

## 1. La boucle étrange, et ce qui la conditionne

Le but : **un moteur qu'un agent peut lire, modifier, faire tourner et
interroger**, sans qu'un humain serve de courroie. Ce qui la conditionne n'est
pas la boucle elle-même — elle existe — mais ce dont elle a besoin pour servir
des projets réels.

**Un moteur qui ne sert que kuzu ne sert pas la production.** C'était le premier
verrou, et il vient de sauter : PostgreSQL est prouvé sur huit points. Ça change
la nature du projet — le catalogue n'est plus une couche au-dessus d'une base,
c'est une couche au-dessus de **plusieurs**.

Ce qui suit sur cet axe, par ordre :

- **Neo4j comme troisième backend.** Bonus de compatibilité, pas priorité :
  laisser les gens garder le graphe qu'ils ont. Cypher lui est natif (c'est le
  dialecte de référence, celui de kuzu en dérive), le plein texte Lucene et le
  vecteur sont natifs aussi. **Mais le sparse reste à nous** sur tous les
  backends, donc le contrat de décalage reste nécessaire — la bonne formulation
  n'est pas « un backend rend des décalages » mais « un décalage est nécessaire
  là où un index Rust vit à côté des données ».
- **Avaler une base étrangère** ([doc 13](../vision_roadmap_08_2026/13-avaler-une-base-existante.md))
  : lire un schéma qu'on n'a pas écrit et en proposer un graphe, en séparant ce
  qui est **déclaré** de ce qui est **deviné**. À faire **après** le pipeline de
  normalisation xlsx — décision explicite de Lucie : la question « quelles
  colonnes portent du texte cherchable » se rencontre là d'abord, sur un terrain
  plus simple.

## 2. Le schéma comme artefact

[Doc 14](../vision_roadmap_08_2026/14-le-schema-comme-artefact.md), écrit
aujourd'hui.

Une base change de forme par **deux chemins** et un seul laisse une trace. Le
chemin **déclaré** donne un fichier `.mmd` qu'on relit et qu'on commite —
`migrations.rs` est déjà un lanceur complet, avec checkpoints, `dry_run` et
`rollback`. Le chemin **induit** — `register_entity` qui émet du DDL à la volée,
`migrate_scope_columns`, `poser_index` — ne laisse **rien**.

Trois positions y sont prises :

- l'artefact doit porter **l'intention et non le SQL**, parce que le même
  changement s'écrit différemment sur chaque backend ;
- ce que ça débloque n'est pas d'abord « rejouer » mais **constater l'écart**
  entre ce que la configuration veut et ce que la base contient ;
- **un schéma n'est pas que des tables.** « Supabase-compatible » est une
  aspiration : une recherche vectorielle ne s'exprime pas en PostgREST, elle
  **exige** une fonction RPC. Donc aussi des fonctions, des politiques RLS, des
  droits — chacun un artefact.

## 3. Les quatre rôles d'agent

[Docs 09 à 12](../vision_roadmap_08_2026/00-index.md). La vision de Lucie, notée
le 30 août :

| rôle | tempérament | ce qu'il fait |
|---|---|---|
| **vision** | ambitieux, visionnaire, observateur | garde l'intention première de l'utilisateur, **jamais réduite** ; découpe en tickets ; note les bonnes idées à proposer |
| **design** | abstractionniste, volontaire, optimiste | tient le cap, approuve ou refuse, sert d'utilisateur à l'agent de code |
| **contexte** | — | juge si le contexte suffit ; peut **mettre en pause** l'agent de code par un `agentCanAnswer` à sortie structurée |
| **code** | attentif, minutieux, obéissant | le seul en mode auto/edit, le seul qui touche au code |

**La mémoire est bornée par la structure, pas par la taille** : l'unité est la
*manche* — parole du design, actions réelles de l'agent de code, réponse du
design. On garde les **deux dernières manches de design**, et l'intention
initiale n'est jamais réduite. C'est beaucoup plus léger que d'attendre un
million de contexte, et l'objectif reste stable.

Et une subtilité que Lucie a posée : au moment où l'agent de contexte décide
s'il peut répondre, il n'a **que** cette sortie structurée — sans les outils de
recherche, sinon il cherche le contexte deux fois.

## 4. Ce qui manque au moteur pour avaler le réel

- **codeparsers, couverture complète** : cahier des charges écrit
  ([spec](../30-aout-2026-06h00/), déplacée chez eux), mesuré depuis — le pavage
  était déjà à 98,2 %, le vrai problème était ailleurs et bien plus gros.
- **La normalisation xlsx**, avant l'ingesteur de base étrangère.
- **La recherche web** : Gemini *grounded* via Vertex, un repli sans Gemini, et
  `fetch`. Première fois qu'un agent sortirait de la machine — surface à revoir
  avant d'écrire.
- **L'agent qui écrit son propre outil** : bloqué sur un point d'architecture,
  `GraphToolBox` tient un `&GraphToolRegistry` immuable là où `attach` veut
  `&mut self`. Forme proposée : une seconde couche mutable en `Arc<RwLock<…>>`,
  fusionnée dans `tool_defs()`, **les intégrés gagnant** pour qu'un agent ne
  puisse pas masquer `run` ou `edit`.

## 5. La concurrence, et jusqu'où on veut aller

Trois configurations, et une seule est acquise :

| | avant | aujourd'hui |
|---|---|---|
| plusieurs fils écrivains, un processus | non | **oui** (Vela, non adopté) |
| plusieurs processus **lecteurs** | non | **oui** (report fait) |
| plusieurs processus **écrivains** | non | **non** |

La troisième est celle qu'on voulait, et aucun des deux forks ne la résout. Elle
demandera un gestionnaire de verrous **inter-processus**, et on ne peut pas en
poser un sur une couche de stockage qui suppose un écrivain unique — d'où
l'intérêt du travail de Vela comme **plancher**, et la nécessité de le relire
avant d'y bâtir.

`rag3daemon` est la réponse d'aujourd'hui : un processus détient la base, les
autres lui parlent. Ce qui vient d'être ouvert permettra aux **lecteurs** de s'en
passer — sous les deux conditions du [rapport de session](01-rapport-de-session.md).

## 6. Ce qui reste ouvert et non décidé

- Où vit la configuration des familles de commandes `Toujours`.
- Comment demander à un humain quand il n'y a pas de terminal.
- L'arbitre GPU **entre processus** : le verrou du démon ne sérialise que les
  embarquements ; rien n'empêche llama.cpp de prendre la même carte.
- L'agentique vers Gemini — la moitié manquante du régime `confort`.
- Le genre du parseur générique de codeparsers, `.cypher`, `.java`.
- Le renommage `kuzu` → `rag3db` : 2 899 fichiers à chaque reprise de l'amont,
  pour un bénéfice que personne n'a énoncé.
