# Exports par défaut : état de reprise et preuves

## Code courant, distinct de la première mesure

- Branche rag3db : `cx-codeparsers-require-2`, code **d96ed3264**, worktree
  `/home/lucied/git_workspaces/cx-wt-codeparsers-require` aux deux postes.
- Sous-module : **a730a9ba89ef233423bb7063f85181de26d98ce0**, branche
  `cx-codeparsers-default-metadata`, worktree du parseur au portable
  `/home/lucied/git_workspaces/cx-wt-codeparsers-metadata`.
- Tout commit est sauvegardé sur sa branche distante. Aucun master n'a avancé
  avec ce lot et aucun tour de push n'est accordé.
- **8 e2e ciblés passent**, avec les formes CJS/ES, l'instance, l'objet de
  réexportation exact, l'externe/généré absent, les deux ordres d'ingestion,
  les extensions explicites et le refus de plusieurs écritures d'export.
- **286 tests unitaires et d'intégration codeparsers passent**, zéro ignoré
  dans `cargo test --tests` (45 résultats de suites). Une invocation antérieure
  de `cargo test` contient aussi un ancien exemple documentaire `ignore` :
  il n'est pas revendiqué comme joué.
- La passe complète rag3weaver n'est pas jouée. Une nouvelle A/B est requise
  pour ces hashes : les mesures de la première implémentation ne les valident pas.

## Le modèle et ses invariants

`ScopeFileAnalysis.default_export` porte un fait optionnel de fichier :
`DefaultExportTarget::Name(String)` ou `Instance { line: usize }` (ligne 1-based).
L'extraction lit la déclaration une seule fois avant la perte du préfixe
`export default` dans le texte propre des scopes. Les autres langages gardent
None. Serde accepte une ancienne analyse sans ce champ ; un témoin vérifie
l'aller-retour du fait et l'absence du champ dans l'ancien format.
Plusieurs écritures reconnues ne désignent pas une cible statique unique :
le fait reste absent, sans choix de la première écriture.

Le résolveur entre fichiers et rag3weaver consomment le même fait. Une instance
vise la zone de fichier qui contient sa ligne d'export, jamais la classe utilisée
pour la construire. Un nom doit désigner un scope non ambigu du fichier.
`resolve_relative_js_module` partage la résolution existante d'un import vers
son **fichier exact**, extension comprise ; le symbole de rendez-vous est
`@default:{source}#{fichier}`. Effacer l'extension fusionnait deux modules voisins.

Les tuple3 `CodeAnalysis.default_exports` et `pending_defaults` relient cette
information aux primitives existantes DEFINES/MENTIONS et aux scopes existants.
Aucun import_binding, gabarit, IR, stable_scope_keys, own_texts ou ScopeRecord
n'a changé. Le fait d'export appartient au contexte de résolution d'une occurrence :
partager contenu/vecteurs ne partage pas automatiquement analyse et visibilité.
Ces blocs ont été coordonnés avec arbre principal.

## Rouge puis vert réellement observés

Journaux à luciepc sous `~/.cache/codeparsers-notes/` :

- `require-temoins-rouge.log` : 4 joués, 3 échecs (alias et réexportation sans
  cible, instance reliée à sa classe), 1 vert externe.
- `require-temoins-correctif.log` puis `require-temoins-incremental.log` : 4 puis
  5 verts sur l'ancienne implémentation e1015a379/e48b414.
- `cx-require-temoins-complets.log` : 7 verts sur la reprise master ; l'objet
  `module.exports = { Alias }` exact est enfin couvert. Le premier témoin de
  « réexportation » renvoyait un objet depuis une fonction.
- `cx-require-extensions-rouge.log` : 6 verts/1 rouge avec un voisin .ts écrit
  `export default OutilTs`; `cx-require-extensions-vert.log` : 7 verts après
  conservation du fichier exact.
- `cx-require-formes-export.log` : 6 verts/1 rouge pour `export default class`;
  `cx-require-fait-export-vert.log` : 7 verts après le fait de fichier typé.
- `cx-require-ambiguite-rouge.log` : 7 verts/1 rouge (première écriture choisie);
  `cx-require-ambiguite-vert.log` : **8 verts**, zéro ignoré, sur d96ed3264/a730a9b.
- `cx-codeparsers-fait-export-tests.log` : **286 verts**, zéro ignoré, sur a730a9b.

