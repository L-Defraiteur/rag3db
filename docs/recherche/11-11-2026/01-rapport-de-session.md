# Session « recherche » : rapport de reprise (11 novembre 2026)

## Travail courant (12 novembre) : montage du témoin close FTS

Ticket unique ouvert `docs/recherche/tickets/2026-11-12-le-temoin-de-fermeture-fts-echoue-avant-l-appel-a-close.md`, recherche, créé par orchestration avant transmission. Worktree/branche `cx-recherche-fts-close-deterministe` depuis898c9e0dc ; patch témoin `5a83eee91`. Diagnostic du rouge CI080 confirmé : ENOTEMPTY à la suppression récursive avant close ; aucune nouvelle causalité1a ni régression du close produit établie.

Montage proposé : adaptateur local de ShardStorage public sur RamShardStorage ; préparation normale, puis armement atomique d'un refus de persistance de racine. Close doit atteindre l'erreur distinctive, compteur positif, puis refus « closed », Drop et sortie enfant bornée20s sont conservés. Aucun retrait de dossier concurrent ni patch Lucivy, aucune assertion supprimée. Diff et parsing rustfmt seuls vérifiés ; **compilation et tests natifs non joués**.

Plan fermé détaillé dans le même ticket, à relire avant tout lourd : jeu CI natif/code/daemon/openai-llm, un témoin exact puis module FTS puis lib avec les deux filtres déjà présents en CI, global2400s, j8/mem32/target97k unique32Go/lib29a7, arrêt au premier rouge ; aucun modèle/service/GPU/DB ni CI rerun. Après ce préfixe, nouvelle revue de la passe finale requise, pas de matrice/e2e automatiquement ajoutés.

**Coordination : aucun créneau CPU réservé ni Cargo lancé par recherche à cette étape.** Les demandes arbre1b/coeur C++ reçues ne rencontrent donc aucun build recherche actif ; orchestration arbitre leurs créneaux. Le namespace TUI reste absent, mais le transport multi_agent_v1.send_input a accepté l’envoi du plan à C7 (soumission01a1cf2f-10c0-7033-9b71-6b7bbb6d0d94) et les réponses de coordination CPU. Plan+témoin publiés code5a83eee91/doc5dddf215c et effectivement transmis pour revue ; aucune autorisation de build ni tour master déduits de cet envoi.

## Lot journal d'embarquement livré le 12 novembre

**Master `375ca1b96c9cb4e71df03a30fc3209cf06883652`**, au tour explicite d'orchestration, sans rebase nécessaire (masterc693 inchangé), source testée43b57eb78, seuls documents ajoutés ensuite. Ticket clôturé par gitmv+État dans le même commit : `docs/tickets/closed/2026-11-11-le-service-d-embarquement-laisse-son-journal-dans-le-tmpdir-d-e2e-search.md`. Aucun autre lot activé.

Passe unique verte : sept faux-service et19contrôles, features5/5, preuve compilation, lib2132/0/64 ignorés, nav39/0/0, deux appels distincts search40 puis queue5 sans saut ; deux filets dans chaque appel. 220gabarits et chemins attestés, trois ELF SHA et identités natives29a7 relus. Preuve réelle par BGE existant (PID3415586 inchangé) distincte des faux-service. Aucune sélection card0, aucun service réel créé/remplacé/arrêté.

**Trois contrôles lus aux deux postes :** status propre, aucun rebase en cours, origin/master égal à HEAD375ca1b96. **Nettoyage terminé aux deux postes :** seules administrations du lot fidèlement archivées hors cible, inventaires des sous-modules/worktrees et garde exact65aa01328 SHA dca13abb… vérifiés VERT et quiescence maintenue avant retrait. Worktrees du correctif, branches locales et target97k supprimés ; remotes conservés. Les deux fichiers ignorés distants (e2e-last.log et charge-last.tsv) ont été déplacés dans les preuves avant le garde.

Seule base attribuée `cx_recherche_journal_20261112` supprimée après dump privé vérifié et contrôle de propriété/commentaire et zéro connexion active. Aucune base tierce touchée. Preuves et archives aux deux postes dans `~/.cache/rag3weaver-build/journaux-cx-recherche-journal-12nov/` : pousse.log, pg-nettoyage.txt, dump et manifests SHA, nettoyage-remote et nettoyage-local (copies locales des administrations). Le target local était absent. Le nettoyage du petit worktree documentaire de cette actualisation suivra le même garde.

CI : le bandeau avant push signalait un ancien rouge du11nov22:06UTC, pas un verdict du lot. Lecture du run375ca1b96 : rag3weaver43174418798 en cours, CI Workflow43174418717 en attente ; aucun vert CI revendiqué. Voir les runs du commit, sans attribuer ni masquer l'ancien rouge.

L'envoi intersessions reste absent de notre inventaire depuis récupération ; l'autorisation entrante a été reçue et appliquée. SHA final de livraison pour orchestration :375ca1b96c9cb4e71df03a30fc3209cf06883652. Les tickets de journal d'hôte, Surveillant, garde de target et autres suites restent ouverts séparément ; ce lot ne les résout pas.

## Lot Lucivy 4.3.3 : livré sur master

**Atterrissage confirmé :** `f96acc6cf97692804520e2d672de080bcae730a3`, après rebase documentaire sans conflit au tour d'orchestration. SHA des sources testées inchangés ; trois contrôles lus aux deux postes : status propre, aucun rebase, origin/master égal à HEAD. Worktrees et branches locales du lot supprimés aussitôt, target distant `target-97k` supprimé, local absent. Preuve `journaux-cx-recherche-lucivy-433-11nov/vert-433/pousse.log`. Journaux, attestations, générés et sauvegardes distantes conservés hors target. Ticket fermé dans `docs/tickets/closed/2026-11-11-la-fermeture-fts-en-erreur-laisse-le-handle-actif.md` ; aucun autre chantier activé.

Ticket porteur : `docs/tickets/closed/2026-11-11-la-fermeture-fts-en-erreur-laisse-le-handle-actif.md` (recherche). La validation complète est verte ; tour explicite reçu, ticket clôturé dans le commit de fin validé.

