# Retrait du monolithe `Catalog::search` — l'écart, les décisions, le plan

**18 septembre 2026.** Chantier confié par l'orchestrateur (décidé le
6 septembre, bilan `docs/6-septembre-2026-11h43/05` question 1 : migration des
tests plutôt que pont). Cartographié en lecture seule sur `3ea70fe93` ; les
lignes citées sont celles de ce commit. Branche : `retrait-monolithe-recherche`,
worktree `../rag3db-recherche`.

## 1. L'état

- `Catalog::rechercher` (catalog.rs:6694) : le graphe `search_base.mmd` monté
  sur les services, `Arc<Mutex<Catalog>>` en entrée — prouvé équivalent sur
  BM25/vecteur/hybride (`e2e_generic_search::le_lanceur_rend_ce_que_le_monolithe_rend`).
- `Catalog::search` (catalog.rs:7226–7606, 381 lignes) : **229 appelants**
  réels — 218 dans 26 fichiers de tests, 5 tests unitaires dans `src/`, et
  **un seul appelant de production** : `search_with_explore` (catalog.rs:7704).
  `KBSearchNode` passe déjà par `rechercher`.
- Aucun chevauchement avec les régions en cours côté orchestrateur (KB
  1676–1902, dérivées 4950–5347, `build_ingestion_graph` 5651–5957) : toute la
  surface recherche (7018–7979) leur est disjointe.

## 2. Ce que `rechercher` ne couvre pas encore

Tout le reste est couvert (consigne stricte, budget, filtres, embarquement
unique, natif/lucivy, sparse, rerank avec pool avant pagination, fan-out de
cellules, SourceResolved, warnings, événement). Les écarts :

