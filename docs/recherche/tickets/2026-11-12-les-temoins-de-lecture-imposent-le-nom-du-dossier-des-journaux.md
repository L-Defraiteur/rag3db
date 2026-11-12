# Les témoins de lecture imposent le nom du dossier des journaux

- **État** : ouvert ; fixture aa58d1697 validée le12novembre2026 : helpers3/0/0 et lectures27/0/0 ; revue des preuves et intégration restantes, porteur recherche.
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

Joué à cette étape : diff --check et parsing rustfmt vers copie externe, pas de compilation ni exécution native. Source à publier/figer avant test. Plan ciblé : après coordination ED/M8 et service7879, poste lourd lib29 complète/-j8/Mem32/target97k privé plafonné32Go ; compilation bornée1500s puis tests du helper60s et lectures27 (24 existants + 3 helpers, aucun filtre ajouté) borné120s sous racine privée arbitraire ne contenant pas le segment `commandes`, global1800s. Logs hors target, source/ELF/native/gabarits attestés. Aucun service/GPU/modèle nouveau ni e2e entier ; service existant uniquement après accord de créneau, premier autre rouge stop sans retry. Diff et preuves à revoir avant globale/master, aucun Rust poussé pendant les globales ED/M8.

## Préfixe exécuté, avant ciblé utilisant le service

Source figée propre et publiée `aa58d16979f20c00bdcff060f49524697c72a8df`, base `af2ef6846`. Préfixe explicitement autorisé après revue du diff : compilation1500s + helpers60s, global1800s incluant l’attente du service et le futur ciblé. Départ12novembre15:48:55 CET, échéance16:18:55. Wrapper2563127, timeout2563159, Python2563160. Tous SERVICE_* et EMBED_SERVICE retirés ; aucun appel au service durant ce préfixe. Scope réel `run-p2563159-i82252833.scope`, `memory.max=34359738368` vérifié pour le lanceur et ses enfants, cible privée target97k neuve et plafonnée32Go.

Compilation rc0 en4min52s ; helpers **3 réussis,0 échec,0 ignoré,857 filtrés**, sans rejouer. Le préfixe s’arrête ensuite ROUGE sur une erreur de son instrumentation, pas du produit : section readelf `.rag3db_native_build` erronée. Le nom réel `.rag3db_compiled_against` a été confirmé et l’attestation autonome seule autorisée par orchestration, sans rejeu compilation/tests et sans prolonger l’échéance. Sa première version cherchait par appartenance mmap au lieu de find : parcours mal écrit et lent, arrêté par pidfd sur le seul PID2655739 ; erreur et arrêt conservés. Version V2 corrigée et bornée120s exécutée en moins d’une seconde : **VERTE**, lib29 complète exacte et **220 gabarits présents**. ELF `target-97k/debug/deps/e2e-f9dffd07ba40d3a6`, SHA256 `729f0d114c65c867e44089219ab4b48e7e44776b327279d41ed99312ad94da1a`. Quiescence du target vérifiée, lourd rendu. Aucun test ni compilation rejoué pour corriger ces outils.

Preuves brutes deux postes : `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/` : source.json, argv.json, compilation.log, helpers.log, resume.log, verdict-prefixe.log (rouge conservé), cgroup-prefixe.json, arret-attestation-autonome.json, attestation-autonome-v2.log, attestation-binaire-corrigee.json, identite-native-corrigee.txt, verdict-attestation-corrigee.log, quiescence-prefixe.json. Les compteurs et verdicts d’outillage restent séparés.

M8 a confirmé ses8suites100/0/0 terminées et la quiescence de son usage7879. Le ciblé **27** (24 existants + 3 helpers) attend la coordination ED/C7, avec contrôle /sante réel, poste lourd effectif, BUILD_MEMORY_HIGH=0 pour éviter un scope frère hors MemoryMax extérieur, et vérification des cgroups enfants. Racine privée avec espace `racine arbitraire`, sans segment commandes. Aucun master avant revue, ED/M8 prioritaires pour l’intégration ; ticket toujours ouvert.

## Ciblé de lecture terminé — verdict soumis à revue

Après confirmation explicite C7 de la fenêtre ED/M8 (M8 terminé, ED libre/non lancé ; ordre recherche27 puis ED2), /sante7879 réellement relue : service rag3weaver-embeddings, modèle bge-m3, dim1024, sparse/dual vrais, factice faux. Le service existant n’a été ni démarré, ni arrêté, ni modifié par recherche.