Branche `cx-recherche-lucivy-4-3-3`, worktree homonyme aux deux postes ; source de la passe `c1463b9ee`. L'arrêt précédent est livré (voir section suivante). Trois minimums passent à 4.3.3 (`lucivy-core`, `ld-lucivy`, `sparse-vector`) ; Cargo.lock ne change que les versions et sommes de ces trois paquets, `luciole` et `lucistore`, tous du registre, sans patch local. Aucun autre paquet mis à jour.

Le nouveau témoin public `fts_handle::repro_drop::une_fermeture_fts_en_erreur_rend_le_handle_inerte_et_le_processus_sort` est commité avant le bump (`820f29840`). Il crée son propre index, indexe/commite, supprime ce stockage, constate l'erreur de close puis exige le rejet d'un nouveau document avec « closed ». Il détruit le handle et le parent exige la sortie normale de l'enfant en 20 s. En 4.3.2 : rouge 0/1/0, 0,05 s ; close échoue sur la sauvegarde shard_0 mais `index_apres=Ok(())`. Journal luciepc `journaux-cx-recherche-lucivy-433-11nov/rouge-432.log`, SHA256 `73d2dd9e011a89b6084f39a7e3a5d124bfa73d2108af353833cefa04e0d8edc1`.

La preuve interne des pools est celle d'amont, déjà exécutée par lucivy sur le tag 663975a ; notre témoin public la complète sans prétendre mesurer ses Arc internes. Aucun test de compatibilité, migration ou banc d'index ajouté : format inchangé. Les contrôles de recherche/réouverture existants restent ceux de la lib FTS et d'e2e_search (notamment sparse mmap persistence).

Passe neuve lancée sous poste lourd, target unique `target-97k` neuf et gardé, lib complète 29a7bc52a sans dérogation, journaux `journaux-cx-recherche-lucivy-433-11nov/vert-433/`. Plan : même témoin puis neuf suites e2e (search, dense après suppressions, cited_by, lectures, code, quatre tx), features 5/5, preuve compilation, lib entière, navigateur séparé, attestation des gabarits. Pas de nouveau juge Gemini inchangé, périmètre confirmé par l'orchestration. Verdict complet VERT vérifié : témoin close 1/0/0, neuf e2e 121/0/0, features 5/5, lib 2105/0/61 (57,93 s), navigateur 36/0/0 (132,34 s), 212 gabarits attestés dans les binaires. Sources inchangées, lock limité aux cinq paquets du registre. Preuve `vert-433/verdict-final.log`, `attestation-artefacts.json`, vérificateur `cx-recherche-lucivy433-verifier.py`. Ticket clôturé avec preuves, push et trois contrôles accomplis ; voir atterrissage confirmé ci-dessus.

## Actualisation de la reprise, 11 novembre en fin de journée

**Atterrissage confirmé :** le lot d'arrêt est poussé sur master `80b467dd303769f81228149a7dfff5381c14db8b`, au tour donné par l'orchestration, après rebase documentaire sans conflit. Sources testées inchangées ; trois contrôles lus aux deux postes : status propre, aucun rebase en cours, origin/master égal à HEAD. Les trois worktrees du lot (reprise, audit, ancien arrêt), leurs branches locales et le target distant `target-97k` sont supprimés ; target local absent. Sauvegardes distantes et journaux hors target conservés. Le ticket de provenance du garde reste ouvert. Le lot suivant Lucivy 4.3.3 est distinct et n'est pas livré par cet atterrissage.

Cette section remplace les chemins et les états de reprise anciens ci-dessous, conservés comme historique.

**Preuve R5 du lot atterri : VERT agrégé.** Source testée `abb9a9559ca9a1ed874c92dbc73dba720952ff93`, base `a5f17368e`. Quatre ciblés d'arrêt, huit e2e (84, dont cinq juge Gemini réellement exécutés), features 5/5, preuve compilation, lib 2104/0/61 ignorés et navigateur reconstruit 36/0/0 (132,35 s). Preuve à luciepc : `~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov-r5/verdict-final.log`, produite par `cx-recherche-arret-verifier-r5-agrege.py`. Le `resume.log` initial et son navigateur 34/2/0 restent rouges et intacts : seuls app.js et bundle.json étaient anciens parmi 212 entrées ; les 212 contenus courants étaient présents dans la lib et les e2e, dont les résultats sont conservés.

**Récupération navigateur R5 achevée :** une seule modification de date de build.rs, contenu inchangé, target inactif, a régénéré les 212 chemins et contenus depuis le bon worktree. ELF SHA256 `3e3770df4000178626a8c07631ab124ab6673c67b643b7ad2cae6f6e7cfc7063`, chemins `.d` conformes, attestation indépendante d'archi. Deux ciblés inchangés passent ensuite 1/0/0 chacun (1,25 s et 67,85 s), puis les 36 navigateur complets. SHA de l'ELF inchangé après ce rejeu. Ancien ELF, empreintes et généré conservés hors target. Aucune capture HTTP supplémentaire ajoutée : les preuves portent sur les octets embarqués et les comportements du navigateur. Ticket de provenance toujours ouvert, correction durable du garde hors lot d'arrêt. Cette preuve a ensuite permis le tour et l’atterrissage consignés ci-dessus.

**Préparation de R5 :** ED `91e5f760b`, projection `6c4b5ad0c` et archi `8d5139f9e` sont intégrés. Rebase sans conflit sur `a5f17368e` (documents seuls après `8d5139f9e`), branche neuve `cx-recherche-arret-du-backend-5`, worktree `cx-rag3db-arret`. Diff réel relu : restitution des réclamations conservée avant la fin du fil attendu par `stop_and_join`, `Option<Result<(),String>>` conservé dans la source finale. Nouveau journal prévu à luciepc : `journaux-cx-recherche-arret-11nov-r5/`. Plan complet neuf : deux témoins normaux, deux contrôles erreur/succès, les huit e2e avec Gemini exigé, features, lib entière, navigateur. Target unique `target-97k`, lib complète 29a7bc52a sans dérogation, délais inchangés. Fenêtre de stabilité accordée pour cette validation ; aucun ancien verdict n'est promu.

