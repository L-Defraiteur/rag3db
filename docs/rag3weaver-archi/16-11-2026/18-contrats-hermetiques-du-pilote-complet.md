# Contrats hermétiques du pilote complet, avant essais

APP274 reste figée. Le [rapport17](17-arbitrage-cargo-du-pilote-complet.md)
précède cette matérialisation ; la branche pilote contient uniquement des
documents/scripts et aucun changement de produit.

`full-pass/policy.py` fournit les contrats de fenêtre7200 issue du reçu commun,
jobs min8/POSTE_FILS, sept champs sélectionnés sans environnement brut,
présence/valeur/profil déclarés, rejet MOTEUR_ANCIEN, admission exacte
cwd/target/argv, calcul du repo_target conforme Rust ASCII, configuration Bac
privée sans mutation du gabarit, précondition task-targets absent, inventaire
de l'ancien état et publication exclusive du résultat avec contrôle aprèsopen.
Le champ `debut` du reçu commun est le starttime /proc en ticks : le début
monotone est `echeance-recette.delai`, jamais une conversion de ces ticks.

Douze fonctions témoins sont écrites et NONJOUÉES à cette publication, avant
tout essai. Elles utilisent seulement TempDirs et données en mémoire :
échéance expirée/autorité terminée sans reset, allocationCPU, profil natif7,
featuresCPU avec environnementSDK hérité distinct, champs absents/vides/faux
et doublons, secret nonpublié, commande/cwd/target nonadmis, nom ASCII,
extra_read privé sans dégrader approval, task-targets préexistant/lien,
ancien état modifié/exclusion extérieure et vrai open final retardé.

Pas d'entrée lourde, shim, broker, Cargo, /proc, signal, réseau, modèle,
service ou reçu réel dans ce module ni dans ces témoins. Le pilote complet
reste à matérialiser ; ces contrats seuls ne qualifient aucune exécution et
ne donnent aucun GO7200. Les résultats éventuels seront ajoutés après
publication préalable, sans réécrire un échec.

Après publication et push `e5b069be07`, premier essai sous `timeout60`, Python
`-B`, failfast : **12/12 en0,002s**, aucun échec ni saut. Journal horsWT
`~/.cache/rag3weaver-build/cx-pv7-policy-e5b069.log`, SHA256
`9f56a0273ab5adfff0f4b891fdbdfd37039e06423462fb8a5fd7f881121f32af`.
Code inchangé depuis publication ; ces simulations ne qualifient ni Landlock,
ni Cargo réel, ni le scénario D6, ni la passe entière.
