# La couche utilisateur du catalogue de gabarits

**6 septembre 2026, 14h43.** Doc intermédiaire d'un chantier ouvert par une
question de Lucie, posée pendant le renommage des identifiants :

> même est-ce que les templates non classés « builtin » devraient pas enrichir
> une extension « utilisateur » du catalogue ?

Ma lecture : ce qu'un projet ajoute à la bibliothèque fournie devrait être une
**couche du catalogue**, visible comme telle, et pas seulement un dossier qui
en masque un autre. Ce doc dit où on en est, ce qu'on décide, et les étapes.
Il se lit après le [doc 08](../vision_roadmap_09_2026/08-des-catalogues-de-gabarits.md),
qui pose la cible, et il en ferme deux questions ouvertes.

## 1. Où on en est

**Sur le disque, deux racines.** La bibliothèque fournie vit dans le crate
(`templates/`) ; ce qu'un projet adopte s'écrit sous `.rag3weaver/templates/`
à la racine de sa source. La lecture consulte le projet d'abord, la
bibliothèque ensuite, et le premier trouvé gagne (`template::racines`,
`scan_racines`, `lire_dans`). C'est prouvé par un test : un `user` de projet
masque le `user` fourni, et n'apparaît qu'une fois.

**En base, une fiche sans origine.** L'entité `Template` porte nom, famille,
catégorie, description, chemin, empreinte. Rien ne dit si une fiche vient de
la bibliothèque ou du projet. Une fois indexées, les deux couches sont
indistinguables.

**Et en production, personne n'indexe.** `register_template_schema` et
l'ingestion des fiches ne sont appelés que par les tests. L'outil `place` dit
à l'agent « cherchez d'abord avec `search(target='Template')` » : hors des
tests, cette cible n'existe pas. L'outil `adopt` écrit le fichier et termine
par « réindexez le catalogue de gabarits pour que la recherche le trouve » —
et rien ne le fait. **La couche utilisateur n'est donc alimentée nulle part.**
C'est le vrai trou, et il est plus large que la question d'origine.

**Ce que les tests couvrent** (`e2e_catalogue_gabarits`) : la recherche par
sens sur les fiches fournies, la pose avec motifs, le refus d'un gabarit
inconnu avec ses voisins, l'adoption puis la repose, le masquage, la
description obligatoire (test ignoré). Tout passe par le disque et par un
catalogue monté à la main ; aucun test ne suit le chemin d'un agent.

## 2. Ce qu'on décide

### Une seule entité, deux champs de plus

Pas de seconde table. Le doc 08 tient à ce qu'un gabarit se cherche avec les
mêmes moyens qu'un document, et deux tables demanderaient deux cibles ou une
base de connaissances pour une bibliothèque de quatre fichiers. La fiche gagne :

| champ | valeur | rôle |
|---|---|---|
| `origin` | `builtin` ou `project` | un filtre ; « ce que ce projet a ajouté » est une recherche ordinaire |
| `derived_from` | l'empreinte du gabarit fourni de même nom au moment de l'adoption, vide sinon | savoir qu'il a bougé depuis |

### Le masquage devient une identité

