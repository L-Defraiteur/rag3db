# Relecture totale du pilote global et D6 — préparation

Référence exacte **dc868252af9015e6069ef9cb872b41eb80da22c5**, parent33c58c21cd252ec3f226988a1e45fe231b2e89b1 ; APP **f1e8a8d502614d10f7cdb40eaea330f08b5d3f37**. **HOLD préparation native** : réserves ci-dessous à traiter ensemble. Cette revue ne donne aucun GO CPU, Cargo, modèle, transition Git PC ou service. Le [ticket porteur ouvert](../../rag3weaver-archi/tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md) conserve le suivi.

## Vérifications effectuées

Lecture des17 modules Python, du rapport33, des contrats et témoins, et confrontation avec les sources APP (Sandbox, garde commune, fixtures Rust, routes IDE) et la passe épinglée. Analyse AST seulement :17 fichiers syntaxiquement acceptés,31 fonctions de test présentes. Aucun import/exécution de ces modules, aucun test rejoué, aucune inspection de processus, lecture de corpus, DB ou secret. Le diff APP→pilote hors docs est vide.

Les dispositions utiles sont bien matérialisées : entrée --launch à configuration constante avant timeout7440+15 ; mesure exclusive, quota800%, jobs8 sous POSTE_FILS>=8 ; Common<=7200 avec échéances de phase intersectées à END−15 et phases non réinitialisables ; somme des caps7170. Le ledger5ELF et les publications task sont placés avant les autorisations de runtime ; copies/SHA et deux postes sont exigés. Le broker compile en no-run puis autorise le même ELF ; il ne joue pas le test sur l'hôte. Le positif742/01c3/72preuves reste séparé. Le lecteur canonique utilise des inventaires antérieurs, distingue enfants et juges, rattache résumés/F8 aux blocs. Le polling ne prétend pas capturer tous les exec. Ces points sont des constats de code, pas des preuves d'exécution.

## Réserves regroupées

### R1 — garde commune illisible depuis le Sandbox réel

`broker.py`, réponse admit : guard vaut `C/scripts/garde_target.sh`, donc le chemin de la source principale PC, extérieur à la tâche privée. `cargo_shim.py:21–25` le source dans le processus déjà sous Landlock avant le vrai Cargo. `policy.sandbox` ajoute seulement Q, le socket_root et le SDK aux lectures initiales Rust/Cargo. Aucun de ces chemins ne couvre C. La politique réelle `BacASable::autour` n'ajoute que système et domaine ; le domaine de la tâche sous F ne couvre pas C non plus. La lecture échoue donc avant compilation. Ce n'est pas corrigé par la capacité de lecture de l'observateur, qui est un autre processus.

Attendu : rendre explicitement lisible la garde réellement employée, de préférence copie privée épinglée et ses dépendances nécessaires, AVANT commit du manifeste et création de tâche. Garder la vraie garde et l'exécution Cargo/ELF sous Sandbox ; ne pas élargir globalement les droits. Témoin de raccordement sur les chemins effectifs et droits déclarés, puis preuve native sous GO distinct.

### R2 — témoin CPU copié avec une dépendance absente

`test_cargo_stop.py:82–93` lit `HERE/driver.py` puis recherche SDK_PROFILE, RUST_LIB et METADATA_BEFORE_EXPORT. Aucun driver.py n'existe dans global-pass ; l'entrée réelle est qualify.py et ces déclarations historiques ne sont plus le contrat du nouveau pilote. Cette fonction atteindra FileNotFoundError ; le comptage31 ne qualifie donc pas le paquet CPU. Constat statique, pas un rouge observé.

Attendu : raccorder le témoin au vrai constructeur de profil du nouveau pilote, conserver un test de présence/absence pertinent et publier le correctif avant le GO CPU. Ne pas simplement fournir un vieux driver inutilisé pour satisfaire le test.

### R3 — le rouge D6 n'établit pas sa cause

`policy.judged` exige un nom, un résumé0/1/0 et la sous-chaîne `panicked at`. `broker.py` ajoute rc101. Cela accepte aussi le même test nommé contenant `panic!("autre cause")`, avec le résumé Rust correspondant. Aucune vérification du corps de test, de l'assertion/position ou du diagnostic ne lie ce rouge au refus de « Om bre » et à agent.name. Le prompt le demande mais n'est pas une preuve. Le test de revue indépendant ultérieur peut être vert après correction sans établir la cause du premier rouge.

Attendu : conserver l'état source du premier test et lier sa trace à l'assertion précise du contrat, refuser un panic étranger portant le bon nom, une assertion étrangère et un simple échec de compilation. Publier ces témoins avant exécution ; le rouge d'infrastructure reste STOP.

### R4 — provenance de controllers.py périmée

Dans provenance.json, destination_sha256 vaut731e219bc889853c8d6694c2a895e4b89f57ccaad5e0ab4241dfdeb829b8ef17 ; le blob exact dc868252 vaut **f763f5a9cf8800576f910d498c2d905e0ffdc50b2f9dddf3d90f67d9e2cfe916**. Les cinq autres destinations déclarées concordent. Le changement blanc final est sans effet logique, mais le manifeste de provenance doit refléter le pin demandé. La configuration future réhashe les copies : cette erreur ne prouve donc pas une substitution runtime.

## Limites et suite

Le mode documentaire du broker accepte uniquement la fixture sommes et quatre argv déclarés. Le test Rust distinct `a_task_plays_cargo_test_through_the_gate` emploie petit-depot et `cargo test --lib -- un_test` ; son setup ne reçoit pas le bridge documentaire. Il doit continuer à appeler le vrai Cargo sans shim : le rôle documentaire ne le couvre pas et ne doit pas être présenté comme couvrant toutes les commandes de la lib.

Le verdict final est réévalué après open/fsync dans evidence.finish ; la borne physique englobe la sortie. Aucun elapsed exact de future exécution n'est déduit des majorants. L'acquisition du poste, les copies PC, les profils réels, la publication2postes, D6 et le canonique restent tous non joués. Les31 fonctions CPU restent NONJOUÉES ici ; les réserves sont établies par lecture, pas par un lancement clandestin. Revoir le correctif groupé et ses témoins ; C7 arbitre ensuite les allocations distinctes. Le ticket reste ouvert, aucune qualification globale, intégration/master ou retrait des anciens artefacts.
