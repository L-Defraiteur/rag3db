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


## Delta groupé4461c338 — avis favorable à la préparation

Pin exact **4461c338126e994e35518cc1fdbec3a86cfb2b56**, APPf1e8a8d502614d10f7cdb40eaea330f08b5d3f37 inchangée ; comparaison avec dc868252. Lecture de tous les deltas et de leurs témoins, confrontation Provider avec la vraie enum/branche Rust APP. Analyse AST indépendante : **18 modules/39 fonctions de test**, aucune fonction exécutée/importée. Six destinations de provenance réhashées depuis Git : toutes conformes. Diff produit hors docs vide. Aucun pin PC/Q, état de target, profil SDK réel, DB, processus ou service inspecté dans cette relecture.

Les réserves R1–R4 de ce rapport sont **traitées dans le code de préparation** :

- R1 : policy.sandbox ajoute précisément C/scripts/garde_target.sh, le chemin rendu par le vrai broker ; droits de lecture sur ce fichier, sans ouvrir son répertoire. Le témoin confronte l'expression guard réelle à la déclaration. Le script reste celui de l'APP épinglée et Cargo/ELF restent dans le Sandbox.
- R2 : la référence driver.py inexistante est retirée. Le témoin extrait par AST les méthodes réelles Observer.cargo_profile/runtime_ld et vérifie le profil global SDK hérité : sept champs présents, LD metadata non gardé limité à rustlib, LD gardé rustlib+SDK. Il ne transpose plus six absences d'un autre pilote ; l'observation SDK réelle reste non jouée.
- R3 : bloc de test exact D6_RED_TEST préannoncé dans la demande et vérifié dans backend.rs à l'admission rouge. Le numéro de ligne d'assertion est calculé depuis cette source, puis le juge exige le message D6_ASSERTION Om bre/agent.name et le panic à cette ligne de src/backend.rs, en plus du nom/compteurs/code. Le Counter de fonctions n'est pas une preuve que ces témoins passent. La revue Rust ultérieure reste distincte du test de l'agent.
- R4 : provenance controllers.py=f763f5a9cf8800576f910d498c2d905e0ffdc50b2f9dddf3d90f67d9e2cfe916 et test_cargo_stop=d706ba543cc8ad1ea29cf73c216190a63eb288a872262321aa97eb4d16413abe correspondent au nouveau pin ; les quatre autres destinations déclarées aussi.

Les autres corrections groupées sont raccordées : models.llm porte provider=compatible/protocol=vertex, confronté statiquement à Provider::Compatible et à la branche réelle model_source.rs ; **aucune désérialisation Rust exécutée**. source_state conserve HEAD/index et modes/SHA des fichiers suivis/non suivis, liens, HEAD/propreté des gitlinks ; égalité exigée après Cargo et après test. Pour D6, index modifié/non-suivis/delta hors backend.rs sont refusés ; l'état après vert est recontrôlé et sauvegardé avant injection de la revue indépendante. Il s'agit de comparaisons de l'état capturé, pas d'une primitive de snapshot transactionnel ou d'une nouvelle identité d'ingestion.

Observer.runtime_snapshot archive environ sélectionné/maps et identité avant/après (PID/start/UID/CG/exe/SHA/argv/cwd) avant de juger égalité et profil. Une lecture impossible reste un refus archivé, jamais une qualification. Le polling conserve sa limite annoncée ; on n'en fait pas une trace exhaustive des exec. preserve_failure appelle le latch avant cleanup, conserve l'exception primaire et lui ajoute les erreurs de fermeture ; host/TTY sont attendus/réapés dans MIN(ENDD6,now+15), broker socket/thread dans MIN(END,ENDphase,now+2). Un résidu/timeout reste un refus et relève ensuite du Common ; aucune capture SDK n'est ajoutée au nettoyage.

**Avis favorable au delta de préparation**, sans nouveau blocage identifié à cette lecture. Cela lève les quatre réserves source initiales de Banc, pas le ticket documentaire ni les exigences globales. Les39 fonctions sont ÉCRITES/NONJOUÉES : ni rouge dynamique ni CPU vert attesté par Banc. C7 arbitre CPU60 exact distinct, puis pins/copies/metadata et nouvelle revue avant tout GO7455 natif/modèle/service. Anciennes742/01c3/72preuves préservées, aucun master/intégration/nettoyage acquis.


## Delta résiduel60f9af5f — identité du juge vert encore à fermer

Pin exact **60f9af5f342d9a24adea8b718ad48d0d5e337f85** ; APPf1e8 inchangée. Lecture du seul delta Broker.handle et du témoin prépublié,18AST/40fonctions présentes **NONJOUÉES**, produit hors docs inchangé. Le refus à l'admission verte est désormais raccordé au vrai handle, avant création de session : modifier directement le texte .is_err() est bien refusé. built/tested recontrôlent le bloc et l'état source capturé. Les avis favorables4461 précédents restent à leur périmètre.

