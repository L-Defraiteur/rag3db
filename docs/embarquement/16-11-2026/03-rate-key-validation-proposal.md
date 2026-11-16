# Qualification proposée après bascule — rate-key

Ticket : [clé du débit](../tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md).
**Proposition seulement, aucune admission ni exécution par cette page.**
Recherche et Archi gardent la priorité sur les deux créneaux Rust de luciepc.
Attendre l'accord de l'orchestration sur un créneau et ce budget neuf.

## Source et artefacts

- Base master `68590270e21d00285b29800bd9cdecf0cb921461`.
- WIP reposé sur branche neuve `cx-rate-key-20261116-r2`, première tête
  `e2c84b5ed192851fc7aabd75c514379cbbe03327`.
- Diff des sources rag3weaver entre r1 et cette tête : vide. Seul le pin de la
  passe et ses documents amont sont apportés ; aucune qualification supposée.
- Script officiel `tools/poste/passe_epinglee.sh` et sa copie PC vérifiée :
  SHA256 `2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d`.
- SDK exact :
  `rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
- **Sans `RAG3WEAVER_MOTEUR_ANCIEN`**. Aucun pointeur `courant`, cache ancien
  `lib_seule`, pilote du 13 ou ancien verdict utilisé comme autorité.

## Portée et limites proposées

Une seule portée commune de **5400 secondes**, créées après acquisition réelle
`poste lourd`, pour les ciblés et toute la passe, sans reset entre phases.
Enveloppe32G, memory.high sans limite intermédiaire, j8, bus privé,
descendants nettoyés par `scripts/confiner_validation.py`. Arrêt premier rouge.
Ce nombre est un budget proposé, pas un GO de durée déjà accordé.

Réutiliser le **seul target de rôle** `~/.cache/rag3weaver-build/target-activation`
à luciepc :16752988160 octets au relevé RO, marque de l'ancien W
`rag3db-rate-key/extension/rag3weaver`. Avant reprise : quiescence et garde
propriétaire ; transfert par `RAG3WEAVER_TARGET_REPRENDRE=1` dans la portée,
plafond32G maintenu. Pas de nettoyage opportuniste ni second target.
L'ancien WIP rate-key et son journal rouge restent conservés.

Journaux neufs hors target sous `~/.cache/rag3weaver-build/cx-rate-key-59ac-v1/`,
recette/source/SHA de scripts/verdict de la portée conservés ; aucune écriture
des jetons d'autorité dans les sorties ou les docs.

## Étapes proposées

1. Contrôler source propre, base et absence de rebase ; vérifier SDK et hash
   de la copie de passe. Refuser changement de code ou pin non audité.
2. Compiler une seule fois la lib de tests sous le jeu large de la passe :
   `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`.
   Inventorier puis jouer les neuf ciblés exacts ci-dessous. Aucun service posé
   ni modèle réel pour ces tests : faux embarqueurs et fixture HTTP Python.
3. Passe officielle, même portée et restant de budget, `CIBLE=target-activation`,
   `SUITE_DU_TEMOIN=lib`, filtre
   `the_measured_rate_is_noted_under_the_called_address_and_host`,
   `SUITES=e2e_code_sync,e2e_lectures,e2e_embarqueur_a_la_demande`.
   Le script joue aussi F5, la preuve de compilation, lib entière et NAV.
   Son appel prend les noms `cx-rate-key` et `cx-rate-key-59ac-v1`, pas des
   chemins absolus pour le dossier des journaux.
4. Lire les comptes de chaque suite et l'absence de sauté/non-joué ; première
   ligne de resume.log = tête qualifiée, dernière = VERT, master testé connu.
   Vérifier fin du bus, descendants, target sous32G, restitution des ressources.

Ciblés exacts (les noms de fonctions sont préfixés par leur module) :

- `catalog::tests::the_measured_rate_is_noted_under_the_called_address_and_host`
- `catalog::tests::an_in_process_embedder_is_noted_as_local_at_its_host`
- `catalog::tests::the_known_rate_falls_back_to_the_old_key_then_prefers_the_new_one`
- `catalog::tests::an_unknown_remote_rate_address_never_becomes_local_or_calls_the_provider`
- `catalog::tests::rate_keys_keep_addresses_and_client_hosts_isolated`
- `catalog::tests::the_known_rate_tracks_the_taken_address_without_provider_calls`
- `catalog::tests::rate_place_access_requires_an_address_and_host_without_legacy_writes`
- `catalog::tests::a_shared_embedder_forwards_its_called_address`
- `model_source::activation::tests::reading_the_rate_address_never_starts_an_on_demand_provider`

Le filtre de la passe officielle rejoue son témoin requis après ces ciblés ;
aucun compteur n'additionne cette invocation à neuf tests distincts.
Les trois rouges historiques sont ceux du journal `rate-key-red.out` et du
WIP0ae ; les nouveaux ciblés et le correctif ne sont encore ni compilés ni joués.

## Coordination et risques

La phase ciblée est hermétique. La passe officielle utilise ses services
configurés et peut exercer le repli de tests iGPU prévu par le script : ce n'est
pas une promesse de passe entière sans GPU. Coordonner les usages des services
avant cette phase avec l'orchestration ; aucun nouveau fournisseur réel, pose,
arrêt ou changement des bases de Lucie n'est autorisé par cette proposition.
`e2e_embarqueur_a_la_demande` est lui-même sur fixture CPU, aucun modèle.
`e2e_code_sync` est exigé par `required_suites.txt` car catalog* est touché.

Ne pas élargir à la mesure par passe, à la table complète des débits ou au seuil
six secondes. Toute erreur nouvelle est tracée avant correction/reprise. Un
code différent de la source qualifiée doit être relu et rejoué selon la garde.
Aucun push master sans nouveau « à toi » après qualification.


## Lanceur préparé, pas exécuté

Les sources du lanceur sont `rate-key/run-qualification.sh` et
`rate-key/qualify.py`. La commande suivante est une recette future : aucun
appel tant que le créneau Rust et les usages de la passe ne sont pas accordés.
Sur le W synchronisé à luciepc, remplacer SOURCE par la tête complète revue :

```bash
bash docs/embarquement/16-11-2026/rate-key/run-qualification.sh SOURCE
```

Le lanceur acquiert lui-même `poste lourd`, puis crée la portée5400 s. Le
pilote vérifie les hashes source/cache du pin, la source propre, le target et
ses références lisibles, compile un seul ELF de tests, inventorie et joue
chacun des neuf noms exacts avec compte1/0/0, puis appelle la passe officielle
sans réinitialiser le délai. `result.json` ne dit VERT qu'après la passe entière.
Les descripteurs illisibles restent nommés inconnus ; aucune suppression du
cache, aucun signal envoyé à un processus, aucune ouverture des bases de Lucie.
Seuls AST Python, syntaxe shell et diff --check ont été contrôlés à ce stade.


## Magasin explicite et XDG privé : précontrôle sans Cargo

Le profil `rate-key/sdk-profile-luciepc.json` épingle le magasin réel absolu,
le SDK59ac et les chemins typés/SHA256 de `librag3db.so`, de l'extension vectorielle
et de l'en-tête BUILD_ID. Le pilote vérifie ces trois artefacts avant tout Cargo,
et exporte explicitement le magasin et les dossiers de lib/en-têtes. Il ne copie
ni SDK ni cache partagé dans le dossier privé. Un changement de fichier arrête.

Précontrôle effectué à luciepc sur le résolveur existant, XDG_CACHE_HOME privé :
- sans RAG3DB_LIBS_DIR : refus attendu/code2, avant tout Cargo ;
- avec le magasin explicite : résolution du SDK exact, paths lib/include attendus ;
- empreintes relevées et conservées, aucun appel natif ou test produit.

Preuves hors target :
`~/.cache/embarquement-notes/rate-key-16nov/xdg-resolver-preflight-v1/`
(`negative.log`, `positive.log`, `result.json`). C'est un contrôle de configuration,
pas une compilation ni une qualification de rate-key. Le profil précédent
épinglait déjà RAG3DB_LIBS_DIR : aucun défaut produit nouveau n'est inventé.


## Preuve de portée et causes avant assertions

Le lanceur ne consulte ni systemctl --user ni l'API7970 : aucun hôte réel n'est
un prérequis de ses fixtures. Il conserve les chemins du reçu courant et du
cgroup, memory.max/high observés dans /sys/fs/cgroup, puis appelle le vérificateur
commun. Aucune réinjection du bus personnel de Lucie dans la portée privée.

Chaque phase conserve avant son assertion : commande exacte sans environnement
secret, code de retour, stdout et stderr séparés, durée et état. Le démarrage
est inscrit avant le sous-processus ; une interruption note sa cause sans
inventer de code retourné. Les anciens logs ne sont ni écrasés ni requalifiés.
Ce delta ne modifie aucune qualification active et ne lance aucun calcul.


## Base ED atterrie et point de coordination non levé

ED confirme masterd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6, trois contrôles
lus. Le lot a été reposé sur cette base dans `cx-rate-key-20261116-r3`, sans
conflit ; aucun des cinq fichiers réservés n'a changé dans l'amont. La vraie
union apporte backend.rs, generic_search_nodes.rs et search.rs : elle doit être
qualifiée, les anciens verts ne suffisent pas. Les anciennes branches restent.

Le lanceur ne force aucun quota CPU : poste garde sa borne. Les jobs des ciblés
respectent min(8, POSTE_FILS), la preuve conserve cpu.max et POSTE_FILS. La passe
officielle conserve son j8 et reste sous le quota du poste. Pas de CPUQuota8%.

**Point non levé avant GO exact** : la consigne actuelle interdit les services
partagés. Or la passe officielle SHA2150 configure7878/7879 et e2e_lectures a
un témoin BGE réel (surlignage creux via l'hôte). Cette recette complète n'est
DONC PAS hermétique. Les neuf ciblés et e2e_code_sync (HashEmbedder) ainsi que
e2e_embarqueur_a_la_demande (fixturePython) sont privés. Aucun de ces témoins ne
remplace les lectures réelles ou la lib/NAV, et aucun filtre ne doit faire dire
VERT à une suite amputée. L'orchestration doit cadrer une phase privée ou un
complément coordonné ; le lanceur complet reste inactif jusque-là.