Lib de ces vérifications :
`rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.

## Première A/B, uniquement e1015a379/e48b414

Source propre a72acfeef, même lib 29a7bc52a aux deux index. Baseline
4f54e473a/c660893, lot e1015a379/e48b414. Résultats à luciepc :
`~/.cache/codeparsers-notes/require-default-exports-e1015a379/`.
L'oracle est identique octet pour octet à celui de la branche historique.

A : index terminé en 1223 s, 137848 morceaux. B : terminé en 1300 s,
137783 morceaux. Ces durées sont sous la charge du poste, sans conclusion de vitesse.
Les hôtes ont été arrêtés par SIGTERM, leur absence et l'absence de connexions
7882 lues ; l'optimiseur a été averti de la libération du service privé.

Sondes : A 128430 CONSUMES, B 128971 ; mentionneurs TS/JS 1381 contre 1922.
Échantillon commun de 298 mentions : A 324 arêtes (279 justes, 10 fausses,
17 muettes, 12 liaisons d'import, 6 décl), B 307 (282 justes, 2 fausses,
17 muettes, 6 décl). Arêtes ajoutées : 464, dont 36 justes, 4 réexportations,
1 fausse et 423 muettes. Retirées : 82, dont 17 justes, 14 fausses et
51 liaisons d'import. Le script a terminé avec le code 0.
Aucun de ces nombres ne valide le code courant d96ed3264/a730a9b.

Les 17 arêtes justes retirées dans l'ancienne mesure du 9 novembre visaient
les liaisons locales : 13 index.js, deux reader, une rag3db_wasm et une CSS.
Une navigation plus lointaine ne rétablit pas automatiquement leurs verdicts LSP.

## Contrat sonde → oracle : mention et cible distinctes

L'oracle cherche le nom **de cible** sur la ligne source avant de demander
une définition. Pour les 415 liaisons historiques, vérification sur TSV+sources :
415 tokens absents, zéro source absente, zéro token présent. Ces questions
n'ont pas été envoyées. Ce n'est pas un silence du serveur.

Le lecteur `outils/lire-interrogations-oracle.py` distingue, sans changer les
entrées ni les verdicts : non interrogé/token absent, LSP interrogé sans réponse,
réponses avec positions visibles dans/hors corpus, verdict original juste/faux
ou autre. Les TSV ne conservent que deux positions : la qualification de corpus
porte explicitement sur ces positions visibles, pas sur la réponse exhaustive.
Aucun faux n'est neutralisé au motif qu'une réponse visible est hors corpus.

Tickets ouverts : le contrat de mention de l'oracle et le faux platform vers
un homonyme du projet. Une baisse des faux accompagnée de non-interrogés ne
prouve pas une précision gagnée. L'oracle n'a pas été modifié dans ces corrections.

## Prochaine mesure et intégration

Le retard initial était 195, puis 216 après fetch : reprise dans un worktree
neuf sur master, sous préfixe cx-, sans réécriture ni suppression des anciennes
branches. La branche -2 a été créée avant rebase et sauvegardée.

Le script `outils/cx-mesure-default-exports-v2.sh` prépare deux bases fraîches :
A=aece4b9f1/c660893 (base exacte de la branche, docs seules après 41526b29a),
B=d96ed3264/a730a9b, même lib 29a7bc52a. Source/modèle restent figés, gabarits
copiés seulement, hôte privé 7978 et service privé granite-278m dense768 sur
7882. Il conserve l'historique et les relevés de charge avant/après chaque index.
Un accord opérationnel neuf de l'optimiseur est requis : rien n'est réservé
par le script ni par l'accord de principe de l'orchestration.

Après la première A/B, proposer un complément borné au vrai token et à la
position source, sur les mêmes cas avant/après, séparé de l'oracle historique :
les 17 positions de liaisons locales, FS et les nouvelles arêtes non interrogées
avec un alias différent de l'export. Les cibles fichier/plage restent celles
de chaque base ; la question LSP porte sur la mention, pas sur leur nom.
Aucune refonte générale de l'oracle n'est incluse implicitement.

## Fin de la première mesure et complément borné autorisé

Le lecteur final a été joué sur les quatre TSV :
`require-default-exports-e1015a379/interrogations-final.jsonl` à luciepc.
Sur les ajouts, 422 questions non envoyées (415 liaisons et 7 imports),
une question LSP sans réponse, 41 questions avec réponse. Les 82 retraits ont
été réellement interrogés. Le faux platform reste faux, même si sa réponse
visible est hors corpus. Aucune précision globale n'est déduite des rapports
279/324 et 282/307 : populations et états d'évaluation diffèrent.

Le complément autorisé fige **23 positions distinctes** : les 17 justes retirées
et les 7 imports nouveaux non interrogés, avec un chevauchement FS index.js:77.
Ses questions, tokens et colonnes UTF-16 sont dans
`outils/cas-lsp-23-d96ed3264.json`, SHA-256
`0d02f7959f9407c22042914cda6cd6a9708acbf35ab37b2f6a2761118a28ebc4`.
Les fichiers de la copie LSP et des sources mesurées ont été comparés octet pour
octet ; les 23 positions ont été validées sans démarrer de LSP.

`outils/interroger-mentions-figees.py` réutilise Serveur et le prédicat strict
`dans_la_cible` de l'oracle historique dont il vérifie le hash. Il refuse un banc
inachevé, vérifie hashes et positions, garde les RPC et toutes les positions,
et produit questions.jsonl, A.jsonl, B.jsonl et provenance.json séparés.
Les mêmes réponses aux 23 questions source jugent les cibles de chaque état.
Il conserve toutes les arêtes d'une ligne : plusieurs candidates restent
explicitement une attribution à examiner, sans sélection ni score global inventé.
L'oracle historique n'est pas modifié. Le complément n'est pas encore joué.

La seconde A/B est **démarrée** après transmission opérationnelle neuve de 7882
par l'optimiseur. Référence aece4b9f1/c660893 et lot d96ed3264/a730a9b figés ;
preuves dans `~/.cache/codeparsers-notes/require-default-v2-d96ed3264/` et
`cx-require-default-v2-lanceur.log`. Les contrôles de source a72 propre et
d'identité de l'oracle ont passé. R4 recherche est prioritaire et partage card2 :
aucune exclusivité ni conclusion de vitesse. Le complément attend la fin du
banc et la coordination des ressources ; aucun LSP supplémentaire parallèle.
