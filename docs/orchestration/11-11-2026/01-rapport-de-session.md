# Orchestration — rapport de session du 11 novembre 2026 (16 h 30)

Session « orchestration sessions » (c7). Ce rapport dit l'état du projet pour un agent qui reprend
l'orchestration sans avoir lu nos échanges. Le knowledge dump (`02-knowledge-dump.md`) dit les règles.

## Ce qui se passe ce 11 novembre

Lucie passe ses sessions sur Codex pour garder du crédit pour son travail. Chaque session écrit son
rapport et son knowledge dump sous `docs/<session>/11-11-2026/`, et ses tickets ouverts iront dans
`docs/<session>/tickets/` dès que `check_tickets.py` saura les lire (lot demandé à la session
coeur-cpp-hotfixs-tickets ; le mot « déplace » part à son hash). Les visions sont rassemblées sous
`docs/orchestration/visions/`, les nouveautés résumées sous `docs/orchestration/news/`.

## L'état des postes (dernier relevé : 9 novembre, 13 h 42)

- **Lib du moteur posée aux deux postes** : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
  (md5 b65cc5be), le correctif des deux sens des petites tables de relations (29a7bc52a). Au magasin
  aussi : 9bae60786 (gardes de l'étendue) et cbc06c3e1. Toute passe épingle cette lib par son
  identifiant complet, sans `RAG3WEAVER_MOTEUR_ANCIEN`.
- **Binaire rag3weaver posé** : 0b0ff221a (tête de master du 9 à 13 h 38), luciepc à 13 h 39, portable
  à 13 h 41 ; depuis, master a reçu a6637eeee, be319fa75, 3e8c87aa5, ab8272deb (archi, 96) : **une pose
  du binaire est due** (lib inchangée). Session optimiseur : `preparer-binaire.sh <hash>` puis
  `poser-binaire.sh <hash> luciepc` puis `portable`, préavis d'une minute, l'atelier de Lucie
  rouvert avec ses 11 outils.
- **Les bases de Lucie gardent leur ancien écart des deux sens** (INHERITS_FROM 650/658, LOCKS
  544/546) : le correctif empêche d'en créer, il ne répare pas. Réparer = un index complet de ses
  bases, à son mot (sa carte, son heure).
- **luciepc** : `discard=async` retiré, `fstrim.timer` hebdomadaire ; Lucie doit poser le drop-in
  `sudo systemctl edit fstrim.service` avec `ExecStart=/usr/bin/fstrim --listed-in /etc/fstab:/proc/self/mountinfo --verbose --quiet-unsupported --minimum 16M`
  et une heure creuse, avant le lundi 16 novembre 00 h 02. Disque à ~65 %. Plafond `lourds-max` : 4.
- **Services d'embarquement à luciepc** (card2) : 7878 granite-278m, 7879 bge-m3, 7880 granite-107m,
  7882 granite privé (hotfixs). Tunnels depuis le portable : 7979 → 7878, 7980 → 7879, 7981 → 7880.
  **Jamais un cinquième service sur card2** (82 % de VRAM : le cinquième a mis la carte en reset de
  ring le 9). card0 est celle de Lucie quand elle joue : à son mot seulement.

## La file des pushes au 9 novembre, 14 h 40 (ce qui a atterri ce jour-là)

29a7bc52a (cœur, deux sens) · 833c89539 (tickets du cœur) · 120a5db31 (hooks-essai, le rappel dit ce
qu'il attendait) · 3b119beb6 (96, fiches classées, allow_create éteint) · 92180836b (recherche,
résidus de suite) · a9872fb87 (96, bancs hors de l'e2e entier) · 6a4d55d13 (96, mesure de l'équipier,
trois tickets) · 8efd3eeb0 (archi, délai des témoins de l'hôte) · d23af9cd2 (archi, l'atelier lit et
écrit) · 0b0ff221a (96, dossier d'état des témoins) · a6637eeee (96, recul du modèle borné) ·
be319fa75 (archi, commandes en attente sur la carte, famille de commandes corrigée) · 3e8c87aa5
(archi, carte de chemin relance) · ab8272deb (archi, jauge du contexte).

## Ce qui restait ouvert, session par session (à relire dans leurs rapports du 11)

- **everything declarative** : le correctif des réclamations du rattrapage (témoin 219053a4b ; la CI de
  master rougit par intermittence sur `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`
  tant qu'il n'est pas atterri) ; la relecture des pages (vues par worktree, l'atelier lit et écrit,
  M8) ; le ticket « une tâche se dit faite sans livrable » (règle de teammate.rs) ; les défauts de
  l'installeur de l'atelier ; le lot « commit des gabarits livrés ».
- **recherche** : arret-du-backend (branche arret-du-backend-6, passe lancée le 9 à 13 h 24, verdict non
  reçu) ; le journal des runs écrit après l'arrêt de l'hôte (ticket, lot suivant) ; le Surveillant ;
  lucivy PR #22 et #23 (fusion au mot de Lucie dans la session lucivy).
- **rag3weaver archi** : `ide-par-le-flux` (wt-flux, a9c5a3108, passe tuée code 137 : à rejouer) ; le
  découpage de ce qui pèse au survol de la jauge ; les propositions et le fichier ouvert par le flux.
- **mémoire (96)** : l'archivage à blanc des docs sur une copie de la base de Lucie (bloqué par un 400
  INVALID_ARGUMENT de Vertex muet : ticket et petit lot « le client garde la requête d'un 4xx ») ; M8
  après relecture d'ED ; le classeur local après le mot de Lucie (la mesure dit que le contenu manque,
  pas le juge : 92 % des rappels vides).
- **codeparsers** : require-js (mesure finie le 9 à 02 h 31, lecture jamais rendue : session bloquée sur
  une approbation) ; ticket import_binding.
- **coeur c++** : l'annulation inachevée après un pool plein (témoin réduit) ; fermer le ticket de
  l'index complet quand hotfixs a rejoué sa trace sous 29a7bc52a.
- **coeur c++/hotfixs/tickets** : le rejeu de la trace exacte sous 29a7bc52a (7882/7992) ; le script
  des tickets pour `docs/*/tickets/`.
- **banc** : le banc Qwen3-4B en GGUF par llama-server (bâtir llama.cpp à luciepc depuis les sources
  épinglées, ou paquet pacman par Lucie : **à trancher par Lucie**, mon mot ayant été refusé par le
  classificateur).
- **optimiseur** : la pose du binaire due ; la nuance de la garde du réveil (un service lent n'est pas
  absent).
- **hooks-essai** (session fermée) : page « la recherche de code par les hooks » en attente du mot de
  Lucie (`extension/rag3weaver/docs/07-novembre-2026-hooks-essai/04-…`).

## Pour Lucie (sa main)

- Trancher la voie de banc (llama.cpp bâti ou paquet) ; lire les pages des vues, de l'atelier, des
  hooks, du classeur local ; le drop-in fstrim ; les hooks dans toutes les sessions et la ligne CLAUDE.md
  « search_code/usages d'abord » ; le mot pour les fusions lucivy ; l'index complet de ses bases ; le
  ménage des 476 `livre.json` de témoins listés dans le ticket de 96 ; la suppression des branches
  distantes fusionnées (jamais sans elle).

## Les trois prochains pas de l'orchestration

1. Recevoir les rapports et dumps des treize sessions, puis « déplace » pour les tickets au hash du
   script ; relire `docs/README.md`.
2. Faire poser le binaire de la tête de master (optimiseur), une seule coupure.
3. Remettre la file à l'agent suivant : d'abord ED (réclamations, CI), puis recherche
   (arret-du-backend), puis archi (ide-par-le-flux).

## 16 h 32 — la passation, ce qui est atterri

- `docs/README.md`, `docs/orchestration/{11-11-2026,visions,news}` : 45c7c9640 (c7).
- Le script des tickets lit `docs/*/tickets/` : f154356ea (hotfixs) ; « déplace » donné aux treize sessions.
- Rapports et knowledge dumps poussés : archi 25d0d5b8c + e34e3bbc3 (Reprendre : wt-flux, ide-par-le-flux
  a9c5a3108, passe tuée code 137) ; banc 1869ac9d7 (llama.cpp : choix (1)/(2) à Lucie) ; mémoire 72c1f2a77
  (Reprendre : wt-mf-96 lot-96-requete-refusee 42ac0bb9c + stash, rag3db-p1r-96 à luciepc, l'archivage à
  blanc ; le lot du refus 4xx vert non poussé, « à toi » donné) ; tty f6aa472c6 + c2fd6048e (rien
  d'inachevé) ; optimiseur 9fe9663af (+ 17 scripts sous docs/optimiseur/scripts/) ; arbre principal
  6661b9984 (Reprendre : wt-deplace, mesures M3 à M5 ; le sous-module codeparsers de l'arbre de Lucie
  extrait à 672a0d5 pour un pointeur c660893 : `git submodule update` par elle) ; recherche 3fb0caa46
  (arret-du-backend-7 a0ea9dc95 : passe ordinaire et second e2e entier verts, un rouge de résidus à
  trancher — trois CSV de COPY d'un processus tué ; journal des runs et Surveillant non commencés) ;
  lucivy ccfcd96b1 (la version publiée est la 4.3.2 du 7 novembre ; #22 et #23 au mot de Lucie) ;
  cœur 5b59d923f (le témoin de l'annulation sous pool plein poussé non joué, annulation-sous-pool-plein
  6e7f11680) ; codeparsers 1d111891b (un commit de l'oracle retrouvé et poussé, 280b5a3f1) ;
  everything declarative c5c64edcd (**le correctif des réclamations, reclamations-rendues 9da8e4433,
  n'a jamais été compilé ni joué** : premier lot pour Codex).
- Tickets déplacés : archi 7ca48077d (4), optimiseur 50a37c80b (1) ; les autres en cours.
- En attente : embarquement (rapport), hotfixs (B et C), les déplacements restants.
- **16 h 34, tickets déplacés** : recherche 37849771f (7), banc a28d8efc8 (9, chemins corrigés 4bc2538f7, le
  ticket du point de reprise qui fige rendu au cœur 3e795b1f5), tty f446db191 (1), arbre principal
  8dce5a1bd (6), codeparsers b81fed2d6 (9, plus le ticket import_binding écrit), cœur 33192de03 (4),
  everything declarative 9e4a3bbcb (8), mémoire 14645b23c (ses tickets, dont les quatre de l'équipier :
  deux corrigés à fermer — recul borné a6637eeee, dossier d'état 0b0ff221a —, la fin sans livrable à
  rendre à ED), lucivy 27b6592c1 (la requête floue fermée, corrigée par la 4.3.2). Index : 340 en-têtes,
  108 ouverts, 231 fermés. Deux cas pour le script (hotfixs) : comparer à la merge-base plutôt qu'à la
  pointe d'origin/master, et reconnaître un déplacement de session à session. En attente : embarquement
  (rapport et tickets), hotfixs (rapport, dump, tickets), mémoire (fermetures et lot 42ac0bb9c).
- **16 h 35, la passation est complète, sauf les fermetures de mémoire.** embarquement 542198abe (rapport,
  dump) et 01cbe59d6 (1 ticket) ; cœur 75fc487e7 (cinq tickets, le point de reprise qui fige revenu du
  banc) ; hotfixs c0fdba70c (rapport, dump, `outils/` : rejoue.cpp, synth.cpp, push-lot.sh, push-docs.sh ;
  **le rejeu de la trace sous 29a7bc52a n'a jamais tourné**, bloqué sur une approbation le 9), 57a8a1365
  (`--lot` comparé à la fourche), 67df4ee67 (déplacement de session à session, déjà juste), 7576ae673
  (cinq tickets, et la section « Les tickets » de docs/README.md complétée : fermer, déplacer, les refus,
  les six formes de ligne de `--lot`). lucivy : ses cinq tickets à jour sur `tickets-2026-11-07` (6ec2e01),
  fusion au mot de Lucie. Il reste 46 tickets ouverts sans session dans `docs/tickets/`. Le service privé
  7882 d'hotfixs (pid 912225 à luciepc) tourne encore ; son rapport dit comment l'arrêter.
- **16 h 45, la passation est close.** mémoire a poussé 2d6a08a65 (le client garde la requête fautive d'un
  4xx du fournisseur ; rejeu court vert) puis d1ef15f9a (deux tickets fermés — le recul borné a6637eeee, le
  dossier d'état 0b0ff221a —, la fin sans livrable rendue à everything declarative). Index : 341 en-têtes,
  106 ouverts. Les treize sessions ont leur dossier sous `docs/<session>/`. Master à d1ef15f9a plus ce
  rapport. La suite est à Codex, en commençant par `docs/README.md`.
- **17 h 14, le banc en GGUF est fait** (banc, 96a98a452, page `docs/banc/11-11-2026/03-le-juge-en-gguf.md`) avec le
  llama.cpp que Lucie avait déjà bâti à luciepc (`~/git_workspaces/llama.cpp/build-hip`, b9590) : Qwen3-4B
  Q8_0 en cache de préfixe égale bf16 (0,76, AUC 0,92, même première fiche à 97 %), 1 076 ms par cas de 5
  (83 ms par fiche en cache), 3 288 ms sans cache ; Q4_K_M 0,76 à 2 k mais 0,72 en forme courte. Forme de
  production : Q8_0 par llama-server avec cache_prompt, ~1,3 s pour k = 5. Le drop-in fstrim est posé par
  Lucie (`--minimum 16M`, le dimanche à 4 h) ; pacman n'a rien changé. Le sous-module codeparsers de son
  arbre remis sur c660893. Le service 7882 reste en route pour Codex. Page de reprise pour l'orchestrateur
  Codex : `03-reprise-par-codex.md` (75dd47a34).


## Reprise des sessions et premier atterrissage après la passation

- Les treize sessions ont reçu la demande de contact du document `04-contacter-orchestration-codex.md`.
  Les communications fonctionnent après relance ; SSH vers luciepc est confirmé par plusieurs sessions,
  sans modification de `/etc`. Les demandes de confirmation techniques ont été désactivées par Lucie ;
  les décisions qui lui sont réservées restent inchangées.
- L’orchestration est la session `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`.
  Lucie a remplacé l’ancienne session archi par `01a1cb70-7ec9-7d53-a7d1-2a76313d457d` :
  celle-ci reprend sans attendre l’ancienne, après inspection des processus locaux et distants.
- **Atterri : e97bae29053d0041d6af948dc9af98675d56aac3**, docs seules, cœur C++ : ticket de l’index complet
  fermé après relecture des deux traces exactes d’hotfixs sous la lib complète 29a7bc52a. Trace d796 :
  797 lignes, 31 écritures surveillées ; item2 : 2 lignes, 1 écriture ; zéro erreur et zéro écart.
  Preuves à luciepc : `~/.cache/rag3db-tickets-notes/reprise-codex-11nov-29a7/{d796,item2}.log`.
  Le cœur a confirmé status propre, aucun rebase et origin/master égal à HEAD. Hash et diff relus
  par l’orchestration. Cela ne répare pas les anciens écarts des bases vivantes de Lucie.
- **Cœur C++** : rouge désormais observé sur COPY de longues clés STRING sous pool plein ; ROLLBACK
  échoue, puis la base refuse BEGIN et COUNT. Cas INT64 voisin vert. Témoin sauvegardé séparément
  sur `annulation-sous-pool-plein`, 65486c02d ; diagnostic de l’allocation en cours, aucun correctif livré.
- **ED** : après trois non-reproductions du témoin intermittent, le témoin des morceaux refusés a rougi
  sans correctif : dette de 6 au lieu de 0 après réouverture avec service sain.
  Preuve : `~/.cache/rag3weaver-build/I-luciepc/reclamations-20261111/refus-rouge-1.log`.
  Correctif et passe à venir ; priorité de file ED, puis recherche, puis archi.
- **Recherche** : `arret-du-backend-9`, 77f6704e5, rebasé sur d1345a115 ; ticket des CSV de COPY tué
  sauvegardé. Rejeu neuf sans MOTEUR_ANCIEN : garde passée, deux témoins verts, suites en cours.
  Le premier rejeu avec dérogation inutile a été interrompu, sans verdict revendiqué.
- **Optimiseur** : binaire d1345a115a8044c5b5f5947e26d5aaba8d340053 préparé aux deux postes,
  lib complète 29a7bc52a inchangée. Hôtes d’essai et manifestes contrôlés ; aucune pose ni coupure.
- **Archi** : anciens processus absents aux deux postes, cause 137 toujours inconnue ; témoin navigateur
  relancé avec journaux neufs. **Arbre principal** : garde-fou isolé sur branche
  `arbre-principal/deplace-sans-reembarquer-2`, 30b5b146e ; rouge de sensibilité observé par mutation
  temporaire restaurée, passe encore à jouer. M3–M5 restent différées.
- **Codeparsers** : require-js différé, les 415 liaisons ajoutées sont muettes pour l’oracle et ne prouvent
  aucun gain. Quatre témoins ciblés : trois rouges, un vert ; correctif minimal des exports par défaut
  autorisé, sans changement de politique externe, d’oracle ou de seuil. A/B strict requis ensuite.
- **Mémoire** : premier document sur copie neuve et binaire vérifié d1345a115 : 2 sections sur 7 réussies,
  cinq HTTP400, 46,9 s. La piste des noms d’enum est expérimentale ; alias à rejouer sur binaire vérifié.
  Aucune validation des 50 pages. **Banc** : protocole k=10 préparé, attend le mot de Lucie pour card0.
  **Lucivy** : fusions et tags attendent sa décision. **TTY** : disponible, aucun nouveau lot lancé.

Aucun tour de push de code n’a été donné pendant cette reprise. Les résultats ci-dessus sont ceux
communiqués par les sessions ; seuls le hash et le diff de l’atterrissage docs ont été relus ici.


### Atterrissage mémoire : diagnostic des refus Vertex

- **133b9013e7585e90001a97bf70f4b7bbdc80c10e**, docs seules : note de reprise de l’archivage,
  rapport mémoire et ticket des noms longs. Hash et diff relus par l’orchestration ; mémoire a
  confirmé status propre, aucun rebase et origin/master égal à HEAD après fetch.
- Sur binaire vérifié d1345a115 et copies isolées, noms d’origine : 2/7 sections réussies,
  cinq HTTP400 en 46,9 s ; alias courts : 7/7, zéro refus en 113,5 s. Lib complète confirmée
  dans maps. Vert de diagnostic uniquement, aucune passe ni modification produit livrée.
- Ni le champ ni un seuil précis du refus ne sont établis : les alias modifient l’enum et sa
  présentation. Prochain lot : résolution vérifiée alias vers nom exact, correctif du gabarit
  et passe ; aucun lancement des 50 documents ni écriture dans la mémoire réelle.
- ED annonce ses deux témoins ciblés verts après correctif sur `reclamations-rendues-2`,
  1d5eefde2ab41db4bee577b6ec3607a0535dedcc ; passe complète en cours, aucun verdict global.
- Archi : navigateur ciblé vert ; passe complète distante relancée explicitement après
  interruption propre d’un premier lanceur au mauvais mode. Journaux faisant foi :
  `flux-passe-complete-20261111-luciepc-2` ; aucun verdict global.
- Le privé 7882 est transmis explicitement par hotfixs à codeparsers pour son A/B, réservation
  confirmée par optimiseur : même service granite-278m, aucun arrêt ni cinquième service card2.


### Cap et convention confirmés par Lucie

Deux axes produit parallèles : vues par worktree sur index commun, en pyramide de couches ;
chat de l’IDE disposant d’un parcours complet de travail de code. Répartition, critères de preuve
et convention `cx-` pour branches/worktrees consignés dans `05-cap-produit-vues-et-chat.md`.
La convention s’applique aussi aux reprises de lots existants, au prochain point sûr si une passe
est active ; aucune suppression des anciennes branches distantes.


### Contrat des vues atterri, implémentation à suivre

- **599cf78b2bd07a60ce2b85b37ac7442b93f0704e**, docs seules, arbre principal : contrat commun
  ED/recherche dans `docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md` et correction
  de la conception historique. Hash et diff relus ; la session a confirmé les trois contrôles.
- Contrat : contenus partagés sans fusionner occurrences/arêtes, empreinte de l’entrée réellement
  découpée et de ses versions de traitement, vecteurs par modèle, parents et générations épinglés,
  tombstones, masque obligatoire avant top-k de chaque signal, visibilité des deux bouts des arêtes,
  lectures de branches sans worktree et migration prouvée sur copies.
- Découpage : garde-fou, contenu partagé sur ancre (1a), migration (1b), couches/résolution (2a),
  filtrage/outils (2b), ouverture des worktrees sans copie (3a), fusion/rejet/rétention (3b).
  L’orchestration a donné suite à 1a avec témoins rouges et coordination ED/codeparsers sur les
  fichiers communs ; aucun tour de push code et aucune activation sur les bases vivantes.
- Les mesures M3–M5 restent différées jusqu’à un créneau libre et aux vérifications de format
  sur copies ; leur absence ne bloque pas les témoins fonctionnels des invariants de 1a.
- **17 h 46, lucivy 4.3.3 publiée** sur le mot de Lucie dans la session lucivy (tag v4.3.3 sur main 663975a, CI
  et workflow de publication verts, vérifiée sur crates.io, npm dont lucivy-wasm, PyPI) ; fusionnées : #22,
  #23, #24 (tickets), #25 (le numéro). Le correctif : un close() en échec arrête quand même ses pools et ses
  acteurs (lié au SIGSEGV de sortie des e2e). **À faire par Codex (session recherche)** : monter
  `lucivy-core` de 4.3.2 à 4.3.3 dans `extension/rag3weaver/Cargo.toml` et Cargo.lock, passe entière, push.


### Premier atterrissage de code après reprise : garde-fou des scopes déplacés

- **1904e04e42cd3666d6b3a64fee5bd51f0629c020**, arbre principal : témoin du scope seulement
  déplacé et aide d’embarqueur injectable, sous `mod tests` de backend.rs uniquement ; aucun
  comportement produit modifié. Tour donné après lecture du diff et des preuves par orchestration.
- Preuves luciepc `~/.cache/rag3weaver-build/deplace-codex/` : rouge de sensibilité puis vert
  après restauration ; features5/5 ; lib2095 réussis/0 échec/58 ignorés ; navigateur33/0/0 ;
  e2e_code27/0/0 et e2e_code_sync17/0/0, deux suites sur deux, verdicts VERT relus.
- Push annoncé sous poste push et garde VERT, branche sauvegardée
  `cx-arbre-deplace-sans-reembarquer-4`. Trois contrôles confirmés : status propre, aucun rebase,
  origin/master égal à HEAD après fetch. Hash/diff relus et avance rapide locale effectuée.
- Worktree/branche du garde-fou au portable supprimés par le script ; nettoyage distant et
  target inactif encore en cours au premier retour. Confirmation finale demandée, preuves1a
  et worktree `cx-wt-scope-version` à préserver. Ne pas compter le nettoyage comme fini avant retour.
- Du Rust a donc avancé sur master : ED, recherche, archi et mémoire prévenus de rebaser et rejouer
  court selon procédure avant leur push. Aucun autre tour simultané n’est accordé.
- Lot1a : témoins dd5f30465 joués, deux tests dont un rouge pour la bonne raison : même contenu
  embarqué deux fois au lieu d’une, occurrences et positions distinctes vérifiées ; docstrings
  différentes : témoin vert. Preuve `~/.cache/rag3weaver-build/cx-scope-version-rouge.log`.

- **Retour suivant : nettoyage terminé**, confirmé par arbre principal : wt-deplace et branche
  locale au portable, wt-deplace-codex détaché à luciepc et target-A-plan inactif supprimés ;
  refs distantes intactes. Worktrees1a conservés aux deux postes, preuves rouges sauvegardées
  hors target supprimé ; prochain target unique1a : cx-target-scope-version.


### Reprise1a et précisions du contrat

- **ec75832167f086e3f490423b7bb0d9ea36720c4b**, docs seules, arbre principal : rapport,
  knowledge dump et contrat actualisés ; trois contrôles confirmés, hash/diff relus ici.
  Le contrat précise que le masque de visibilité reste obligatoire sans modifier la précédence
  existante des filtres domaine/utilisateur, et que les octets sources restent conservés avant
  remappage des surlignages. Worktree documentaire et branche locale retirés, aucun target.
- Reprise1a : worktrees cx-wt-scope-version et rouge dd5f30465 conservés ; validation à reprendre
  sur le master portant le garde-fou, target unique neuf cx-target-scope-version.


### Lucivy4.3.3 publiée, intégration rag3weaver à suivre

- Lucie confirme publication4.3.3 sur crates.io/npm/PyPI, PR22–25 fusionnées et CI verte sur
  main663975a avant tag v4.3.3, workflow de publication entièrement vert. Correctif : close()
  arrête pools/acteurs même en erreur ; tests de corpus désormais rouges si corpus absent.
  Aucun changement de format d’index, démo conservée en4.3.2. Publication non rejouée ici.
- **bf3a0bfb6**, docs seules, session lucivy : rapport/dump actualisés, attentes des fusions
  devenues obsolètes retirées. Elle a relu localement tag663975adf11fc4a9d6cadf90e70ee68f6c9db075,
  CHANGELOG, note e805235 et présence du témoin close ; pas de test ni de registre revérifié.
  Trois contrôles postpush confirmés ; hash/diff relus par orchestration.
- Recherche prend un lot distinct cx-recherche-lucivy-4-3-3 après son lot d’arrêt : minimums
  lucivy-core/ld-lucivy/sparse-vector et lock ciblés, luciole/lucistore alignés, source registre.
  Lecture locale actuelle : les cinq crates sont encore verrouillées en4.3.2.
- **Périmètre corrigé par Lucie** : aucun scénario ou fixture supplémentaire de compatibilité
  d’index4.3.2→4.3.3, aucune migration ni banc dédié. Preuve ciblée sur fermeture échouée,
  arrêt effectif des pools/acteurs et terminaison propre ; passe habituelle maintenue. Recherche
  a confirmé qu’aucun code de scénario de compatibilité n’avait été ajouté.

- **21250c51b**, docs seules, lucivy : rapport/dump corrigés pour retirer tout scénario
  supplémentaire de compatibilité d’index ; preuve limitée à la fermeture en erreur et à la passe
  habituelle. Diff relu ici ; la session confirme status propre et origin/master égal à HEAD.

### Rappel de publication des textes

Lucie confirme : aucune attribution à un outil d’assistance ou à un modèle dans les messages
poussés, PR et notes ; une mention reste pertinente si elle décrit réellement la fonctionnalité
concernée. Identité git Lucie gmail et préfixes de branches/worktrees cx- inchangés. Rappel transmis
aux treize sessions, sans réécriture d’historique publié.
