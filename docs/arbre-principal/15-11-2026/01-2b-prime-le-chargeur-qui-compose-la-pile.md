# 2b′ : le chargeur qui compose la pile d'une vue, et ce qu'il ne promet pas avant 2a′-3

15 novembre 2026, arbre principal, sur le oui de Lucie (par c7). Ce lot se joue en parallèle de la mesure
d'ED, sur la seule couche qui existe en base, `backend`, pour que 2b-C (le paramètre `view` des outils)
suive sans attendre l'histoire de 2a′-3. Il est rouge d'abord, et hors push jusqu'au mot de c7.

## Ce qui existe sur master (repérage du 15 novembre)

- **La lecture est figée sur une couche** : `Catalog::poser_les_generations` pose
  `Visibilite { couche: "backend", rang: <max lu à l'ouverture> }` dans `Forme.generations`. Aucun
  paramètre ne la change.
- **Le seul point où une vue entre dans la recherche** est `Forme.generations`, lu par
  `masque_en_predicat`, `admitted_parent_uuids_for` et `resoudre_le_filtre`. Les plans et les sauts sont
  bornés par `borner_plan` et `borner_hop`.
- **`_generation`** a ses colonnes `parente_couche` et `parente_rang`, mais `merge_generation` ne les écrit
  pas.
- **Le résolveur 2a** (`publication/layers.rs`, `PinnedLayers`) compose une pile **en mémoire**, sur des
  ensembles d'occurrences complets. Le produit ne l'appelle pas.
- **Les suivies** donnent la parente git de chaque couche et son id stable, mais aucun lien vers
  `_generation`.

## Ce que 2b′ promet

1. **Une vue se nomme** `(couche, rang)`, où la couche est un id stable des suivies et le rang une
   génération. Un rang omis vaut la dernière génération publiée de la couche.
2. **Le chargeur lit la chaîne dans `_generation`** : la génération demandée, puis sa parente
   (`parente_couche`, `parente_rang`), et ainsi de suite jusqu'à la racine. Il en fait **la pile de
   lecture** : une liste de (couche, rang) et les tombstones de chaque étage.
   - **Les refus sont nommés**, avec les règles de `PinnedLayers` : couche inconnue, rang absent,
     parente manquante, cycle, et profondeur au-delà du réglage.
   - Il ne charge pas d'occurrences : la pile reste une **description**.
3. **La composition est un prédicat, pas un ensemble.** Une ligne est visible dans la pile si elle est
   visible dans l'un des étages, à son rang, et si aucun étage plus proche ne la retire, par sa clé ou
   par son fichier. Le prédicat se rend aux mêmes trois points qu'aujourd'hui (`masque_en_predicat`,
   `resoudre_le_filtre`, `borner_plan` et `borner_hop`). `Forme.generations` porte la pile au lieu d'une
   couche seule.
4. **Une pile d'un étage ne change rien.** Avec `backend` seul, chaque réponse est identique au bit près
   à celle d'aujourd'hui, dans les trois signaux, `usages`, `impact`, `outline`, `read_file` et
   `grep_files`. C'est le témoin 0, jugé par le banc.
5. **Une pile de deux étages se compose**, posée à la main dans un test (deux couches et leurs
   générations écrites en `_generation`) : la vue de l'enfant voit ses lignes, celles de sa parente à son
   rang épinglé, et pas ce que l'enfant retire. Le test est rouge d'abord, dans les trois signaux et dans
   `usages`.
6. **La publication écrit la parente** d'une génération quand les suivies la connaissent : la couche
   parente (son id), à son dernier rang au moment de la publication. Avec `backend` seul, elle reste
   vide.
7. **Une génération déjà écrite sans parente se relit comme racine** de sa chaîne. Ce n'est pas un refus,
   et l'état le dit (« pile d'un étage : aucune parente écrite »). Aujourd'hui, seule `backend:0` est
   dans ce cas, et elle est racine par construction. Une génération à venir d'une couche dont les suivies
   connaissent la parente se publie avec elle ; si elle s'écrivait sans (un chemin oublié), l'état le
   dirait aussi, et le témoin de la publication rougirait.

## Les réglages (dans le manifeste, sous `tracked`)

`"views": {"max_depth": 16, "load_timeout_ms": 2000}`. C'est la profondeur au-delà de laquelle une pile se
refuse, et le temps qu'un chargement attend la table `_generation` sous une lecture. Toute clé inconnue est
un refus qui la nomme, comme pour `writes`.

**Le retour arrière de la bascule des bases de Lucie** se prépare, sans sa réponse, sur « une semaine, puis
retrait de l'ancien index base par base, à son mot » (c7, 15 novembre).

## Ce que 2b′ ne promet pas tant que 2a′-3 n'est pas là

- **Pas de couches d'histoire réelles** : aucune branche n'a encore de génération écrite. Seule
  `backend` existe, et toute vue d'une autre couche se refuse (« couche sans génération, 2a′-3 »).
- **Pas de paramètre `view` dans les outils** : c'est 2b-C, avec recherche. Le refus d'aujourd'hui (« vue
  contextuelle non disponible », `daemon_host/invocation.rs`) reste jusque-là.
- **Pas de `view: b@c` à la demande** ni de rattrapage : c'est 2a′-3.
- **Pas de base de Lucie** : elles sont d'avant les versions, et se disent « à migrer ».

## La découpe de 2b-C, à proposer à recherche quand ce lot est vert

1. **Le paramètre `view`** dans `SearchOptions` et dans le graphe `search_workspace.mmd` : `{"layer": <id
   ou nom>, "rank"?: n}`, résolu par les suivies (nom → id) puis par le chargeur. Une vue inconnue est
   refusée avant toute recherche.
2. **La vue courante par le contexte** : la session attachée à un worktree lit sa couche de travail sur
   sa branche ; sinon, la couche du backend. Le refus de `daemon_host/invocation.rs` tombe.
3. **L'état et l'IDE** : la vue lue se dit dans chaque réponse (couche, rang, profondeur de la pile).
