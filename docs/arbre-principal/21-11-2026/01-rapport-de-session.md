# Arbre principal — note de pause du 21 novembre 2026, vers 15 h 15

*Écrite à la demande de c7 avant le redémarrage de la tablette (Lucie sort /tmp de la RAM). La session reprend par
`claude --resume`.*

## Où j'en suis

Les cinq manques des vues de tty (nuit), puis l'agent neutre, les diffs des vues, RootGrants et la vue qui suit son
worktree. **Atterris aujourd'hui** (vérifiés dans `landed.tsv`) : `arbre-atelier-pose`, `arbre-retrait-et-taches`,
`arbre-open-command`, `arbre-no-index`, `arbre-index-on-demand`, `arbre-index-offer`, `arbre-views-diff`,
`arbre-root-grants`, et cette nuit `lire-la-vue`, `ouvrir-un-dossier`, `un-seul-nom`, `poste`, `nom-de-vue`,
`ecrire-la-vue`, `fermer-tout`, `run-dans-la-vue`, les tickets du target de vue et des PR d'un agent.

## Ce qui tourne

**Rien, ni à la tablette ni à luciepc.** J'ai arrêté mes deux passes à luciepc vers 15 h 06, avant qu'elles partent :
leurs e2e et leur attestation seraient tombés dans la boucle du confineur (« tient cpu »), que corrige
`confiner-poste-classes` de G, pas encore atterri. Mes targets de la tablette ont été effacés (c7, disque plein) ; toutes
mes passes se font à luciepc, worktrees sous `~/git_workspaces/arbre/` là-bas, journaux sous `~/.cache/rag3weaver-build/`.

## Branches et têtes

| Lot (id au pousseur) | Branche | Tête | État |
|---|---|---|---|
| `arbre-non-git-folder` | `arbre-principal/non-git-folder-3` | abf2e725f5 | **en file**, rang 2 derrière le correctif de G (G l'a remis lui-même) ; recherche dépose `ide-opened-text` derrière |
| `arbre-sandbox-roots` | `arbre-principal/sandbox-roots-2` | 18130db024 | **en file**, déposé 14 h 33, attestation 8/8, parent - |
| `arbre-protected-folders` | `arbre-principal/protected-folders-2` | 76f294b198 | rouges vus (c6a6a8bc02, abb013b5f8) ; **jeu complet à relancer**, puis dépôt, parent `arbre-sandbox-roots` |
| `arbre-view-follows` | `arbre-principal/view-follows-3` | 74a745ea71 | témoin de recherche (2e97e5dc3b, 3596677ddc) + délai (d801886c4b) + repos rouge (1866ab9589) + inotify ; **jeu complet à relancer** (le rouge du repos d'abord), puis dépôt, parent - |
| — | `arbre-principal/view-search` | 404856c1ce | mon témoin en double, abandonné pour celui de recherche ; ne pas déposer |

## Ce qui attend

- **Relancer à luciepc**, seulement après l'annonce par c7 de l'atterrissage de `confiner-poste-classes` :
  - `protected-folders-2` : `poste cpu -- timeout 7200 bash pile-luciepc.sh ~/git_workspaces/arbre/21-11-2026-protected-folders $B/en-protected-folders-2 c6a6a8bc02 arbre-principal/protected-folders-2 the_protected_folders_cover 1` (rouges déjà vus dans `$B/pf2.out`) ;
  - `view-follows-3` : `poste cpu -- timeout 7200 bash vf3.sh` (le rouge du repos, puis `pile-luciepc.sh` sur la tête).
  - `pile-luciepc.sh` pose `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,7879,7880` (sans lui, e2e_lectures rougit sur le creux).
- **Témoins navigateur à luciepc** : prendre l'environnement du pousseur (`RAG3WEAVER_PAQUETS_REACT`), sinon 18 rouges de banc.
- **Ticket ouvert à faire** : la route des PR d'un agent (`docs/tickets/2026-11-21-les-pr-d-un-agent-ne-se-lisent-pas.md`),
  et le target propre à une vue (`…-sous-une-vue-cargo-ecrit-le-target-partage-du-depot.md`).

## Prochaine action

À la reprise : relire la file et `landed.tsv` ; attendre l'annonce de c7 ; relancer `view-follows-3` puis
`protected-folders-2` à luciepc ; déposer chacun à son vert ; retirer les worktrees atterris (tablette et luciepc).