| # | Écart | Où | Poids |
|---|---|---|---|
| a | **`options.result_mode` n'atteint pas les nœuds de signaux** : ils lisent leur `self.result_mode` de config (défaut `Aggregated`) ; `Detailed` par `rechercher` rend des résultats agrégés sans un mot. 13 sites de test en dépendent (`e2e_result_mode`, `e2e_highlight_long_text`, `e2e_simple_entity`). | generic_search_nodes.rs:259/501/806 | **bloquant** |
| b | `diagnostics.bm25_hits` et `engine_warnings` jamais peuplés (`BM25SearchNode` passe `None` en diag, les deux branches). | generic_search_nodes.rs:684/726 | bloquant pour les tests de diagnostics |
| c | `diagnostics.enrich_ms` jamais posé (pas de nœud d'enrichissement séparé). | catalog.rs:6853–6866 | mineur |
| d | **Typage d'erreur** : cible inconnue = `UnknownKB` par le monolithe, `DbError("recherche « … » : …")` par le lanceur. `catalog_search_unknown_kb` (search.rs:3778) matche la variante. | catalog.rs:6817 | contrat |
| e | **`meta.signals`** : le monolithe rapporte les signaux *demandés* ; le lanceur OU-ise ceux des métas effectivement émises (un signal muet ne met pas son bit). `assert_eq!(…, HYBRID)` (search.rs:3789) casse. | fondre_les_metas | contrat |
| f | Comptes de signaux : monolithe = hits *chunks* avant résolution ; nœuds = `unified.len()` *après*. Divergent quand plusieurs chunks se replient sur un parent. | generic_search_nodes.rs:446/738/971 | à documenter |
| g | `target.default_fusion` honoré seulement si `has_source_refs` (KB) ; une entité simple qui déclarerait sa fusion prend les poids du gabarit. | generic_search_nodes.rs:1104–1112 | latent |

## 3. Décisions proposées

- **(a)** Le `SearchSourceNode` reçoit déjà tout `SearchOptions` ; les nœuds de
  signaux prennent `self.result_mode.unwrap_or(options.result_mode)` — même
  motif que `bm25_mode` (B10). `Option<ResultMode>` dans la config de nœud,
  `None` dans le gabarit.
- **(b)** Brancher le paramètre `diag` des deux branches de `BM25SearchNode`
  vers sa méta (les diagnostics remontent par le port `meta`, `fondre_les_metas`
  les fusionne). **(c)** poser `enrich_ms` = durée du nœud `resolve`.
- **(d)** `rechercher` résout la cible *avant* de monter le graphe
  (`resolve_search_target` → erreur typée), le reste garde l'habillage `DbError`.
- **(e)** Après le pli des métas : `meta.signals` = signaux demandés
  (`options.signals ?? target.default_signals`), comme le monolithe.
- **(f)** Garder les comptes des nœuds (après résolution : c'est ce que
  l'appelant reçoit) et le **documenter** — le test d'équivalence n'exige que
  la parité de nullité. À contredire si quelqu'un dépend des comptes chunks.
- **(g)** Retirer la garde `has_source_refs` : `options.fusion` puis
  `target.default_fusion` s'il existe, sinon les poids du gabarit.

## 4. Plan

1. **Phase 1 — parité** : (a)+(b)+(c), puis (d)+(e)+(g) ; le test d'équivalence
   s'enrichit de `Detailed`, des diagnostics et des deux asserts de contrat.
2. **Phase 2 — migration des 218 appels de tests**, fichier par fichier, suites
   une par une. Motif : helpers `&mut Catalog` → `&Arc<Mutex<Catalog>>` +
   `Catalog::rechercher` ; navette `Arc::try_unwrap(...).into_inner()` là où le
   test reprend la main en `&mut` (motif déjà en place dans
   `e2e_catalogue_gabarits:558`). **Risque n° 1, nommé : le verrou n'est pas
   réentrant** — les sites qui tiennent un guard pendant la recherche doivent le
   lâcher d'abord (`e2e_code` : 16 sites ; `e2e_agent_loop` : 2). Ces deux
   fichiers passent en dernier, un test à la fois.
3. **Phase 3 — retrait** : `search`, `search_fan_out` (7064), la récursion de
   bascule de cellule (7244), les 5 tests unitaires de `src/` migrés, les
   commentaires de doc qui le citent (12 emplacements relevés), et
   `search_with_explore` bascule sur `rechercher`.
4. **Phase 4** : passe complète depuis le worktree
   (`RAG3DB_BUILD=…/build/lecteurs-csv`, `RAG3DB_ROOT=` dépôt principal — les
   tests y chargent l'extension vecteur — `CARGO_TARGET_DIR` partagé).

Hors périmètre : des cellules par requête sans bascule d'état global (limite
nommée dans la doc de `rechercher`), et `search_with_strategy` (autre graphe).

## 5. Phase 1 — livrée (même soir)

(a) `result_mode` hérite de la requête sur les trois nœuds de signaux (motif
B10, `Option<ResultMode>` en config, absent = hériter) ; (b) `BM25SearchNode`
remplit `bm25_hits`/`engine_warnings` sur ses deux chemins et les remonte par
sa méta, `rechercher` superpose les durées ; (d) la cible est résolue avant de
monter le graphe — erreur typée ; (e) `meta.signals` = signaux demandés ;
(g) la garde `has_source_refs` de `base_de_fusion` est retirée (go de
l'orchestrateur : c'était un raccourci).

Documentés sans alignement : (c) `enrich_ms` reste à zéro — l'enrichissement
vit dans le nœud `resolve`, sa durée est `resolve_ms`, la dupliquer mentirait ;
(f) les comptes de signaux sont mesurés après résolution parent (le monolithe
comptait les chunks avant) — personne ne dépend des comptes chunks ; si un
test les asserte, l'assertion s'adapte en le disant dans le commit.

Preuve : `e2e_generic_search` 17/17, dont le nouveau
`le_lanceur_honore_detailed_diagnostics_et_contrats` (Detailed de bout en
bout, `bm25_hits` peuplés, `UnknownKB`/`UnknownEntity` préservées,
`meta.signals` demandés même quand un signal se tait) — et les seize
d'avant inchangés, équivalence comprise. Unitaires 1045/1045.

## 6. Le repli des KB atterrit pendant la migration (notes de l'orchestrateur)

Master `3586166c1` (+ `acda811ad`, burn `630c546c`) : une KB est une entité
dérivée — tables `{kb}`, `{kb}_Chunk`, `{kb}_DERIVED_FROM`, `{kb}_CHUNKED_FROM`,
champs `title`/`content`, `entity == "TreeKB"` dans les résultats. Pour cette
migration :

- le plein texte d'une dérivée porte sur **`content` et `title`** ;
- **`_source_field` n'existe plus** — le champ d'origine d'un chunk est
  `_parent_field` (dialecte déjà changé) ;
- `e2e_search`, `e2e_highlight_long_text`, `e2e_phase0b`, `e2e_result_mode`
  sont réécrits pour ce monde sur master : il n'y reste que search →
  rechercher à faire, après rebase de la branche.

## 7. Phases 2 et 3 — livrées, et ce que la migration a débusqué

Les 229 appels passent par `Catalog::rechercher` ; `search` (381 lignes) et
`search_fan_out` sont supprimés, `search_with_explore` prend l'`Arc`, la
fabrique dit « Runs Catalog::rechercher via service registry », les
commentaires sont balayés. Les tests migrent avec la recette d'`e2e_rerank` ;
`e2e_entites_derivees` et `e2e_generic_search` prennent un helper `cherche`
(envelopper → lancer → `Arc::try_unwrap` → rendre le catalogue) parce que
leurs receveurs restent nus. `le_lanceur_rend_ce_que_le_monolithe_rend`
devient `le_lanceur_tient_la_page_et_les_comptes` : la moitié référence est
morte avec le monolithe, restent les invariants qu'elle prouvait.

La migration a débusqué **trois bugs de production** du chemin composable :

1. **La fuite de cellule vectorielle** — le filtre de cellule n'était compilé
   qu'en présence d'une condition utilisateur ; `compile_filter_for_vector`
   est maintenant systématique.
2. **L'instantané de modèles périmé** — quand `Immediate` draine pendant le
   graphe, les services montés avant ne connaissent pas la colonne née du
   drain ; repli sur le service `catalog` verrouillé brièvement (porté du
   nœud BM25 au stockage vecteur), et requalification
   `EmbeddingModelUnavailable` à la frontière de `rechercher`.
3. **La fusion aplatie** — le retrait de la garde `has_source_refs` (écart g)
   faisait passer `default_fusion` — aplatie par `unwrap_or_default()` dans
   `TargetInfo` — pour une fusion *déclarée*, ce qui éteint les `weights` du
   gabarit. L'outil des agents fusionnait 0,3/0,7 au lieu du 0,6/0,4 de
   `search_base.mmd`, et la correspondance exacte (`merge_port_values`,
   rang 1 bm25 sur master) coulait sous le vecteur, hors du top 5. C'est le
   seuil « markdown ×3 < JSON » d'`e2e_code` qui l'a attrapé — la
   comparaison champ à champ contre master n'a montré aucune clé perdue,
   mais des résultats différents. Correctif : `TargetInfo.default_fusion`
   redevient `Option` (l'`ec.fusion` de la config, sans aplatir) — déclarée
   elle prime, rien de déclaré et le gabarit décide. L'assertion du test est
   durcie (`### 1. merge_port_values` : en tête, pas seulement citée dans le
   titre du rendu).

**Changement de comportement assumé** : le monolithe fusionnait les entités
simples aux défauts moteur (bm25 0,3 / vector 0,7) ; par `rechercher`, c'est
le gabarit `search_base` qui fait foi (0,6/0,4, penché plein texte à dessein).
Porté à Lucie par l'orchestrateur ; l'embarquement est prévenu — son banc
mesurait l'hybride avec bm25 à 0,3.

Hérité, pas à nous : `e2e_symbol_search` est à 2/12 **sur master aussi**
(vérifié le 18 au soir depuis l'arbre master).

Revalidation après le correctif de fusion, en série : lib 979, `e2e_code`
24/24 (seuil ×3 restauré, assertion de rang durcie), `e2e_generic_search` 17,
`e2e_entites_derivees` 3, `e2e_scope` 9, `e2e_rerank` 3, `e2e_result_mode` 10,
`e2e_search` 39, `e2e_catalogue_gabarits` 12, `e2e_agent_loop` 8 — tout vert.
`e2e_postgres` est migrée à l'aveugle et s'exécutera quand un serveur sera là.

Le banc vecteur seul (18 au soir) : l'écart monolithe→lanceur (MRR 0,346→0,328,
R@1 10→9) n'est pas le chemin — les deux exécutent les mêmes fonctions dans le
même ordre, et un run sondé rend exactement 0,328/9 sur un index reconstruit.
La question flippée se joue à +0,002 de cosinus (`lot_budget` rang 2 derrière
`budget_conseille`, 0,73693 contre 0,73892) : quasi-ex-æquo que le recall d'un
HNSW reconstruit réordonne, comme la variance M3 de l'embarquement (0,206→0,241
entre deux passes identiques).
