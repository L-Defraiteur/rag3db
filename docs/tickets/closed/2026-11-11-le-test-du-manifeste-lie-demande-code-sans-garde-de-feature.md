# Le test du manifeste lié demande code sans garde de feature

- **État** : corrigé, source `bffbe5033`, matrice avec/sans code et passe finale vertes ; clôture dans ce lot.
- **Famille** : produit
- **Gravité** : vérification bloquée dans la configuration rag3db-native sans code ; aucun défaut en service établi.
- **Atteignable en service** : non, témoin unitaire ; le refus du manifeste sans feature est attendu.
- **Touche rag3weaver** : oui, src/builtin_templates.rs, test a_linked_manifest_loads_and_a_modified_copy_is_reported.
- **Pour** : everything-declarative, avec orchestration si le porteur doit changer.
- **Signalé par** : arbre-principal, [reprise du lot 1a](../../arbre-principal/11-11-2026/01-rapport-de-session.md).

## Constat et preuve

La passe `scripts/cargo_magasin.sh test -j8 --lib --features rag3db-native` sur le lot arbre-principal
(49caf64c6 avec le diff de projection et de transaction en cours) donne 1 565 réussis, 1 échoué,
15 ignorés. Le seul échec est builtin_templates::tests::a_linked_manifest_loads_and_a_modified_copy_is_reported :
`workspace` exige un binaire avec la feature code, alors que le test n'a qu'un attribut test.
Il prépare explicitement templates/backends/code/backend.json, puis unwrap PreparedBackend::load.
Le refus produit est correct ; le montage de cette configuration de test ne l'est pas.

Journal luciepc hors target : ~/.cache/rag3weaver-build/cx-projection-natif-regressions.log.
Source : src/builtin_templates.rs autour de 521–529 dans extension/rag3weaver.
Ce fichier n'est pas modifié par le lot 1a ; aucune régression de recherche n'est établie par cet échec.

## Travail attendu et fin

Confirmer le contrat de la matrice de features et vérifier le témoin sur sa base, puis le borner
à code ou employer un manifeste portable qui conserve son intention. Prouver la configuration
sans code et celle avec code, sans affaiblir le refus d'un workspace par un binaire qui ne le sait pas.
Ne pas marquer la passe native précédente verte. Le transfert ou le simple signalement ne ferme pas ce ticket.

## Reprise ED sur master, indépendante du lot 1a

Base `fc3ca92aa`, source documentation seule `1fffe50dc` : le ciblé natif sans
code échoue bien sur le refus de workspace (`0/1/0`, 0,00 s). Preuve luciepc :
`I-luciepc/ouverture-20261111/baseline-native.log` et `commit-baseline`.
Le premier lancement, sous-module codeparsers absent du worktree neuf, n'avait
rien compilé ; log de montage conservé séparément.

Le candidat rebasé sur master `2e910930d` garde le test historique sous feature
code. Un témoin portable charge un graphe livré de schéma, le lie puis vérifie
la dérive de sa copie modifiée ; un autre prouve le refus explicite de workspace
sans code. Aucun refus produit n'est affaibli. Deux premières fixtures portables
avaient un chemin qui ne désignait pas le gabarit et une clé workspace.source
manquante ; ce ne sont pas des régressions produit. Corrections tests `a166474a0`.
Ciblés avec et sans code, puis lib native entière en cours : aucun vert global.

Premier résultat `a166474a0` sans code : ouverture6/0/0 (0,58 s), module
builtin_templates9/0/0 (0,01 s). Lib native entière et ciblés avec code encore
en cours ; preuve `cibles-3.log`, aucun verdict global anticipé.

Ciblés terminés sur `a166474a0` : sans code, ouverture6/0/0 (0,58s),
gabarits9/0/0 (0,01s), lib native entière1575/0/15 ignorés (20,44s).
Avec code, ouverture6/0/0 (0,60s), gabarits9/0/0 (0,01s), aucun ignoré dans
les ciblés. `cibles-3.log`, `lib-native-3.log`, `verdict-cibles-3`.
Ce vert ciblé ne remplace pas la passe globale ni la lecture croisée avec 1a.

La passe globale08 est verte (features5, lib2121/0/64, nav39 et sept e2e),
preuves détaillées dans le rapport. Ticket encore ouvert avant tour/intégration ;
le point de revue du schéma nommé est porté par l'autre ticket existant et peut
retarder l'atterrissage du lot. Aucun nouveau défaut de matrice n'y est observé.

## Validation finale et passage de lot

Source Rust/UI/e2e `bffbe5033` : features5/5, gardes31/0/0, lib2132/0/64 ignorés
(43,16s), navigateur39/0/0 (132,98s). Sept suites e2e sans SAUTÉ : dérivées4,
lectures24, PG34, txvecteurs3, txarrêt18, txmerge1, txligne3. La suite txarrêt
laisse un CSV connu (12K), suivi par
`docs/recherche/tickets/2026-11-11-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md` :
ce vert ne signifie pas absence de résidu. Les ignorés de la lib sont ses marques
existantes ; les tests navigateur sont joués séparément.

Native sans code sur11ea : ouverture6 et gabarits9 verts, lib1575/0/15 ignorés
(20,60s). Avec code : ouverture16 et gabarits9, fournisseur1 verts. Les propriétés
compatibles s'ajoutent, les identités et configurations incompatibles ne
s'écrasent pas ; refus nommés avant le premier appel base. La baseline292 prouve
le contrat Catalog additif et le refus d'extrémités, deux verts. Les anciens
rouges de fixtures et les vrais rouges produit restent séparés dans les logs.

Après rebase1365b872b, diff depuis bff strictement deux Markdown : Rust/UI/Cargo/
gabarits et helper inchangés. Helper65aa SHA dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628,
19 tests verts aux deux postes ; nouvelle exécution sur SHA documentaire final
avant push. Tour « à toi » accordé par orchestration après lecture du verdict.
Preuves sauvegardées aux deux postes : `~/.cache/rag3weaver-build/I-luciepc/ouverture-20261111/`
(`verdict-final.json`, `*-passe-2.log`, `cibles-4.log`, logs baseline et garde).
Push sous poste push/garde VERT, trois contrôles puis archives administratives
fidèles hors cible et garde fraîche avant chaque retrait ; aucune suppression
si refus. Worktrees/target/PG du reste-équipier restent hors de ce lot.