**Une réserve résiduelle, HOLD sur la garantie “même juge exécuté”.** policy.red_source vérifie seulement count(D6_RED_TEST)==1 puis sa ligne ; judge_sha256 est l'empreinte de cette constante. Cela ne vérifie pas que le bloc est actif dans la source Rust verte. Contre-exemple statique à placer dans le même module tests :

```rust
#[cfg(any())] // désactive le bloc conservé sans modifier ses octets
#[test]
fn agent_name_rejects_an_inner_space() {
    assert!(valider_le_nom_de_l_agent("Om bre").is_err(), "D6_ASSERTION: Om bre accepted; agent.name must reject interior whitespace");
}

#[test]
fn agent_name_rejects_an_inner_space() {}
```

La constante D6_RED_TEST apparaît exactement une fois ; son SHA est donc celui du rouge acquis. Le premier item est conditionnellement désactivé, le second garde le nom exact avec un corps vide. Les contrôles d'admission autorisent les différences dans backend.rs avant Cargo ; le snapshot complet fige ensuite cet état sans le comparer sémantiquement au juge rouge. Le filtre Rust exact peut ainsi obtenir1/0/0 sur l'autre item, alors que l'assertion originelle n'a pas été exécutée verte. Le validateur peut être corrigé en parallèle et passer la revue indépendante : cela ne reconstitue toujours pas le vert de ce juge initial. **Aucune compilation ni test de ce contre-exemple n'a été effectué ici** ; c'est le chemin non exclu par les conditions de code, pas un rouge dynamique observé.

Attendu pour fermer : lier la source verte à la source rouge acquise en n'autorisant que la modification du validateur admise, ou une autre preuve structurelle effective préservant item/attributs/contexte actif du test. Le simple hash de la sous-chaîne constante n'ajoute pas cette garantie. Publier un témoin du vrai Broker.handle où le bloc inchangé est désactivé et remplacé par un homonyme, à refuser AVANT session/Cargo, puis garder la relecture et le GO CPU C7 distincts. Aucun nouveau mécanisme produit/parsing imposé par cette revue. Ticket toujours ouvert, aucun CPU/native/pinPC/GO7455 déduit.


## Pin rouge isolé3a786a59 — RO du témoin ciblé

Pin **3a786a5933c924a46d99d8898652be9b85b3bb81**, branche cx-archi-global-cfg-red-r1 ; APPf1e8 inchangée. Avis favorable à la **préparation du seul témoin rouge** test_review_boundaries.Boundaries.test_actual_broker_rejects_cfg_disabled_judge_and_empty_homonym. Ce n'est pas un candidat natif.

Broker.py est byte-identique à60f9, SHA256 afacb193bc35e4b4fe655d0fcff5abddb61b239011c6d0da5508e073cb9dfef3. La fixture appelle son vrai handle avec identité/provenance/état source simulés ; elle fournit le judge_sha256 du rouge acquis simulé et un validator_frame disponible, mais ce Broker vulnérable ne vérifie pas le frame. Les chemins source/target sont des TemporaryDirectory privés, git et source_state sont remplacés, et le contexte/observer ne lance rien. Le helper validator_frame peut lire la petite fonction synthétique donnée : sa présence évite l'AttributeError de montage. Sur ce chemin, les conditions actuelles autorisent le bloc intact désactivé puis l'homonyme ; le handle peut retourner normalement après création de session. **Attendu statique : un FAIL “RuntimeError not raised”**, non un ERROR de fixture. Aucun résultat dynamique n'est revendiqué. AST de broker/policy/témoin relus ; aucune importation ou exécution. Le candidat structurel demeure séparé et non jugé par cet avis.

La commande proposée cible une seule méthode, pas toute la classe. Sa protection timeout TERM60+kill5 a une borne dure65s : C7 doit retenir cette enveloppe exacte, ou TERM55+kill5 pour un plafond physique60. Cela n'autorise aucune invocation ici. Log neuf horsW et premier résultat à conserver ; une erreur de harnais ne prouverait pas le défaut. Aucun CPU/proc/Git/Cargo/SDK/modèle/service/DB/target exécuté ou ouvert par cette revue. GO CPU rouge exact distinct encore attendu, aucune pin PC ou autorité native ne doit consommer cette branche vulnérable.


## Rouge acquis3a786 / correctif structurel a797 — preuves et RO séparées

