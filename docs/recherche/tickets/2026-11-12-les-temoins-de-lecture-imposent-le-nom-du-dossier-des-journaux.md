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