L'identité de la fiche est déjà `(family, name)` (`hashsafe`). Indexer le
projet **après** la bibliothèque fait que sa fiche prend la place de la fiche
fournie — même `_uuid`, contenu mis à jour, `origin = project`. Le masquage
n'est plus une règle de lecture du disque : il est dans la base, et il tient
sur tous les chemins de recherche sans étape de plus. C'est ce que le test du
masquage exige déjà (« masquer n'est pas ajouter »).

Ce qu'on perd : la fiche fournie masquée n'est plus cherchable en base. On la
retrouve sur le disque, et `derived_from` garde son empreinte. Accepté.

### Une synchronisation, nommée, appelée par ceux qui savent

`Catalog::sync_templates(project: Option<&Path>)` : enregistre l'entité si
besoin, lit les racines, pose les fiches (l'empreinte évite de réembarquer ce
qui n'a pas bougé), **efface** les fiches dont le fichier a disparu. Idempotent.
Trois appelants :

1. **l'ouverture d'un catalogue pour un agent**, là où la racine du projet est
   connue (la source de fichiers dont le curseur est `worktree:…`) ;
2. **`adopt`**, juste après avoir écrit le fichier — la ligne « réindexez »
   disparaît, l'outil rend une fiche trouvable ;
3. **`schema`** ou un outil de rafraîchissement, si un humain édite un gabarit
   à la main pendant une session. À voir à l'étape 4 ; ce n'est pas un
   troisième mécanisme, c'est le même appel.

### « Ce gabarit a bougé depuis que tu l'as pris »

Question ouverte du doc 08, fermée sans lien normatif : à la synchronisation,
une fiche de projet dont `derived_from` n'est plus l'empreinte du gabarit fourni
de même nom est **signalée** dans le rapport de synchronisation et dans la méta
de recherche, pas migrée. Poser est un acte daté.

### Ce qu'on ne fait pas ici

- **Pas de cellule par origine.** Une cellule en lecture pour la bibliothèque
  serait juste avec C3, mais la bibliothèque voyage avec le crate et tient en
  quatre fiches. Le champ suffit tant qu'aucun catalogue n'est partagé entre
  projets ; ce jour-là la cellule reprend le rôle sans changer la fiche.
- **Pas d'arête `POSÉ_DEPUIS` sur `place`.** Le doc 08 dit non par défaut ;
  `derived_from` sur l'adoption couvre le cas utile (une variante du `user`
  fourni) sans lier une entité vivante à son moule.
- **Pas la famille des composants.** Même mécanique, territoire neuf, un autre
  chantier.

## 3. Les étapes

| | quoi | comment on saura |
|---|---|---|
| E1 | la fiche : `origin`, `derived_from` ; `scan` les renseigne selon la racine ; `template_config` les déclare | test de bibliothèque : la fiche d'un fichier de projet dit `project`, celle du crate `builtin` |
| E2 | `Catalog::sync_templates` : registre, lecture des racines, pose, effacement des disparus, rapport (posés, mis à jour, effacés, en retard) | e2e : après sync, `search(target=Template)` trouve les fournies ; un fichier de projet masque par identité (`_uuid` inchangé, `origin` changé) ; un fichier supprimé disparaît ; un fichier modifié met la fiche à jour sans réembarquer les autres |
| E3 | `adopt` appelle la sync et rend la fiche ; `derived_from` posé quand un gabarit fourni de même nom existe | e2e par l'outil : adopter puis chercher trouve, sans autre appel ; adopter `user` garde l'empreinte du `user` fourni |
| E4 | le chemin de l'agent : la sync à l'ouverture du catalogue quand la racine du projet est connue ; `place` et `search` documentent la cible `Template` qui existe maintenant | e2e agent : un agent monté comme en production cherche un gabarit et le pose, sans montage à la main |
| E5 | le signal « a bougé » : à la sync, les fiches de projet dont `derived_from` ne correspond plus ; rendu dans le rapport et la méta | e2e : modifier le `user` fourni (racine de test), sync, le rapport nomme la fiche en retard |
| E6 | les noms anglais dans `template.rs` et `template_nodes.rs` (`lire` → `read`, `racines` → `roots`, `ecrire_entity` → `write_entity`, `sous_le_nom` → `alias`, `motifs` → `patterns`, `DOSSIER_PROJET` → `PROJECT_DIR`…) | la suite verte ; c'est la part de ce fichier dans le renommage décidé le 6 |

Chaque étape est un commit. E1 et E6 sont mécaniques ; E2 est le cœur ; E4
demande la cartographie de l'ouverture du catalogue côté agent, qui est en
cours.

## 4. Où ce chantier se place

Deux décisions prises aujourd'hui restent à exécuter et **ne sont pas
remises** : les identifiants en anglais dans tout le crate, et la migration
des cent cinq appels de tests sur le lanceur pour retirer le monolithe
`Catalog::search`. Ce chantier passe devant parce que Lucie l'a demandé, et
parce qu'il ne dépend d'aucun des deux : la recherche des fiches suit le
gabarit `search_base`, donc le chemin qui reste. E6 en est un morceau. Après
lui, dans l'ordre : le renommage, puis la migration.

## 5. Ce qu'on a trouvé en le faisant

- **Aucun montage d'agent en production.** La cartographie l'a dit avant la
  première ligne : la chaîne catalogue + source `worktree:` + palette
  n'existait que dans les tests, chacun avec son montage à la main, et aucun
  code de production n'appelait `builtin_graph_tools`. Le montage a
  maintenant un nom, `agent::mount_agent_services` (et `_on` avec une source),
  et les trois tests d'agent passent par lui.
- **La suite cloud était cassée derrière sa porte.** `e2e_cloud_code_agent`
  n'entre dans la passe qu'avec `--features openai-llm` ; jouée, elle tombait
  avant tout appel de modèle, sur la garde du 5 septembre (un `vector`
  indexé par BGE-M3 et interrogé par `HashEmbedder`). Personne ne l'avait
  jouée depuis. Corrigé : le même modèle des deux côtés, comme les autres
  suites.
- **Les suites e2e ne s'exécutent qu'avec `--ignored`.** Un test e2e sans
  `#[ignore]` n'est pas joué par `run_e2e.sh` et passe pour filtré. À savoir
  quand on en ajoute un.

## 6. État

| étape | état |
|---|---|
| E1 | fait — `d5e1618c8` |
| E2 | fait — `c32bff1ff` |
| E3 | fait — `adopt` réindexe et garde l'empreinte de ce qu'il masque |
| E4 | fait — `mount_agent_services`, les trois montages de test migrés ; suite cloud en cours de jeu |
| E5 | fait, dans E2 — `stale` dans le rapport de synchronisation, relayé par `adopt` et par le montage |
| E6 | fait pour `template.rs` et `template_nodes.rs` (dans E1) ; le reste du crate suit dans le renommage général |
