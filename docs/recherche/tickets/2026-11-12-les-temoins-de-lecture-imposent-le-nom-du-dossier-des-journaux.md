# Les témoins de lecture imposent le nom du dossier des journaux

- **État** : ouvert le 12 novembre 2026 ; porteur recherche, signalé par ED.
- **Pour** : recherche, fixtures e2e_lectures ; aucun correctif de fixture dans le lot des accords.
- **Famille** : produit
- **Gravité** : coût ; une isolation valide des journaux fait échouer la passe avant les lectures.
- **Atteignable en service** : non, dépendance de montage des témoins.
- **Touche rag3weaver** : oui, témoins et configuration RAG3WEAVER_JOURNAUX_DIR.

## Constat et preuve

Sur `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`, `RAG3WEAVER_JOURNAUX_DIR` est posé sur
un dossier privé nommé `.../accord-20261112/journaux-passe`. L'outil index rend bien le
chemin du journal réel sous `journaux-passe/index/`, mais les témoins de lecture cherchent
une sous-chaîne littérale `commandes/index/` dans leur extraction de la réponse.

Sources : `extension/rag3weaver/tests/e2e_lectures.rs:279`, `:322`, `:683`, `:1460`.
Le produit compose le chemin depuis la variable dans `src/dataflow/run_nodes.rs::dossier_journaux`.
Résultat réel : **5 réussis, 19 échecs, 0 ignoré en 6,37 s**. Les 19 paniques portent
exactement « le journal de l'index » ; elles surviennent sur l'extraction du chemin,
pas sur une assertion de résultats de recherche.

Preuve luciepc, hors target :
`~/.cache/rag3weaver-build/I-luciepc/accord-20261112/passe-rouge-journaux-b42/`
(`e2e_lectures.log`, `rapports-e2e/`, source et verdict rouge conservés).
Même source et mêmes assertions, seule racine privée changée en `.../accord-20261112/commandes` :
**24 réussis, 0 échec, 0 ignoré en 9,57 s**, une suite jouée sur une demandée.
Preuve : `lectures-commandes.log` et `lectures-commandes-complet.log` dans le dossier ED voisin.
Ce montage compatible débloque le lot courant ; il ne corrige pas la dette de fixture.

## Suite et critères de fin

Réutiliser une extraction du chemin effectivement rendu, sans imposer le nom de sa racine
ni fabriquer un résultat ou abaisser les assertions de lecture. Chercher d'abord une primitive
commune de reçu/journal pour éviter quatre parseurs différents. Prouver la suite complète avec
une racine arbitraire hors dossier réel, sans le segment commandes, et garder les assertions
sur l'index et les lectures. Le ciblé et la passe restent requis selon le périmètre du correctif.

Avant toute globale mémoire contenant lectures24, un montage privé `.../commandes` évite ce
rouge identifié. ED transmet ce ticket et cette précondition à mémoire et orchestration.

## Reprise recherche et montage préparé

Reprise explicitement autorisée par orchestration, worktree/branche neuf `cx-recherche-lectures-journaux`, base master `af2ef6846`. Ticket repris depuis le blob35c23c2a6 sans embarquer le lot ED. Périmètre limité à `tests/e2e_lectures.rs` : aucun runtime, manifeste, ED/M8 ou service modifié.

La relecture des contrats existants trouve le reçu dans IndexNode : chaîne Markdown avec ligne `journal : …`, ou objet JSON à champ `journal`. Le backend enveloppe dans `result`, l’hôte ajoute une enveloppe `result` ; aucun helper de reçu/journal commun aux témoins consultés ne fait cette extraction. Neuf parseurs locaux découpaient la sérialisation JSON par guillemets, antislashs et espaces puis exigeaient `commandes/index/`.

Un helper local unique lit maintenant la Value réelle, descend les enveloppes `result`, puis extrait le champ/ligne désigné. Il préserve les espaces et échappements ; il exige toujours un journal `.out` sous `index`. Les neuf usages le composent, sans changer leurs boucles bornées de fin/échec ni les assertions index/recherche. Trois tests de helper couvrent les deux formats, zéro/une/deux enveloppes et l’aller-retour JSON avec chemin arbitraire/espaces/accents/guillemets/antislash ; les refus d’une simple mention de chemin et d’un autre journal restent explicites. Ces tests non ignorés se jouent séparément ; la suite ignorée existante conserve ses24 témoins.

Joué à cette étape : diff --check et parsing rustfmt vers copie externe, pas de compilation ni exécution native. Source à publier/figer avant test. Plan ciblé : après coordination ED/M8 et service7879, poste lourd lib29 complète/-j8/Mem32/target97k privé plafonné32Go ; compilation bornée1500s puis tests du helper60s et lectures24 borné120s sous racine privée arbitraire ne contenant pas le segment `commandes`, global1800s. Logs hors target, source/ELF/native/gabarits attestés. Aucun service/GPU/modèle nouveau ni e2e entier ; service existant uniquement après accord de créneau, premier autre rouge stop sans retry. Diff et preuves à revoir avant globale/master, aucun Rust poussé pendant les globales ED/M8.
