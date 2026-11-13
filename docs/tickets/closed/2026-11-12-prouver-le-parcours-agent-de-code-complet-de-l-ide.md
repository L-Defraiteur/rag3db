# Prouver le parcours agent de code complet de l'IDE

- **État** : corrigé — source union empilée `c9e6b6e39` ; parcours réel hôte/IDE et globale verts, livraison selon ordre Pub→ED.
- **Famille** : produit
- **Gravité** : capacité principale ; l'atelier doit réellement explorer, modifier, tester et reprendre.
- **Atteignable en service** : les outils existent ; leur parcours complet dans une session réelle reste à qualifier.
- **Touche rag3weaver** : oui, chat/agent de l'atelier, outils accessibles, exécution et reprise.
- **Pour** : ED ; archi relit les contrats d'interface, sans édition concurrente.
- **Signalé par** : objectif explicite de Lucie pour l'IDE et demande de lots indépendants cette nuit.
- **Ouvert le** : 12 novembre 2026

[Répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Créer un hôte, un dossier d'état et un petit projet Git de fixture indépendants
de l'IDE personnel. Passer par le vrai chat et le catalogue d'outils du framework :
demande synthétique, recherche/lecture, modification, vrai test nommé, résultat,
diff et reprise. Utiliser Gemini via Vertex configuré quand le modèle est requis,
avec une recette et des appels bornés ; aucun secret ou chat réel envoyé.

Vérifier fichiers et effets réels, nombre/tests effectivement exécutés, travail
isolé et continuité des identités de conversation/outils. Une réussite verbale,
une fonction Rust présente sans binding ou un appel direct contournant le chat
ne satisfait pas le contrat. Les refus et accords suivent le mécanisme normal,
sans mode permissif de test masquant un défaut d'accessibilité.

Publier le premier constat/source/recette avant ciblé. Corriger les lacunes dans
les primitives et gabarits communs ; respecter propriété navigation/code_tools
et modèle/serveur des autres sessions. Pas de nouvel orchestrateur multiagent
builtin, ni de déploiement/restart personnel dans ce lot initial.

Fin : parcours réel reproductible avec outils accessibles, tests/diff/reprise
observés, corrections éventuelles relues et intégrées, limites restantes dites.
Le ticket reste ouvert si seule une partie du parcours est démontrée.

## Revue du correctif de lecture Git b0090e2be

Les preuves36e restent partielles : vrai chat Vertex, exploration, test nommé
rouge puis édition et test vert ; diff refusé128, reprise pas encore atteinte.
Le helper proposé réduit les chemins communs et conserve les écritures.

Contre-exemple concret à qualifier avant nouvel avis favorable : créer une
racine faux avec son entrée `.git` **lien symbolique vers le gitfile d'un vrai
worktree**, puis appeler avec_lecture_git sur faux et le principal du dépôt.
La comparaison actuelle canonicalise les deux gitfiles : retour enregistré et
gitfile faux deviennent le même chemin réel. Le contrôle de retour passe alors
pour une racine qui n'est pas le worktree enregistré ; son administration
(éventuellement celle d'un voisin) est ajoutée aux lectures.

Revue statique, aucun scénario de ce type exécuté par orchestration. Ajouter le
témoin négatif avec la même exigence que le gitfile copié déjà refusé ; valider
la racine de worktree enregistrée, pas seulement l'égalité après résolution du
lien final, ou refuser explicitement ce gitfile symbolique non qualifié. Préserver
le dépôt ordinaire, le worktree lié réel et ses espaces ; aucune écriture du
principal/voisin ne devient nécessaire. ED garde le correctif et sa qualification
kernel puis chat sous source figée ; cette revue ne remplace pas ces résultats.
## Reprise ED et premier constat

Worktree isolé `cx-ed-parcours-agent-ide`, branche homonyme, base `a783013f1`.
Aucun appel modèle, build ou service lancé à ce stade. Les fichiers réservés
navigation et résidence des modèles restent à leurs propriétaires.

Le témoin existant `le_chat_d_une_tache_code_sur_gemini` utilise bien le vrai
chat et Vertex mais ne qualifie que l’édition. Le gabarit code déclare déjà
la chaîne Rust en lecture et Cargo hors réseau dans son bac à sable ; les
méthodes normales relisent tâches et conversations persistées.

Préparation : fixture Git Cargo sans dépendances, test nommé immuable,
exploration par outils exposés, échec initial attendu du test, correction et
réussite effective, diff par commande autorisée puis recréation de l’hôte et
reprise. L’attestation indépendante vérifiera les octets du principal, les
identités et les appels/résultats liés, sans remplacer les actions de l’agent.
La source et la recette bornée seront publiées avant compilation/ciblé.

## Première source et recette bornée

Nouveau témoin ignoré par défaut :
`daemon_host::chat_qui_code::le_chat_code_teste_et_reprend_sur_vertex`.
Il emploie le modèle réel par défaut de l’hôte, sans callback substitué.
Trois tours : recherche/lecture et test initial attendu rouge ; édition du
seul fichier source puis test vert et diff ; nouvel hôte, registres relus,
historique identique (identifiants outils inclus), test et diff rejoués.
Le test d’intégration immuable `salutation_nettoie_le_nom` doit effectivement
s’exécuter (1 failed attendu puis 1 passed), jamais zéro test. Les preuves JSON
et la fixture sont conservées sous un parent privé, y compris en cas de refus.

Sur luciepc uniquement, target neuf propre à ce lot, lib native29 épinglée,
j8/Max32G/HIGH0, aucune charge GPU nécessaire et aucun usage 7879 prévu.
Primitive commune extérieure, budget global1900s incluant nettoyage :
compilation lib de tests1500s maximum, puis témoin exact360s maximum.
Features initiales `rag3db-native,code,daemon,openai-llm` ; pas de navigateur
à ce préfixe (preuve du contrat hôte/chat/outils, pas encore preuve visuelle).
Compilation par `cargo test --lib --no-run --message-format=json` ; extraction
de l’unique ELF lib dans le flux JSON. Sonde `--list --ignored --exact` sur
le nom complet, puis ELF `--ignored --exact <nom> --nocapture --test-threads=1`.
Attendu : exactement1 témoin, 1 passed/0 failed/0 ignored. Au premier rouge
inattendu : arrêt, preuve et diagnostic avant toute nouvelle recette.

Variables runtime : `CARGO_MANIFEST_DIR` du worktree ; `LD_LIBRARY_PATH` lib29 ;
`RAG3WEAVER_EXIGER_GEMINI=1`, chemin autorisé des identifiants Vertex (jamais lu),
projet configuré `lr-hub-472010`. États, journaux, targets de fixture et preuves
sont isolés sous le dossier privé du lot. Le target Cargo de la fixture est
celui du contrat sandbox normal, sous `RAG3WEAVER_TARGETS_DIR` privé. Aucune
politique permissive ou réponse à un accord humain n’est injectée.

Préflight statique avant tout lourd : le gabarit livré active le rattrapage
vectoriel à l’ouverture. La déclaration de cette fixture non indexée le désactive
explicitement ; `search_code` garde son balayage déclaré. Aucun modèle substitué,
ni charge d’indexation nécessaire au parcours. Cette source complète remplace la
première pour le ciblé, encore non lancé.

## Rouge réel initial conservé et correction préparée

Source `36e284824` : compilation verte134s, unique témoin rouge56.77s.
Exploration par balayage, lectures, test nommé1 failed attendu, édition et test
nommé1 passed effectifs. `git diff -- src/lib.rs` rend code128 : Landlock refuse
l’administration externe indiquée par le gitfile du worktree. La reprise n’a
pas été atteinte. Preuves PC : `parcours.log`, JSON01/02 et supervision
`validation-3_5jj1d5` : rouge101, `restes=[]`, `nettoye=true`. Aucun rejouage.

Correction commune proposée : `backend_code::avec_lecture_git`, raccordée au
montage du bac dans `backend.rs` (hors initialisation modèles réservée). Elle
compare le répertoire commun canonique avec celui du principal porté par l’hôte ;
un worktree doit être enfant direct de `common/worktrees`, avec retour `gitdir`
vers son propre gitfile. Lectures : son administration seule, puis les chemins
communs nécessaires `objects`, `refs`, `HEAD`, `config`, `packed-refs`, `shallow`.
Pas de dossier commun entier, pas de racine issue d’un lien symbolique sortant,
pas de configuration utilisateur ouverte et aucune nouvelle écriture accordée.
Les objets alternatifs extérieurs restent soumis au refus du noyau.

Témoin kernel nommé
`backend_code::tests::le_bac_git_lit_le_diff_sans_ouvrir_les_voisins` : dépôt
ordinaire et worktree lié avec espace ; diff réel ; lecture du principal privé
et HEAD du worktree voisin refusées ; copie dans le domaine permise, vers
principal/config Git/HEAD voisin refusée ; gitfile forgé, dépôt étranger et
symlink sortant ne créent aucune autorisation. Ce test de la primitive noyau
ne fabrique aucun accord dans le parcours agent.

Relecture Archi retenue : résultats JSON décodés et liés au `tool_call_id`, à
`arguments.command` exact et au code réel ; phases séparées pour ne pas relire
le test initial ; lectures des deux chemins et immutabilité tests/Cargo dès01.

Recette corrective DISTINCTE, pas remplacement du rouge initial : même target,
lib29 et conditions ; global1200s incluant nettoyage, compilation600s,
témoin kernel60s, témoin réel360s. Les preuves correctives vont sous
`parcours-agent-ide-20261112/correctif-git` ; aucun nouvel appel Vertex avant
publication de cette source et relecture du delta de permissions. Attendu
kernel1/0/0 puis réel1/0/0 ; arrêt au premier rouge inattendu.

La lecture des objets et références expose l’histoire du dépôt, pas seulement
les lignes du diff courant ; cette capacité reste en lecture et porte sur le
même dépôt connu de l’hôte. Le fichier de configuration global privé demeure
inaccessible. Si Git réclame une actualisation d’index facultative, désactiver
ses verrous facultatifs, jamais ouvrir l’administration du principal en écriture.
Source corrective figée `b0090e2be` : compilation/kernel lancés, attente d’un
avis RO sur cette source avant l’étape Vertex, dans le même budget1200s.

Correctif `b0090e2be` : compilation13.94s verte ; témoin noyau rouge0.03s
sur `git diff` code128, après lecture de l’administration désormais possible :
Git charge implicitement `~/.gitconfig`, inaccessible, puis rend une erreur
fatale de configuration. Aucun Vertex correctif appelé. Supervision rouge101,
restes vides et nettoyage vrai, preuves `correctif-git/kernel.log` conservées.

Delta motivé : par défaut dans le bac, `GIT_CONFIG_GLOBAL=/dev/null` et
`GIT_OPTIONAL_LOCKS=0`, sauf valeur explicitement déclarée dans `sandbox.env`.
La configuration du dépôt reste lue ; la configuration personnelle n’est ni
ouverte ni copiée. Aucun droit ajouté. Nouvelle étape corrective2 distincte,
même recette1200s/compile600/kernel60/réel360, preuves `correctif-git-2` ;
Vertex attend toujours la revue de cette nouvelle source.

Source `b310b5c0c` : compilation puis noyau1/0/0 vert0.10s. Aucun Vertex :
Archi relève un alias INTERNE encore accepté (`refs` vers
`common/worktrees/voisin`). La portée en attente a été arrêtée volontairement
par pidfd du superviseur1164394/start87036829, identité et argv vérifiés dans
son reçu privé, avant toute ouverture de la gate modèle. Ne pas confondre ce
vert partiel avec le parcours complet.

Correction de la réserve : chaque chemin commun accordé doit être exactement
le chemin canonique attendu, pas seulement un descendant du commun ; les alias
internes et externes n’ajoutent donc aucune racine. Témoin supplémentaire :
`refs` vers administration voisine et `config` vers son HEAD ; lecture noyau du
HEAD voisin toujours refusée. La configuration explicitement déclarée conserve
sa priorité sur le défaut isolé. Étape corrective3 distincte même recette1200s
et target, preuves `correctif-git-3`, attente de revue avant Vertex.

Revue C7 `65b57052f` : autre contre-exemple statique, faux `.git` symbolique
vers le gitfile réel, dont la canonicalisation confondait les occurrences.
Réserve retenue avant tout Vertex correctif. Le gitfile final symbolique est
maintenant refusé avant découverte des métadonnées ; témoin négatif ajouté
au côté du gitfile copié. La portée corrective3, noyau1/0/0 vert0.12s, est
arrêtée volontairement par pidfd du superviseur attribué avant la gate.
Nouvelle source corrective4 à relire, mêmes bornes1200/600/60/360 et target,
preuves `correctif-git-4` distinctes ; aucun appel modèle pour les étapes2/3.

## Préfixe réel acquis, accès depuis la page restant à qualifier

Source `4d6045cdb` : compilation9.91s verte, noyau1/0/0, parcours Vertex1/0/0
vert46.05s ; supervision verte rc0/restes[]/nettoyé. Reçus01/02/04 : 10/20/26
tours, recherche/lectures, Cargo101 nommé puis Cargo0 nommé, édition/diff0,
historique et identifiants relus, nouveau Cargo0/diff0. Principal inchangé,
tests/Cargo immuables. ELF archivé immédiatement hors target, SHA conservé.

Limite observée : le manifeste principal désactive le rattrapage, mais le
manifeste de tâche le réactive ; la relecture a donc lancé un index plein texte
local (2 morceaux sans vecteurs), achevé. Les services dense/sparse absents sont
explicitement signalés ; aucun usage7879/GPU ni modèle substitué. Ne pas dire
« aucune indexation » ; la preuve reste celle des appels et effets observés.

Étape UI ajoutée, sans changement du runtime corrigé :
`daemon_host::chat_qui_code::le_parcours_vertex_se_reprend_dans_la_page`.
Elle exige la fixture explicitement passée et le verdict vert précédent ;
aucune recréation silencieuse. Nouvel hôte HTTP privé, mêmes tâche et historique,
vrai Chrome, message envoyé par le bouton de chat, test nommé et diff avec reçus
liés aux appels réels, relecture du diff dans la page, console sans erreur et
code/principal inchangés. Les helpers d’attestation de reçus sont factorisés,
le préfixe réel acquis n’est pas rejoué à l’identique.

Recette UI distincte : global1100s incluant nettoyage, compilation600s sous
`rag3db-native,code,daemon,openai-llm,navigateur`, même target/lib29/j8/Max32/HIGH0,
puis unique témoin UI360s maximum (1/0/0 attendu). Paquets React épinglés déjà
installés, navigateur requis, chemin fixture source4d explicitement enregistré.
Un seul tour Vertex supplémentaire. Évidence `page-ide` (HTML chat/diff, JSON
état, logs, SHA ELF et supervision), hors fixture acquise. Aucun déploiement ni
accès au daemon personnel. Arrêt au premier rouge imprévu.

UI source `ec57ae39b` : compilation34.03s verte, témoin rouge0.36s au démarrage
CDP (`navigateur.rs:232`, canal fermé), avant tout nouveau tour Vertex. Reprise
hôte/identité/historique préalables passent ; aucune preuve de page acquise.
Supervision rouge101/restes[]/nettoyé. Le helper jetait stderr : instrumentation
annoncée à Archi, frontière libre confirmée, enregistrement stderr sous profil et
panic avec état du fils + stderr, sans modifier argv/sandbox/bus du navigateur.

Diagnostic distinct sans modèle : nouveau témoin ignoré
`daemon_host::navigateur::le_demarrage_cdp_du_parcours_est_diagnostique`.
Même lancement CDP puis `Browser.getVersion`, borne30s, compilation600s,
portée globale700s, source publiée avant exécution ; preuves `diagnostic-cdp`.
Aucun autre parcours Vertex/UI lancé avant explication de ce démarrage rouge.

Diagnostic `fe0a5be4e` : compilation14.97s verte puis rouge0.24s, Chrome abort134
avec stderr fatal `Socket path too long` : le TMPDIR de preuves long dépasse la
limite du socket Unix `SingletonSocket`. Ce n’est pas un refus D-Bus ou sandbox,
ni une indisponibilité du modèle. Aucun appel Vertex dans ce diagnostic.

Correction de recette uniquement, source/ELF `fe0a5be4e` inchangés : TMPDIR privé
court créé par `mktemp -d "$HOME/.cache/r3v-ed-XXXXXXXX"`, chemin enregistré.
Ni flag Chrome, ni portée/bus, ni racine de la fixture Git avec espace ne change.
Pas de nouveau Cargo nécessaire : ELF archivé du diagnostic, même lib29.
Étape `page-ide-2`, budget global450s incluant nettoyage : CDP30s doit rendre
1/0/0 vert avant le témoin UI360s, puis unique tour Vertex supplémentaire.
Premier rouge conserve les preuves et stoppe, sans retry. Le TMPDIR n’est rendu
qu’après quiescence de la portée propriétaire.

## Parcours complet composé vert et passe d’intégration annoncée

`page-ide-2`, même ELF/source `fe0a5be4e` : diagnosticCDP1/0/0 vert0.23s,
puis reprise UI1/0/0 verte19.71s. Message envoyé par le bouton réel, test nommé
1 passed et diff0 liés à leurs appels, diff rendu dans la page de relecture,
historique conservé, code de tâche et principal inchangés, aucune erreur console.
Supervision verte0/restes[]/nettoyé. Cette étape compose le préfixe réel source4d
avec une reprise de la même fixture ; elle ne prétend pas refaire trois tours.

Rebase sur `origin/master=65b57052f`, branche neuve
`cx-ed-parcours-agent-ide-2`, source `39e6f0346` publiée. Seul conflit documentaire,
revue C7 intégralement conservée ; quatre fichiers Rust du lot identiques octet
pour octet à la source UI `fe0a5be4e`. Aucun nouvel appel Vertex requis au rebase.

Passe globale à publier sous cette branche avant lancement, globale6000s via
primitive canonique, lib29/j8/Max32/HIGH0 et unique target déjà utilisé (5.8G à
l’entrée). TMPDIR privé COURT (limite socket mesurée), journaux de tests sous
racine privée avec espace. Burn CPU ; pas de service d’embarquement dans la lib.
Étapes : F5 (900s par jeu, arrêt immédiat au premier rouge), check minimal lib
300s, compilation ELF lib large/NAV/e2e (1500s au total), lib entière900s,
NAV600s, puis e2e lectures27/120s, hôte13/120s, PostgreSQL sur base privée/300s.
Inventaires et décomptes exacts lus avant exécution ; filets2 à chaque entrée
e2e, zéro test ou saut non déclaré rouge. Les deux témoins cloud et le diagnostic
ignorés restent des étapes acquises distinctes, pas des sauts cachés de la lib.

Les entrées communes NAV/e2e héritent du reçu vivant, sans scope sœur ni verrou
supplémentaire. 7879 n’est demandé qu’au moment des lectures27 après compilation,
avec annonce prise/fin et santé avant/après ; pas de réservation durant les
phases CPU. Base PostgreSQL neuve `cx_ed_parcours_20261113`, aucune donnée réelle.
Premier rouge inattendu stoppe la portée et garde preuve/diagnostic ; aucun retry
inchangé. Après passe/revue, tour C7 puis intégration et nettoyage gardé2postes.

## Union après livraison des symboles

Préunion source8e : F5 5/5, minimal0, trois ELF archivés, lib2244/0/66 verte46.70s,
NAV40/0/0 verte131.91s. Inventaires lectures27/hôte13/PG36 validés, aucune suite
service encore jouée. Relevés PID/start/mapsnative29/Max32/HIGH0 réels conservés.

Master a reçu la navigation3ed pendant cette passe. Arrêt volontaire au gate
`ATTENTE_7879`, avant toute utilisation du service, via pidfd du superviseur
1491979/start87211120 validé. Le verdict de portée rouge pour signal reste
conservé ; les phases vertes ne constituent pas une globale complète.

Rebase sans conflit, branche NEUVE `cx-ed-parcours-agent-ide-3`, source1eaa6e598,
base8f604328a incluant symboles3ed et C++ formaté. Revue C7 préservée. Les témoins
chat/UI et le helper de démarrage sont identiques àfe0 ; le helper Git garde son
comportement relu, READ_NODES des symboles reste à sa place.

Recette UNION même F5/min/3ELF/lib/NAV/lect27/hôte13/PG36 et paramètres, mêmes
worktree/target/basePG ; preuves `globale-union`. Budget = RESTE strict de la
deadline initiale monotone878111.26244974 (pas un nouveau6000). La préparation,
attente et la préunion sont comprises ; chaque phase conserve ses bornes ou le
reste inférieur. Source publiée avant compilation. Aucun rejeu Vertex nécessaire
pour cette union de code déjà attesté ; les suites service ne jouent qu’ici.

## Verdict final composé et livraison préparée

Union sourcee4 : F5 5/5, minimal0, lib2253/0/66 en46.86s, NAV40/0/0 en131.07s,
lectures27/0/0 en9.54s, hôte13/0/0 en12.32s, PostgreSQL36/0/0 en7.48s ; filets
2/0/0 à chaque entrée. Services BGE-M3 réel1024/sparse vérifiés avant/après,
7879 rendu. Supervision union verte0/restes[]/nettoyé et quiescence indépendante[].
Contrôle global borne supérieure1446.97s/6000, préunion/préparation comprises,
deadline monotone878111.26244974 préservée, sans nouveau6000.

Le helper Git est identique octet par octet à4d ; les témoins chat/UI et le helper
navigateur sont identiques àfe0. Équivalences SHA persistées avec le verdict.
Douze appels ont chacun un reçu unique, historique26→32 avec préfixe identique.
La preuve réelle est composée des sources4d/fe0 et de cette union validée,
aucun nouvel appel modèle simplement pour le rebase.

Le lanceur e2e canonique pose `CARGO_INCREMENTAL=0` : il a produit un quatrième
ELF e2e, également archivé/SHA, et les trois suites ont réellement joué sur lui.
Les trois ELF préparés et celui effectivement exécuté restent distingués.
Target29704553416octets<32Gio, source223gabarits, lib29 et cgroups32/HIGH0 relevés.
Dossier privé2postes : `I-luciepc/parcours-agent-ide-20261112/globale-union/`,
`verdict-compose.json`, équivalence-code, SHA/ELF, inventaires, JSON/HTML, logs,
rouges et arrêts historiques. Aucun vert n’efface ces limites de provenance.

Revues RO Git C7/Mémoire favorables, revue Mémoire deltaUI/stderr favorable ;
préparation du push selon tour C7 puis contrôles et retrait gardé2postes.
Cette livraison ne déploie ni ne redémarre le daemon personnel.

## Livraison empilée demandée par C7

Avant tout push master, C7 demande l’union Pub+Symboles+ED : base figée Arbre
`31db55abbfe928234de646895819ac12bae4e9fd`, branche
`cx-arbre-publication-composite-r2`, parent8f604328a. Empilage de nos seuls
commits depuis8f, nouvelle branche4 `cx-ed-parcours-agent-ide-4`, source9ad2d7bdb
publiée ; conflit documentaire news résolu en conservant les deux capacités.
Les fichiers Publication/KV d’Arbre sont inchangés, tout comme Git/UI acquis.
Le candidat3024 et son script push restent historiques, jamais exécutés sur master.

Recette même F5/min/3ELF/lib/NAV/lect27/hôte13/PG36 sous portée canonique, source
empilée publiée avant compilation, preuves `globale-union-pub`. Même target/basePG,
Max32/HIGH0/j8/BurnCPU, TMPDIR privé court et racine journaux avec espace. Reste
strict de deadline878111.26244974, calculé APRÈS acquisition poste (environ3900s
à préparation), aucune remise à6000. Les tests CLI5/tx25 de Pub+Symboles sont
acquis par Arbre sur31, avec fichiers Pub identiques ; pas de rejeu aveugle de
ces composants ni de Vertex. Ordre master Pub→ED→Embar fixé par C7.
Ancien dumpPG conservé distinctement ; export final rafraîchi après cette union.

## Union empilée C9 verte

F5 5/5, minimal0, lib2267/0/66 en47.57s, NAV40/0/0 en131.66s,
lectures27/0/0 en9.73s, hôte13/0/0 en12.45s, PostgreSQL36/0/0 en7.62s ;
filets2/0/0 chaque e2e. ScopeVERT0/restes[]/nettoyé, quiescence indépendante[].
Le contrôle2834.58≤6000 garde deadline878111.26244974 ; reste3809 effectif
calculé après poste. Un chemin de relevé budget a repris celui de l’étape
précédente : relevé Pub préservé au bon dossier, ancien restauré depuis la copie
portable fidèle, runner utilisé et note d’audit conservés ; aucune incidence
sur le délai effectif ni rejeu. Quatre ELF archivés, dont le canonique e2e
INCREMENTAL0, et maps runtime NAV/e2e/native29/CG32. Target29846025589octets.

Pub31 : neuf fichiers produit identiques, CLI5 et transactions3+1+18+3 qualifiés
par Arbre sur cette base, pas revendiqués comme rejoués ED. Les helpers/tests
réels4d/fe0 restent équivalents SHA. BGE7879 rendu, santé réelle/sparse avant/après.
Dossier `globale-union-pub/verdict-compose.json`, rawlogs, équivalences et source
223gabarits. DumpPG final rafraîchi, ancien avant-Pub et ses SHA gardés.
La livraison attend Pub master puis équivalence/rebase MD ; aucune mutation Rust
après C9, aucun push avant « à toi » C7.

Signal indépendant d’Embarquement sur son union842 : carte de `show_progress`
absente au second outil d’une fixture sans barrière temporelle. Diag RO transmis
sous son ticket activation/résidence : contrat « index actif », deux causes
possibles (index fini ou verrou indisponible), proposition de fixture contrôlée
sans grossissement/sleep ni changement runtime. C9 reste figé, aucun Cargo ou
modèle supplémentaire joué pour ce diagnostic ; son routage reste coordonné C7.

Base Pub effectivement livrée `bc1a81500a05489673b64c2e5930d335510e6dda` ;
rebase de nos commits sur elle, branche5, delta sourceC9 Markdown/JSON seulement.
La qualification reste celle de C9, sans rejeu du modèle ou des phases inchangées.

Le signal de carte a reçu un correctif TESTSEUL compagnon `08c379b11` (parentC9),
qualifié exact1/0/0 en3.70s après compileF3/native29, scope0/restes[]/nettoyé et
quiescence[]. Barrière réelle, RAII et cas terminé ajouté, assertions initiales
maintenues. Transmis à Embar pour son union, sous ticket activation ; ce correctif
n’est pas mélangé au candidat ED dont le Rust reste C9 figé.
