# Couple neuf après correction de la fixture, aucun run repris

Même [ticket](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md).
Source FIXTURE SEULE `71fbe786fcf0b2314e72b3eb4b8484faf590d78c`, moteur59ac
inchangé, basée sur5099. Le [refus de compilation](07-rows-red-compile-refused.md)
est conservé, ancien budget clos. Revue C7 de la source71 favorable bornée :
réserve/emplace des neuf mêmes types, quatre cas/assertions et production
inchangés. Cette préparation n'affirme pas une compilation réussie.

Branche/worktree pilote NEUFS `cx-hotfixs-rows-outside-graph-red-pilot-v2`.
Par rapport au code pilote db833, seuls changements fonctionnels : pin SOURCE
passé5099→71fbe, dossier de paquet `cx-hotfixs-rows-outside-graph-red-v2`, scratch
`cx-hotfixs-rows-red-v2`, unité `cx-hotfixs-rows-red-02.service`. Controller,
parseur de refus binder, caps, admission et garanties de quiescence identiques.
Pas de nouvelle primitive, de tolérance XML ou d'autre cas joué.

Même budget2400 proposé NEUF après vrai poste32G/swap0/quota effectif/j≤8,
watchdog2390+10, controller2310/native2330/final2380, build1800 et seul cas1
cap30 après export/identité. Autres trois NON JOUÉS. Compteur de collecte0
éventuel ne signifie pas livraison : résumé finit toujoursROUGE.

Préparer sur PC uniquement avec `prepare.py` au nouveau SHA complet et un
nouveau worktree source71 propre ; aucune réutilisation des dossiers du
premier passage. SHA des trois fichiers natifs calculés depuis source71,
source.bundle complet et quatre scripts pinés dans recette neuve. Après
publication/RO du paquet, GO2400 exact distinct obligatoire avant `launch.sh`.
Anciennes source5099, pilotdb833, recettea31745, Jv1 et preuves38/f067/b678
restent intacts. Aucun CMake/build/test/poste/unité lancé dans cette préparation.

Après publication du code pilote `b307f74cc21587cb982d6ef44cdb95763f992cba` :
six tests du parseur synthétiques verts0,002s, AST5 et bash-n/diff-check valides,
[reçu CPU v2](rows-red-v2-cpu-receipt.json). Cela n'exécute pas le contrôleur,
ni le C++, ni le callee SHOW_INDEXES, et ne qualifie pas le rouge produit.

Préparation PC fichiers/Git seulement terminée code0, sur deux worktrees
détachés propres NEUFS :
`/home/lucied/git_workspaces/cx-hotfixs-rows-outside-graph-source-v2` àsource71,
`/home/lucied/git_workspaces/cx-hotfixs-rows-outside-graph-pilot-v2` àpiloteb307.
[Recette exacte copiée du PC](rows-red-v2-prepared-recipe.json) SHA256
`37e29c250113c12bd4ff0385a1ca06590c41cfa519268c3590bf699223387867`.
PaquetJv2 `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v2`,
source.bundle657545040octets, SHA256
`42f19dc05811589c8f4180c0c1546e85862e76a1c9f31ab769107a1d747ed280`.
Bundleverify/listHEAD71 puis rehash indépendant, quatre scripts et trois
fichiers natifs conformes ; raw/résumé/allocation ABSENTS, scratchv2 VIDE.
Aucun CMake/build/test C++/poste/systemd ; nouveau couple reste HOLD avant
RO recette/paquet et GO2400 exact distinct. Ancien Jv1/recettes/preuves conservés.