**Historique du départage R4 :** R4 a fini à 4 tests juge réussis/1 échoué/0 ignoré en 999,52 s (code 101). Aucun processus n'a été tué : le garde d'arrêt a trouvé la fin naturelle et la vérification des restes était vide. L'unique ciblé inchangé autorisé ensuite passe 1/0/0 en 50,72 s, même source `0a2d2dbd9` et configuration ; la preuve cite cette fois `required`. Cela observe une variabilité, sans en établir la cause ; ticket ouvert et rouge conservé. Fenêtre code libérée au profit d'ED ; quatre tx/features/lib/navigateur non joués pour R4. Pas de nouveau R4 complet sur ancien master, pas d'autre retry aveugle.

**Historique du rouge R4 :** Sur `0a2d2dbd9`, quatre contrôles ciblés d'arrêt, lectures 24, code 27 et cited_by 3 passent ; le juge des outils échoue après 438 599 ms car sa preuve ne cite pas `required`, bien qu'il nomme le défaut et rende `fichier` optionnel dans la forme proposée. Assertion conservée ; ticket `docs/recherche/tickets/2026-11-11-le-juge-gemini-omet-required-dans-la-preuve-du-schema.md`, ED et orchestration informés. Les autres étapes après le juge ne sont pas encore jouées. Ce résultat ne remet pas à lui seul en cause le correctif d'arrêt, mais empêche de revendiquer un vert global.

**Relecture et correction d'arrêt :** La R3 ci-dessous est verte sur les chemins d'avant le correctif supplémentaire, mais ne valide pas la nouvelle tête. L'audit `8c426ad3d` a reproduit un faux succès après arrêt figé : premier Err, second Ok, vue ouverte (0/1/0, 1,01 s). Le correctif `7f2a0f65e` conserve `Option<Result<(),String>>` ; deux ciblés sont verts et un contre-témoin prouve l'idempotence après succès. `545fbfd17` isole la réaction figée dans un enfant pour préserver le runtime de la lib entière. Le ticket correspondant est dans `docs/tickets/closed/2026-11-11-un-second-shutdown-annonce-ferme-apres-un-arret-echoue.md`. Reprise réelle : branche `cx-recherche-arret-du-backend-4`, worktree `cx-rag3db-arret` aux deux postes ; les worktrees `cx-rag3db-arret-audit` gardent la preuve ciblée. Nouveau rejeu requis dans `journaux-cx-recherche-arret-11nov-r4`, avec les deux témoins normaux, les deux contrôles erreur/succès, les mêmes huit e2e, features/lib/navigateur. La fenêtre de stabilité est maintenue pour cette correction bornée ; aucun autre code n'est introduit.

- Contact établi avec la session d'orchestration `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`.
- Le résidu de COPY est documenté dans `docs/recherche/tickets/2026-11-11-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md` : les trois CSV de 12 Ko sont relus dans `journaux-arret/e2e-entier-1.log:8457–8459` à luciepc. La suite est un résidu intermittent connu, pas une cause corrigée.
- Première reprise : branche distante `arret-du-backend-9`, `77f6704e5`, base `d1345a115`. Conflit de classement du ticket backend résolu vers `docs/tickets/closed/` ; les deux tickets ouverts sont dans `docs/recherche/tickets/`.
- Sous la lib complète `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`, sans `RAG3WEAVER_MOTEUR_ANCIEN`, les journaux `~/.cache/rag3weaver-build/journaux-recherche-arret-reprise-sans-derogation/` à luciepc attestent : lectures 24/24, code 27/27, cited_by 3/3, arrêt par paquet 18/18, vecteurs après rollback 3/3, merge puis COPY 1/1, ligne à ligne 3/3 ; features 5/5 ; lib entière 2096 réussis, 0 échec, 58 ignorés ; navigateur 33 réussis, 0 ignoré. Le juge annonçait 5 réussis mais deux étaient **SAUTÉS** faute de configuration : résultat incomplet, rejeu Gemini exigé distinct dans `e2e_juge-gemini.log`, verdict `gemini-verdict.log` : **VERT**, cinq tests réellement joués, aucun ignoré, en 336,05 s.
- Le premier lancement avec le drapeau d'ancienneté, inutile, a été interrompu et conservé dans `journaux-recherche-arret-reprise-interrompue/`. Le lancement sans dérogation a aussi refusé un nom de suite inexistant (`e2e_tx_par_paquet`, zéro test), ensuite remplacé par les trois suites transactionnelles existantes. Le résumé historique conserve ces erreurs ; ne pas prendre son seul dernier mot pour une preuve complète.
- Master a depuis reçu le garde-fou Rust `1904e04e4`. Nouvelle reprise suivant la convention de Lucie : branche `cx-recherche-arret-du-backend-3`, worktree `/home/lucied/git_workspaces/cx-rag3db-arret`, même chemin prévu à luciepc ; rebase effectué sans conflit, série sauvegardée sur sa branche. Le rejeu de `762d93a20` sur la base `ec7583216` est lancé dans `journaux-cx-recherche-arret-11nov` après la fin du juge ; verdict **en cours**, target unique `~/.cache/rag3weaver-build/target-97k`, sans cargo concurrent.
- Correction du compte ciblé : les deux réussites de `temoins.log` sont les filets e2e, et le filtre demandé joue zéro test. Les vrais témoins unitaires sont verts dans la lib historique (lignes 1625 et 1639), mais exigent une preuve ciblée correcte pour le verdict final. Défaut du lanceur documenté dans `docs/recherche/tickets/2026-11-11-un-filtre-e2e-vide-peut-donner-un-code-de-sortie-vert.md`.
- Troisième rebase de la reprise : lot sauvegardé sur `cx-recherche-arret-du-backend-3`, base `41526b29a` (alias archivage mémoire). La passe sur `762d93a20` a validé les huit e2e (84 tests), les features 5/5 et la lib 2097/0/58 ignorés ; les deux vrais témoins sont nommés verts dans cette lib (lignes 1618 et 1630), mais le ciblage initial reste invalide. Finir son étape navigateur, conserver les journaux, puis lire `journaux-cx-recherche-arret-11nov-r3/` pour le nouveau rejeu avec ciblage lib corrigé. Fenêtre de push demandée à l’orchestration après ce troisième rejeu ; aucun tour accordé.
- **Verdict final du troisième rejeu : VERT**, code testé `f161f1fb00927dd660fc8a11952d54da195779fa`, base `41526b29a79e7137a315ac81bb9c5a5b0a6b83a9`. Preuve à luciepc : `~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov-r3/verdict-final.log`, vérifié par noms exacts, comptes par suite et lib unique. Témoins unitaires 2/2, huit e2e 84/84 (24 lectures, 27 code, 3 cited_by, 5 juge réellement joués avec Gemini exigé, 18 arrêt par paquet, 3 vecteurs après rollback, 1 merge puis COPY, 3 ligne à ligne), zéro ignoré dans ces étapes. Features 5/5, preuve compilation lue, lib entière 2097 réussis/0 échec/58 ignorés (49,72 s), navigateur 33 réussis/0 échec/0 ignoré (132,44 s). Aucun drapeau d'ancienneté, aucun saut nommé dans les étapes exigées ; le juge conserve son résidu intermittent connu avec ticket. Cette preuve remplace les états « en cours » de la troisième reprise, pas les journaux historiques conservés.
- Le futur lot `cx-recherche-lucivy-4-3-3` est distinct : minimums et lock ciblés, pas de mise à jour globale ni scénario de compatibilité/migration. Le témoin amont `a_failed_close_still_stops_its_pools` a été joué par la session lucivy sur le tag `663975adf11fc4a9d6cadf90e70ee68f6c9db075` : 1 réussi/0 échec/0 ignoré, 0,16 s, processus code 0 ; journal `journaux-lucivy/close-433.log`, SHA256 `3a2208790e67d2216a55d1c2c9d2a30bde79cb56ff0569054838562a29e69274`. L'intégration registre et sa preuve enfant close en erreur restent à faire après stabilisation de l'arrêt.

