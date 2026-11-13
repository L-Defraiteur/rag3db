# Parcours agent de code réel : preuve et reproduction

Le parcours est observé sur une fixture Git neuve, dans un hôte privé : recherche,
lecture, édition, test nommé, diff et reprise. Les trois premiers tours passent
par le vrai chat du framework et le modèle Vertex configuré ; un quatrième tour
reprend la même tâche depuis le bouton de chat de l’IDE dans Chrome. Aucun modèle
simulé, accord humain fabriqué ou daemon personnel ne participe.

Le correctif produit porte sur la lecture Git des worktrees dans le bac à sable.
L’administration du worktree doit renvoyer vers son gitfile réel, être enfant
direct du dossier commun du principal connu de l’hôte ; un gitfile final symbolique
est refusé. Seuls son administration et les chemins communs exacts nécessaires
sont lisibles. Les objets/références exposent l’histoire du dépôt, pas seulement
le diff. Aucun accès aux autres administrations de worktrees ni nouvelle écriture
n’est accordé. Les alias internes et externes sont refusés. Git charge par défaut
`/dev/null` comme configuration globale et désactive ses verrous facultatifs,
sauf configuration explicitement déclarée.

## Preuves acquises

| Étape | Source | Résultat observé |
|---|---|---|
| Premier parcours | `36e284824` | Test1failed attendu, édition/test1passed ; diff128 refusé, reprise non atteinte |
| Première correction Git | `b0090e2be` | Compilation verte, noyau rouge : configuration globale privée inaccessible |
| Réserves de revue | `b310b5c0c`, `b68569d3a` | Noyau vert partiel ; arrêts volontaires avant Vertex pour ajouter les contre-exemples |
| Contrat Git complet | `4d6045cdb` | Noyau1/0/0 ; parcours3tours1/0/0 vert46.05s, reçus exacts, reprise persistée |
| Première UI | `ec57ae39b` | Démarrage Chrome rouge avant nouveau tour ; stderr était masqué |
| Diagnostic CDP | `fe0a5be4e` | Abort134, stderr `Socket path too long` dans TMPDIR trop long |
| Reprise UI, TMPDIR court | même `fe0a5be4e` | CDP1/0/0 en0.23s, UI1/0/0 en19.71s ; test/diff et rendu de relecture réels |

Preuves privées aux deux postes sous
`~/.cache/rag3weaver-build/I-luciepc/parcours-agent-ide-20261112/` :
`correctif-git-4` et `page-ide-2`, JSON01/02/04/05, HTML du chat et du diff,
recettes, sources, ELF/SHA, supervision et quiescence. Les deux portées finales
sont vertes, code0, restes vides et nettoyage vrai. Les rouges et arrêts sont
conservés à côté, jamais convertis en succès par un retry.

Les SHA/logs des premiers ELF sont conservés, mais leurs exécutables ont été
remplacés par Cargo avant archivage indépendant ; l’ELF b685 et les suivants sont
copiés hors target. Les relevés tardifs sont identifiés comme tels.

## Rejouer volontairement sur une fixture neuve

Ce scénario consomme des appels Vertex et se joue explicitement, jamais dans la
lib ordinaire. Publier source/recette/budget, utiliser la primitive commune,
lib native épinglée, j8/Max32/HIGH0 et un target propre au lot. Poser le chemin
autorisé des identifiants et le projet configuré ; ne pas lire/copier les clés.
Un TMPDIR court créé sous le cache privé évite la limite Unix des sockets Chrome.
Paquets React épinglés et navigateur réel sont requis. Aucun service local de
modèle n’est nécessaire ; les absences dense/sparse sont dites. La tâche peut
réindexer son plein texte à la relecture : cela ne prouve pas du dense disponible.

1. Compiler la lib de tests avec
   `rag3db-native,code,daemon,openai-llm,navigateur`, extraire l’unique ELF lib du
   flux Cargo JSON, le copier hors target et conserver SHA/lib chargée.
2. Poser `RAG3WEAVER_PREUVES_PARCOURS` sur le dossier de preuves du préfixe,
   puis jouer exactement l’ignoré
   `daemon_host::chat_qui_code::le_chat_code_teste_et_reprend_sur_vertex`.
   Attendre1passed/0failed/0ignored et `verdict-parcours=VERT` ; sinon arrêter.
3. Poser `RAG3WEAVER_FIXTURE_PARCOURS` sur l’unique dossier `parcours ide *`
   créé sous ce préfixe ; conserver ce chemin et ne pas recréer la fixture.
4. Poser `RAG3WEAVER_PREUVES_PARCOURS` sur un AUTRE dossier UI et jouer exactement
   `daemon_host::chat_qui_code::le_parcours_vertex_se_reprend_dans_la_page`.
   Le témoin exige le verdict et l’historique précédents, envoie le message par
   le bouton réel et vérifie les nouveaux reçus ainsi que la page de relecture.

Les ELF se lancent avec `CARGO_MANIFEST_DIR` du worktree et `LD_LIBRARY_PATH` de la
lib épinglée ; les racines état/journaux/targets-fixture restent privées. Chaque
étape a une borne (360s pour chacun des deux témoins réels dans la recette acquise),
et la portée globale inclut l’arrêt/nettoyage. Les tests/Cargo de la fixture
restent immuables, le principal est attesté octet par octet et HEAD compris ;
les résultats sont liés aux identifiants d’outils et commandes exactes.

## Intégration

Ticket : [parcours IDE](../../tickets/closed/2026-11-12-prouver-le-parcours-agent-de-code-complet-de-l-ide.md).
Branche2 sur master65b, source8e366e4aa publiée et code du lot identique àfe0.
L’union après symboles3ed/C++ est validée sous sourcee4eaf4ef2, branche3 :
F5 5/5, minimal0, lib2253/0/66, NAV40, lectures27, hôte13 et PostgreSQL36, tous
sans échec ; filets2 à chaque e2e. Supervision verte, restes[] et quiescence[].
Le contrôle1446.97s reste dans la deadline6000 initiale, sans reset. Le runtime
Git et les témoins réels sont byte-identiques aux sources4d/fe0 acquises.
Le lanceur e2e canonique désactive l’incrémental Cargo : son ELF réellement
exécuté est archivé séparément des trois préparés. Target29.7Go<32Gio.
Le tour master C7 puis le nettoyage gardé2postes restent la fin de livraison.


### Union empilée finale

À la demande de C7, source `c9e6b6e39` = Pub31 + Symboles + ED : F5/min verts,
lib2267/0/66, NAV40, lectures27, hôte13 et PG36 verts, filets2 par e2e. Scope0,
restes[] et quiescence[]. Contrôle2834.58/6000 dans la même échéance initiale.
Pub conserve ses neuf fichiers produit ; ses CLI5/tx25 sont les preuves Arbre,
qualifiées séparément sur31. Git/UI réels restent équivalents aux sources4d/fe0.
Preuves distinctes `globale-union-pub`, avec quatre ELF et dumpPG rafraîchi.
La livraison suit l’ordre Pub→ED ; bilans ultérieurs seuls peuvent changer.