Source distante toujours figéeaa58d16979f20c00bdcff060f49524697c72a8df et ELF729f0d114c65c867e44089219ab4b48e7e44776b327279d41ed99312ad94da1a inchangé avant/après. Lancement wrapper2687989 sous vrai poste lourd, marker seulement dans cette portée, MemoryMax32G et BUILD_MEMORY_HIGH=0. Les cgroups enfants observés restent dans la portée bornée ; argv/env/ascendance conservés. Compilation/tests du préfixe n’ont pas été rejoués pour corriger l’outillage.

Un appel `run_e2e.sh --test e2e_lectures`, borne120s : **27 réussis,0 échec,0 ignoré en9,46s**, soit24 existants +3 helpers, sans filtre/skip ajouté. Les2filets préalables sont verts. Racine réelle privée `~/.cache/rag3weaver-build/journaux-cx-recherche-lectures-12nov/racine arbitraire`, avec espace et sans segment commandes : les journaux `.out` sous index y sont recensés dans preuves-racine.json. Toutes les attentes de fin/échec d’indexation et assertions de lecture/recherche existantes sont conservées et jouées.

Fin vers15:59:20 CET, avant échéance globale16:18:55, sans extension de budget. Vérification /proc exe target97k ou cwd worktree : **liste vide** dans quiescence-lectures.json ; santé7879après relue OK. Fin et disponibilité du service annoncées directement ED/M8/C7 ; aucun autre usage recherche prévu. Aucun nouveau modèle, GPU ou service déployé, aucun e2e entier, aucune CI relancée.

Preuves ajoutées aux journaux sur les deux postes : sante-7879-avant/apres.json, lancement-lectures.log, argv-env-lectures.json, cgroup-enfants-lectures.json, lectures.log, e2e-complet.log, preuves-racine.json, verdict-lectures.log et quiescence-lectures.json. Le rouge d’attestation initial reste conservé et attribué à l’outil ; compilation, helpers, attestation corrigée et ciblé ont chacun leur résultat distinct.

Diff et preuves transmis à orchestration pour revue avant toute globale/master. ED/M8 gardent priorité à l’intégration ; aucun Rust poussé sur master pendant leurs globales. Ticket ouvert, worktrees et target97k privés conservés avec remotes et journaux, sans nettoyage d’un autre lot.

## Passe d’intégration après ED/M8 : arrêt de confinement navigateur

Dépendance levée explicitement par C7 : masterb726d907b4c808c501263b61da3424ed3d893bee. Rebase sous nouveau nom `cx-recherche-lectures-journaux-integration`, source propre publiée `487c68af39575582784cc26fc67dddad9bbc4c3a`. Conflit add/add limité au ticket déjà apporté par ED : texte commun identique et ajout recherche conservés. Range-diff archivé : Rust identique au correctif validéaa58, deux commits documentaires inchangés.

Nouvelle passe autorisée sous nouveau global2400s, départ après vrai lourd16:47:44 CET, échéance17:27:44. Même target97k privé unique/plafond32Go, lib29 complète, -j8/Max32, TMP court privé ~/.cache/cx-rj et React existant. Source/argv/env/cgroups enfants enregistrés ; tous services retirés avant les lectures. **Features5/5 vertes** ; contrôle `cargo check --lib --no-default-features --features rag3db-native` rc0 en11s, correctement identifié comme contrôle natif redondant. La précision C7 demandait le vrai minimal SANS --features ; le pilote étant déjà chargé, celui-ci restait à jouer séparément sous même échéance, sans interrompre les compilations. **Liblarge2201 réussis,0 échec,65 ignorés,0 filtré**,36,61s de tests.

Démarrage de l’étape navigateur séparée :40 tests détectés, puis **premier rouge du garde de confinement**, PID2997235 déplacé dans une scope Chromium extérieure au Max32 du poste. Arrêt de la session2994167 et des descendants possédés ; aucun résultat fonctionnel NAV40 complet revendiqué. Détail et suite dans [ticket commun des scopes](../../orchestration/tickets/2026-11-12-une-scope-e2e-imbriquee-peut-echapper-au-plafond-du-lanceur.md). Quiescence après arrêt active=[]/chromes=[] ; wrapper et PID fautif absents. **Lectures27 et vrai contrôle minimal non joués sur cette source**, aucun appel7879 ni relance après le rouge. Le vert fonctionnel antérieuraa58 reste historique, sans être présenté comme une passe intégration complète.

Preuves brutes aux deux postes dans `journaux-cx-recherche-lectures-12nov/integration-487c68/` : source.json, argv.json, features.log et features-detail/, minchecklib.log (natif redondant), lib.log, navigateur.log, resume.log, verdict-final.log ROUGE, cgroups-enfants.json, arret-session-2994167.json, quiescence-apres-arret.json et lancement.log. Aucun code runtime modifié, aucun master avant nouvelle revue/àtoi. Worktrees/target privés conservés, aucun retry aveugle ni budget prolongé.