- Historique avant atterrissage : la demande de tour attendait un rejeu vert ; tour et push sont désormais accomplis (voir ouverture du rapport).

### Relecture des vues, transmise à arbre principal et ED

Accord sur l'ancre unique et la pyramide ; contrat à consolider avant implémentation du modèle. Le filtre par `(fichier, version)` du §3 ne suffit pas au composite par scope du §2 bis : résoudre récursivement la pile et ses générations épinglées en **occurrences visibles** ; un tombstone arrête la recherche dans les ancêtres. La visibilité est un invariant que le filtre utilisateur ne remplace jamais. Un backend incapable d'appliquer le masque doit refuser ; la précédence actuelle domaine/filtre utilisateur est une décision séparée.

Dense, creux et plein texte doivent restreindre leurs candidats avant le top-k. Un contenu/version partagé donne des occurrences distinctes avec leurs chemins, positions et relations ; l'expansion visible précède le classement final, la déduplication et la pagination. Filtrer les deux extrémités des arêtes. Garder une même génération sous L6 pour résolution, enrichissement et provenance. Une branche sans worktree exige une lecture des blobs/composites, le disque seul ne couvre pas ce contrat.

La réutilisation des vecteurs exige les mêmes champs réellement embarqués, modèle et configuration de découpage. Une normalisation des octets impose un remappage des positions de surlignage vers le texte rendu. Les témoins proposés portent sur la pile f/g/h, le tombstone hérité, l'ancre qui avance sans rebase, les occurrences partageant une version, le top-k saturé d'invisibles et le filtre utilisateur visant une occurrence masquée.

M3–M5 **non jouées** : l'optimiseur coordonne une future fenêtre isolée avec arbre principal. Aucun masque en cache ni stratégie HNSW n'est choisi sur des chiffres inexistants.


Ce rapport s'adresse à un agent qui reprend le périmètre de la session « recherche » sans aucun
souvenir de ses échanges. Il dit l'état exact au 11 novembre 2026, 16 h 30 : ce qui est sur
`master`, ce qui attend, ce qui reste sur disque, et les trois prochains pas.

Le savoir de fond (comment marche la recherche, comment on joue les tests, les pièges appris) est
dans le second fichier, `docs/recherche/11-11-2026/02-knowledge-dump.md`.

## Vocabulaire minimal

- **rag3db** : ce dépôt, une base de graphe (fork de Kuzu) en C++, avec une extension Rust,
  **rag3weaver** (`extension/rag3weaver/`), qui fait l'indexation, la recherche et les outils
  des agents.
- **Le portable** : la machine principale de Lucie, où vivent ce dépôt et ses sessions.
  **luciepc** : l'autre poste (accès `ssh lucied@luciepc`), où se jouent les passes de tests
  lourdes. Il porte les cartes graphiques et les services d'embarquement.
- **c7** : la session d'orchestration, qui tient l'ordre des pushes. Au 11 novembre, elle parle
  sous le nom `rag3db-5a` (avant : `rag3db-74`). L'ordre des
  pushes reste donné par l'orchestration, quelle qu'elle soit.