Publication du rouge **d53d98695d264a5d7ac4216e70d33995b05e563c**, source exécutée3a786a5933c924a46d99d8898652be9b85b3bb81 sous GO C7-ARCHI-CFG-RED-3A786-17nov60. Rehash indépendant PC et local R/cx-archi-cfg-red-3a786-17nov-r1 : manifeste350ddb1e7bb69219fdb8148b83fccca3318c4caec153a1e29181eb185e5da0ed,3/3 fichiers conformes sur chaque poste. Les8SHA source.json correspondent aussi aux blobs du pin rouge. tests.log=2d07b04b6e76d1074ff4e0efef598a25e0d91b8118456980486f875be69ce292 : UNE méthode,1FAIL/0ERROR, RuntimeError not raised dans l'appel réel handle. result.json rc1/une invocation/sans retry,0.183236793s mesurées ; unittest0.002s. SourceJSON archive timeout TERM58+kill2, donc physiqueMAX60 : la question60+5 de la commande candidate a été réglée pour cette exécution. Aucun rejeu par Banc.

Chronologie conservée : témoin prépublié, correctif structurel DÉJÀ publié avant ce rouge isolé. Ce rouge observé démontre le contournement du Broker vulnérable ; il n'est pas présenté comme une observation historique avant publication du fix. Les deux copies archivées constituent les preuves relues ; aucun nouveau scan de processus/service n'est effectué.

Correctif distinct relu : **a79709034a694449ebeebf46df64daef0ca2b18d**, APPf1e8 inchangée.18AST/41fonctions écrites/non exécutées ; diff produit hors docs vide. Le Broker enregistre validator_frame au rouge et exige au vert l'égalité de prefix_sha256/suffix_sha256 au rouge réellement dans completed[0]. Ce cadre inclut signature et accolades externes, tous les octets avant/après le corps : attribut cfg ajouté devant le juge et homonyme ajouté après modifient le suffixe, donc sont refusés avant création de session. La normalisation ne tolère que le SHA contenu de src/backend.rs ; HEAD/index/modes/chemins/autres fichiers/gitlinks restent égaux à l'état rouge acquis. built/tested conservent les recontrôles complets de l'état de l'admission.

Le helper validator_frame est une grammaire bornée, pas un parseur Rust général : signature unique exacte, accolades équilibrées, littéraux échappés et commentaires ligne ; rawstrings/commentaires de bloc refusés. Le prompt déclare ces restrictions et exige de ne modifier que le corps du validateur. Le corps APP actuel relu appartient à cette grammaire ; préfixe/suffixe restent inchangés lorsqu'on modifie seulement ce corps. Aucun test ou rustc joué pour vérifier cette dernière possibilité : c'est une lecture du mécanisme. Ne pas étendre cette garantie à du Rust arbitraire sans revoir ce contrat.

**Avis favorable à la préparation du correctif a797** : réserve cfg/homonyme fermée dans le mécanisme relu, sans vert CPU acquis. Le témoin ciblé et la suite41 restent NONJOUÉS sur ce pin vert ; C7 doit encore attribuer son GO CPU exact distinct, borné, puis la suite copies/RO/natif reste séparée. Le pin rouge3a786 est exclu des recettes natives. Aucun Cargo/SDK/proc/target/modèle/service/cloud/DB lancé ou ouvert dans cette revue ; ticket OUVERT, aucun master/GO7455 déduit.


## Harnais a0d74de5 après ERROR3839 — RO courte

Pin exact **a0d74de57fe8bec364372264e4520b0b736a28a8**. Le diff du commit modifie une seule ligne du témoin Provider : settings.C (futur chemin PC) devient HERE.parents[3]/extension/rag3weaver, racine du porteur Git réel. Le calcul des parents a été relu depuis global-pass ; le blob src/model_source.rs de ce porteur est identique à l'APPf1e8. La confrontation reste statique enum/branche Compatible/vertex, pas une désérialisation Rust. AST du témoin accepté ; aucune exécution. Le code du pilote/broker structurel n'est pas changé par ce commit.

Preuves de la tentative précédente relues indépendamment PC et local R/cx-archi-cpu-green-3839-17nov-r1 : manifeste **2cd32197e837834b48bcbdc52730c752709ac14404b3d0d2c29f5e712627abf1**,3/3 SHA conformes sur chacun des deux postes. Log :37 méthodes jouées,36ok/1ERROR/0FAIL, FileNotFoundError exact sur model_source.rs au chemin PC ; quatre méthodes non jouées par failfast. Les deux témoins du vrai Broker apparaissent ok AVANT cet ERROR. result.json rc1/green=false/une invocation/sans retry/0.166063224s ; unittest0.040s. Ce résultat n'est ni41verts ni un défaut runtime du pilote. GO précédent consommé, premier ERROR préservé.

**Avis favorable à la préparation du delta de harnais a0d74** et à sa présentation à C7 pour un nouveau GO CPU exact distinct sur les41 méthodes (18+16+7), commande unique TERM58+KILL2/python3-B/failfast et les trois modules déclarés. Aucun essai sur ce nouveau pin effectué par Banc ; aucun GO donné ici. Pas de native/SDK/target/service/Vertex/DB/proc ni pin PC ouvert ou modifié. L'historique du rouge cfg et du correctif publié avant rouge est conservé, ticket OUVERT.