- **Un lot** : un ensemble de commits poussé d'un coup sur `master`, toujours précédé d'un
  **témoin rouge** (un test qui échoue sur le code d'avant, pour la bonne raison) puis du
  correctif qui le rend vert.
- **Une passe** : la série de tests jouée avant un push (voir le second fichier, « La passe »).
- **La lib** : la bibliothèque C++ du moteur (`librag3db.so`) que le Rust lie. Une passe
  l'**épingle** par son identifiant complet, par exemple
  `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
- **Le témoin des résidus** : la règle de `run_e2e.sh` qui exige qu'une suite e2e laisse son
  dossier temporaire vide (voir le second fichier).

## Lots sur master (le plus récent d'abord)

| hash | date | ce que ça change |
|---|---|---|
| `92180836b` | 9 nov. | **Les résidus de suite** (lot `residus-de-suite`). Le cache d'index du catalogue quitte `/tmp` (`catalog::index_cache_dir`, variable `RAG3WEAVER_CACHE_DIR`, sinon `~/.cache/rag3weaver/cache`), et les caches des processus morts s'effacent à l'ouverture. `run_e2e.sh` refuse qu'une suite laisse quelque chose dans son dossier temporaire, avec une liste de dette déclarée. Le ticket du banc de charge d'ingestion. Huit commits, de `5d2e6a919` à `92180836b`. |
| `022ade8e8` | 8 nov. | **Mes recherches, suite** : une page de la vue lit au plus 2 000 runs, `relancable` est dit par l'hôte, et les outils de recherche se lisent sans ouvrir la base. |
| `932197e79` | 8 nov. | **Mes recherches** : la vue de l'IDE qui liste les recherches faites (par la personne et par les agents) et les résultats cliqués, lues dans le journal des runs. Routes `GET /ide/recherches` et `POST /ide/recherches/clic`. Corrige au passage une course d'écriture du journal des runs. |
| `ad07c8b16` | 8 nov. | Deux lignes de `docs/nouveautes.md`. |
| `d540813d5` | 8 nov. | **Le score d'une ligne dit d'où il vient** (`bestSignal`, `chunk.signal`), et le morceau montré suit le rang, plus le score brut. |
| `0e528854a` | 8 nov. | **La recherche filtrée lit sous l'instantané de son appel** (`SearchSourceNodeFactory::snapshot_safe` rend `true`). |
| `d0b7fa678` | 9 nov. | Rapport et knowledge dump du 8 novembre (`extension/rag3weaver/docs/08-novembre-2026-recherche/`). |

Plus anciens, du 7 au 8 novembre : `ccec05572` (montée de lucivy 4.3.2, une ligne collée longue
retrouve son fichier), `617de00da` (le surlignage par les positions que rend le service
d'embarquement), `80ca3d561` (un second processus écrivain est refusé en nommant le premier),
`77d0b0676`, `aaf475d4b` et `c5a40d376` (la requête longue bornée, le filtre du creux, le creux
servi par l'hôte).

## Chantiers en cours

### 1. `arret-du-backend` : passe jouée, pas poussé

**Ce qu'il corrige.** Un *backend* (un manifeste ouvert, avec sa base et son catalogue ; classe
`Backend` de `extension/rag3weaver/src/backend.rs`) qu'on laisse tomber sans appeler
`shutdown()`, comme le font les tests, laissait des fils de fond tenir son catalogue. Le catalogue
se fermait alors plus tard, quand son dossier n'existait peut-être plus : des dossiers temporaires
à moitié effacés, l'erreur « fermeture FTS incomplète … save_metas : No such file », et un plantage
(SIGSEGV) à la sortie d'`e2e_cited_by`.

**Le correctif** :
- un seul registre d'arrêt pour les fils de fond d'un catalogue
  (`extension/rag3weaver/src/catalog_threads.rs`), qui tient le catalogue par une référence faible ;
- le rebâti du plein texte s'arrête au lot suivant ;
- `impl Drop for Backend` appelle `shutdown()`, une seule fois ;
- `Drop for Catalog` vide la vue de lecture avant de fermer le plein texte.

Il ajoute `RAG3WEAVER_E2E_SAUF` à `run_e2e.sh`, ferme le ticket
`2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md` en le rangeant dans
`docs/tickets/closed/`, et ouvre le ticket `2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`.

**Où il en est exactement.**
- Branche locale `arret-du-backend`, worktree `/home/lucied/git_workspaces/rag3db-arret`, tête
  `a0ea9dc95`, posée sur `d23af9cd2`, poussée sous `origin/arret-du-backend-7`. Les branches
  distantes `arret-du-backend` à `arret-du-backend-6` sont des états plus anciens du même lot.
- La passe a été jouée à luciepc sur `d89ae998e`, la même série posée sur `8efd3eeb0`, sous la lib
  29a7bc52a. Journaux : `~/.cache/rag3weaver-build/journaux-arret/` à luciepc.
  - **Passe ordinaire : VERTE** (lib entière 2082, navigateur 30, e2e_lectures, e2e_code,
    e2e_cited_by, e2e_juge). Les deux témoins du lot sont verts, ainsi que
    `daemon_host::worktrees::tests::owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`.
  - **Premier e2e entier** (112 suites) : **un rouge du témoin des résidus**.
    `e2e_tx_par_paquet_arret` a laissé trois fichiers `rag3weaver-lignes-<pid>-42-File-<ms>.csv`
    (12 Ko chacun) dans son dossier temporaire. Ce sont des fichiers de COPY écrits par
    `extension/rag3weaver/src/dataflow/record_nodes.rs` (`fichier_csv`) que ce test d'arrêt
    brutal laisse quand il tue un processus en plein COPY. C'est une cause à part, déjà citée dans
    le ticket des dossiers temporaires, et elle n'est pas du lot.
  - **Second e2e entier** (112 suites) : **vert**, code 0.
  - Dans les deux, `e2e_juge` dit « résidu connu » (son ticket du journal des runs), comme prévu,
    et `e2e_cited_by` et `e2e_lectures` finissent propres.
- **Pas poussé.** Il manque :
  1. trancher le rouge de `e2e_tx_par_paquet_arret` : l'inscrire comme rouge connu du témoin, avec
     un ticket « un COPY tué laisse son CSV dans le dossier temporaire », ou corriger l'écriture
     des CSV (les écrire dans le dossier de la base, ou les effacer à l'ouverture suivante) ;
  2. rebaser sur le master du moment (`3c0ec28b5` au 11 novembre, qui porte du Rust neuf depuis
     `d23af9cd2`), puis le rejeu court (script `~/.cache/rag3weaver-build/arret-rejeu.sh` à
     luciepc, dont la base est à mettre à jour) ;
  3. le dernier pas sous `poste push`, à l'ordre de l'orchestration.

### 2. Le journal des runs écrit après l'arrêt de son hôte : pas commencé

Le ticket est dans la branche `arret-du-backend` :
`docs/tickets/2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`. Il arrivera sur
master avec ce lot.
- **La cause** : `JournalDesRuns::noter` (`extension/rag3weaver/src/daemon_host/journal_des_runs.rs`)
  écrit le journal d'un run après une grâce de 250 ms, dans un fil détaché `journal-ecrire` que
  l'arrêt de l'hôte n'attend pas. C'est le résidu d'`e2e_juge`.
- **L'ordre fixé par c7** : ce lot passe juste après `arret-du-backend`, devant le Surveillant.
- **Le témoin rouge attendu** : un hôte lâché juste après un tour, et le compte des fichiers nés
  après l'arrêt.
- **Le correctif attendu** : l'arrêt de l'hôte écrit les journaux en attente de leur grâce et
  attend ses fils.

### 3. Le Surveillant : pas commencé

Ticket : `docs/recherche/tickets/2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md`.
- **Le test** : `e2e_invariant_des_vecteurs::l_embarquement_du_retard_appelle_le_service_hors_du_verrou`
  (`extension/rag3weaver/tests/e2e_invariant_des_vecteurs.rs`). Son embarqueur espion, le
  `Surveillant`, rougit si un autre fil tient le verrou du catalogue pendant un appel au service
  d'embarquement.
- **La mesure** : le test rougit 5 fois sur 10 sur master quand on le joue seul, sous la charge du
  8 novembre. Cette charge était une attente disque (le NVMe de luciepc en TRIM asynchrone), réglée
  depuis.
- **À faire** : refaire les dix essais sur disque calme, puis trouver qui tient le verrou.

### 4. Les PR de lucivy #22 et #23 : en attente du mot de Lucie

Elles sont sur `L-Defraiteur/lucivy`, ouvertes le 8 novembre par la session lucivy, CI verte,
fusionnables, sans relecture :
- **#23** (`4.3.2.2-close-arrete-ses-pools`) : `ShardedHandle::close` arrête et attend ses pools
  même quand une écriture échoue. C'est la moitié lucivy du plantage d'`e2e_cited_by` ; la moitié
  rag3weaver est `arret-du-backend`.
- **#22** (`4.3.2.1-corpus-absent`) : un test de corpus qui ne trouve pas son corpus échoue au lieu
  de passer en silence.

Aucune action de ma part n'est due : elles attendent Lucie.

## Tickets ouverts qui sont à moi (dans `docs/recherche/tickets/` depuis le 11 novembre)

- `2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md` : dans la branche
  `arret-du-backend`, pas encore sur master. Le prochain lot.
- `2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md` : le Surveillant.
- `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md` : corrigé par
  `arret-du-backend`, et rangé dans `closed/` par ce lot à son atterrissage.
- `2026-11-09-le-banc-de-charge-d-ingestion-tient-la-carte-pres-de-deux-heures-dans-l-e2e-entier.md` :
  il reste à donner les chiffres de débit, puis à jouer la suite seule sous `poste mesure`.
- `2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md` : le témoin des résidus
  et sa liste. Les suites restantes sont celles de la dette déclarée ; la branche de l'équipier
  (hotfixs) en rend une partie propre.
- Le ticket du plein texte de code-lourd qui compte plus de documents que de lignes est passé à G,
  dans `docs/optimiseur/tickets/` (11 novembre).
- `2026-11-07-une-requete-floue-de-lucivy-coute-en-proportion-de-ses-termes.md` : côté lucivy ;
  rag3weaver borne déjà la requête longue (au plus 900 caractères avec flou, au plus 64 termes).
- `2026-11-07-un-arret-ne-coupe-pas-une-requete-du-moteur-en-cours.md` : relevé par hooks-essai,
  l'arrêt d'un run ne coupe pas un appel au moteur en cours (`Connection::interrupt` existe), pas
  regardé.
- `2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent.md` : vu par recherche le
  25 octobre, pas repris.
- `2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md` :
  ma part est faite (`d540813d5`), le reste est à archi (le seuil, mesuré sur des prompts
  étiquetés).

## Ce qui reste sur disque

**Sur le portable** (`/home/lucied/…`) :
- le worktree `git_workspaces/rag3db-arret`, propre, branche locale `arret-du-backend` à
  `a0ea9dc95` ;
- les scripts de passe dans `.cache/rag3weaver-build/` : `passe_epinglee.sh`, `push_garde.sh`,
  `lib_seule.sh`, `e2e_entier.sh`. Ce sont les originaux de ceux de luciepc ; on les édite ici,
  puis on les copie.

**À luciepc** (`/home/lucied/…`) :
- le worktree `git_workspaces/rag3db-arret`, propre, détaché sur `d89ae998e` ;
- le target cargo `.cache/rag3weaver-build/target-97k` (7 Go), celui d'`arret-du-backend` ;
- les scripts du lot : `arret-ordinaire.sh`, `arret-e2e.sh`, `arret-tout.sh`, `arret-rejeu.sh` ;
- les journaux de passe : `journaux-arret` (la passe décrite plus haut), `journaux-arret.1320`,
  `journaux-residus`, `journaux-residus.8nov`, `journaux-residus-rejeu`,
  `journaux-residus-rejeu.banc`, `journaux-best`, `journaux-best2`, `journaux-suite`,
  `journaux-suite2`, `journaux-recherches`, `journaux-recherches2` (entre 160 Ko et 2,3 Mo
  chacun) ;
- des journaux isolés : `arret-rouge.log`, `arret-vert.log`, `arret-vert2.log`, `memoire-rouge.log`,
  `departage-invariant.log`, `best-rouge.log`.

Aucune base d'essai de cette session n'est restée. Les branches distantes de mes lots déjà sur
master (`dense-filtre-sous-l6`, `best-signal*`, `mes-recherches*`, `residus-de-suite*`,
`nouveautes-recherches`, `docs-recherche-8-nov`) restent sur GitHub : leur suppression est une
décision de Lucie.

## Ce qui est bloqué, et sur quoi

- `arret-du-backend` attend la décision sur le rouge de `e2e_tx_par_paquet_arret` (ci-dessus),
  puis une place dans l'ordre des pushes.
- Les deux lots suivants (le journal des runs, le Surveillant) attendent `arret-du-backend`.
- La priorité de Lucie du 8 novembre au soir, **les vues par worktree** (recherches filtrables par
  worktree, branche ou PR ; un worktree qui n'indexe que ses scopes neufs), attend la page de
  conception de la marche 1, écrite par la session « arbre principal », et sa relecture par Lucie.
  Ma part y sera :
  - le filtre de vue dans le plein texte (lucivy : une requête restreinte à un ensemble de
    versions de documents, ou un champ de vue indexé ; à trancher par mesure) ;
  - le dense filtré par vue ;
  - le drain par empreinte côté plein texte.

## Les trois prochains pas que je ferais

1. **Atterrir `arret-du-backend`.** Ouvrir le ticket du COPY tué qui laisse son CSV, inscrire
   `e2e_tx_par_paquet_arret` comme rouge connu dans `tests/e2e/suites_qui_laissent_des_residus.txt`,
   rebaser sur master, faire le rejeu court sous la lib du moment, puis pousser.
2. **Le journal des runs.** Le témoin rouge (un hôte lâché juste après un tour, les fichiers nés
   après l'arrêt), puis le correctif (`shutdown` de l'hôte écrit les journaux en attente et
   attend ses fils) ; `e2e_juge` sort alors de la liste des résidus.
3. **Le Surveillant.** Dix essais sur disque calme, master contre la branche, puis trouver le
   détenteur du verrou du catalogue pendant l'appel au service.

## Reprendre

Chaque travail non fini, avec ce qu'il faut pour repartir de son point et non de master. Les
commandes se tapent dans **bash** : sur le portable, l'outil de commande de Claude Code est zsh, et
à luciepc le shell de connexion est fish, d'où les `ssh … bash -s`. Les chemins `~` sont ceux de
l'utilisateur `lucied` sur chaque poste.

### `arret-du-backend`

- **Portable** : worktree `/home/lucied/git_workspaces/rag3db-arret`, branche locale
  `arret-du-backend`, dernier commit `a0ea9dc95` (« test(e2e): e2e_juge reste en rouge connu… »),
  posé sur `d23af9cd2`, propre. La même tête est poussée sous `origin/arret-du-backend-7`.
- **luciepc** : worktree `/home/lucied/git_workspaces/rag3db-arret`, détaché sur `d89ae998e`
  (la même série, posée sur `8efd3eeb0`), propre. Target `~/.cache/rag3weaver-build/target-97k`
  (7 Go), bâti pour ce lot. Aucune base d'essai : les tests ouvrent leurs bases dans des dossiers
  temporaires.
- **Fait** : le témoin rouge, le correctif, le second témoin, la liste des résidus réduite, les
  tickets (un fermé, un ouvert), la variable `RAG3WEAVER_E2E_SAUF`. La passe ordinaire est verte,
  le second e2e entier aussi.
- **Reste** : le rouge de `e2e_tx_par_paquet_arret` dans le premier e2e entier (des CSV de COPY
  laissés par un processus tué), le rebase sur le master du moment, le rejeu court, le push.

Commandes (portable, puis luciepc) :

```bash
# 1. Sur le portable : inscrire le rouge connu (ou le corriger), puis rebaser.
cd /home/lucied/git_workspaces/rag3db-arret
git config user.email luciedefraiteur@gmail.com      # jamais l'adresse pro
# … écrire le ticket docs/tickets/2026-11-1x-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md
#     et ajouter à extension/rag3weaver/tests/e2e/suites_qui_laissent_des_residus.txt la ligne :
#     e2e_tx_par_paquet_arret docs/tickets/2026-11-1x-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md
python3 extension/rag3weaver/scripts/check_tickets.py          # doit finir par VERT
git commit -m "…" -- <chemins>                                   # par chemins, sans ligne d'attribution
git fetch -q origin master && git rebase origin/master          # relire tout conflit à la main
git submodule update extension/rag3weaver/codeparsers
git push origin HEAD:refs/heads/arret-du-backend-8              # un nom neuf : jamais de push en force
BASE=$(git merge-base HEAD origin/master); HEAD_=$(git rev-parse HEAD)

# 2. À luciepc : mettre le worktree sur la tête poussée, poser la base, lancer le rejeu court.
ssh -T lucied@luciepc bash -s -- "$HEAD_" "$BASE" <<'X'
cd ~/git_workspaces/rag3db-arret && git fetch -q origin arret-du-backend-8 \
  && git checkout -q --detach "$1" && git submodule update -q
B=~/.cache/rag3weaver-build
sed -i "s/^J=.*/J=\$B\/journaux-arret-rejeu; mkdir -p \$J; echo $2 > \$J\/base/" $B/arret-rejeu.sh
~/.cache/rag3weaver-build/poste --qui        # lire qui tient la machine avant de lancer
nohup $B/poste lourd timeout 14000 $B/arret-rejeu.sh > /dev/null 2>&1 < /dev/null &
X
# Le verdict : dernière ligne de ~/.cache/rag3weaver-build/journaux-arret-rejeu/resume.log (VERT ou ROUGE …).

# 3. Le push, seulement si VERT et au tour donné par l'orchestration (garde : passe verte sur ce
#    commit, une seule lib, aucun code arrivé sur master depuis la base testée, puis trois contrôles).
ssh -T lucied@luciepc bash -s <<'X'
B=~/.cache/rag3weaver-build
tail -1 $B/journaux-arret-rejeu/resume.log | grep -qx VERT && \
  $B/poste push rag3db-97 -- $B/push_garde.sh rag3db-arret journaux-arret-rejeu
X
```

Avant le rejeu, vérifier la lib épinglée dans `~/.cache/rag3weaver-build/passe_epinglee.sh`, ici et
à luciepc (ligne `export RAG3DB_LIB=`) : elle doit être celle que l'orchestration désigne, par son
identifiant complet. Si le C++ de master est plus récent qu'elle, ajouter
`export RAG3WEAVER_MOTEUR_ANCIEN=1`. On édite les scripts **sur le portable**, puis on les copie à
luciepc par un fichier temporaire et un `mv`, jamais en écrasant un script qui tourne.

### Le journal des runs écrit après l'arrêt de son hôte

- **Aucun worktree encore.** Le ticket vit dans la branche `arret-du-backend` ; il arrivera sur
  master avec elle.
- **Fait** : la cause, lue dans le code, écrite au ticket.
- **Reste** : tout le lot.

Pour l'ouvrir, une fois `arret-du-backend` sur master :

```bash
cd /home/lucied/git_workspaces/rag3db
git fetch -q origin master
git worktree add -b journal-apres-arret ../rag3db-journal origin/master
cd ../rag3db-journal && git config user.email luciedefraiteur@gmail.com
git submodule update --init extension/rag3weaver/codeparsers
# Témoin rouge d'abord : un DaemonHost de test (with_runs_root sur un TempDir), un tour de chat
# (le modèle factice de tests/common/fake_sse.rs), drop de l'hôte juste après la fin du tour,
# attendre 1 s, compter les fichiers nés dans runs/ après le drop : il en faut 0.
# Correctif : JournalDesRuns garde ses fils journal-ecrire, et DaemonHost::shutdown (et un Drop)
# écrit tout de suite les journaux en attente de leur grâce, puis attend ces fils.
```

Le rouge se prouve à luciepc par `~/.cache/rag3weaver-build/lib_seule.sh <worktree> <target>
<journal> <filtre du test>`, sous `poste lourd`, avec un target neuf (par exemple `target-97l`).

### Le Surveillant

- **Aucun worktree.** Le ticket est sur master.
- **Fait** : la mesure sous charge (5 rouges sur 10 sur master seul, 6 sur 10 sur la branche), et le
  script de départage `~/.cache/rag3weaver-build/departage_invariant.sh` à luciepc, qui joue dix
  essais sur master puis dix sur une branche.
- **Reste** : refaire les dix essais sur disque calme, puis trouver le détenteur du verrou.

```bash
# À luciepc, sur un worktree de la tête voulue : dix essais du test seul.
cd ~/git_workspaces/<worktree>/extension/rag3weaver
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/<target> RAG3DB_LIB=<id complet de la lib>
for i in $(seq 1 10); do
  ./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_invariant_des_vecteurs \
    l_embarquement_du_retard_appelle_le_service_hors_du_verrou 2>&1 | grep -E "^test e2e_invariant.*(ok|FAILED)$"
done
```

## Actions encore ouvertes, rattachement aux tickets

La consigne de Lucie du 11 novembre demande un ticket avant toute transmission d'une nouvelle action. Les tickets de recherche existants portent désormais leur session et leurs sources ; aucune duplication de l'historique. Les tâches déjà décrites restent dans leurs tickets, sans nouvelle autorisation implicite de lot :

- Intégration Lucivy validée : ticket désormais dans `tickets/closed/2026-11-11-la-fermeture-fts-en-erreur-laisse-le-handle-actif.md`, recherche ; livré sur master `f96acc6cf`, contrôles et nettoyage terminés.
- Journal après arrêt hôte : `recherche/tickets/2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`, recherche, correctif et preuve encore requis.
- Surveillant : `recherche/tickets/2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md`, recherche, départage puis correction selon preuve.
- Provenance des artefacts : `recherche/tickets/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md`, recherche pour le suivi, correction durable du garde à attribuer avec orchestration ; récupération seule accomplie.
- Preuve du juge, COPY interrompu, filtre vide : leurs trois tickets du 11 novembre dans `recherche/tickets/`, recherche pour le suivi ; les sections de ces tickets disent preuves et critères de fin. Le juge se départage avec everything declarative, sans cause d'environnement présumée.


## 12 novembre — reçu du journal dans les témoins de lecture

Lot isolé `cx-recherche-lectures-journaux`, worktree homonyme aux deux postes ; baseaf2ef6846, sourcefigéeaa58d1697 publiée sur branche distante. Ticket : [racine des journaux de lecture](../tickets/2026-11-12-les-temoins-de-lecture-imposent-le-nom-du-dossier-des-journaux.md). Neuf extractions ad hoc remplacées par un helper local lisant le reçu réel, sans toucher runtime/ED/M8 ni réduire les assertions. Compilation et helpers3/0/0 verts ; attestation autonome corrigée native29/220gabarits/SHA729f0d114c65c867e44089219ab4b48e7e44776b327279d41ed99312ad94da1a. Les deux incidents d’outillage d’attestation et le rouge initial sont conservés séparément ; aucun test rejoué. Source distante reste figéeaa58, target97k privé conservé, logs hors cible sur les deux postes. Ciblélectures27/120s restant après coordination7879 ED/M8, deadlineglobale16:18:55 CET ; aucun master autorisé. Reprendre depuis le même ticket et ses preuves, sans relance aveugle ni extension implicite du budget.


**Mise à jour après ciblé :** sur source figéeaa58d1697 et même ELF729f0d…, e2e_lectures **27/0/0 en9,46s** (24 existants +3 helpers) et2filets verts sous racine privée `racine arbitraire`, sans segment commandes. Vrai poste lourd/Max32/HIGH0 et cgroups enfants attestés, santé7879 avant/après et quiescence exe/cwd vérifiées. Service rendu directement ED/M8/C7, aucun usage recherche restant, budgetglobal1800 respecté. Revue du diff/des preuves avant globale/master toujours requise ; ticket ouvert et ressources privées conservées en attendant le tour, aucun Rust poussé sur master.


**Intégration après ED/M8 :** nouvelle branchecx-recherche-lectures-journaux-integration, source487c68af3 surmasterb726d907b, Rust du correctif inchangé au range-diff. Nouveauglobal2400, features5/5 etliblarge2201/0/65 verts. NAV40 séparée arrêtée dès qu’un Chromium PID2997235 sort du scopeMax32 ; pas de résultat NAV complet, lectures27 et vrai checklib minimal non joués. Confinement remonté au ticket commun orchestration des scopes, quiescenceexe/cwd/profil privée prouvée après arrêt. Preuves2postes sousjournaux-cx-recherche-lectures-12nov/integration-487c68/. Aucunservice utilisé, aucunretry/master ; ressources propres conservées pour diagnostic/revue.
