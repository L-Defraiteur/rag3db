# Le témoin de fermeture FTS échoue avant l’appel à close

- **État** : corrigé ; code5a83eee91 validé sur5dddf215c, verdict cb3bb35cc relu et tour master explicitement accordé le12novembre2026 ; préfixe et configurations5/5 verts.
- **Famille** : produit
- **Gravité** : blocage de la validation CI ; le témoin peut échouer avant de juger le contrat de fermeture.
- **Atteignable en service** : non pour ce panic de préparation du test ; aucune régression produit de close établie.
- **Touche rag3weaver** : oui, src/fts_handle.rs::repro_drop et validation CI.
- **Pour** : recherche ; coordination lucivy si un défaut amont distinct est réellement reproduit.
- **Signalé par** : orchestration, contrôle périodique de la CI le12novembre2026.

## Preuve exacte

Commit08086ba3600a73f4828e541b093cb8751a1984fb ;
[run rag3weaver43187074046](https://github.com/L-Defraiteur/rag3db/actions/runs/43187074046),
[job131076793291](https://github.com/L-Defraiteur/rag3db/actions/runs/43187074046/job/131076793291).
Étape cargo test --lib sous rag3db-native,code,daemon,openai-llm.
Résultat2089 réussis,1 échec,64 ignorés,2 filtrés ;117,96s.

Le seul échec est
`fts_handle::repro_drop::une_fermeture_fts_en_erreur_rend_le_handle_inerte_et_le_processus_sort`.
L’enfant panique à fts_handle.rs652 sur remove_dir_all(index).unwrap :
`Os { code: 39, kind: DirectoryNotEmpty, message: "Directory not empty" }`.
Le log signale aussi un fold de dictionnaire en arrière-plan échoué à ouvrir
`index/shard_0/dict-1.1.sfx.fst.tmp`. Le parent rapporte ensuite l’échec de l’enfant.
Le close que ce témoin doit mettre en défaut est APRES la suppression qui a paniqué.
Aucun résultat sur son inactivité après close n’est établi par ce run rouge.

## Diagnostic actuel et limites

La suppression récursive est déclenchée alors que le handle et ses acteurs sont vivants.
Une création de fichier concurrente est une cause plausible d’ENOTEMPTY, à reproduire ;
ne pas remplacer la preuve par une supposition sur le runner. Le fichier fts_handle.rs
est strictement inchangé entre f96acc6cf (mise à jour4.3.3) et08086ba36 : aucune causalité
avec le partage de contenu1a n’est démontrée. La passe locale831d reste verte mais ne
remplace pas cette CI. L’ancien ticket du vrai close en erreur reste clos ; ce ticket
ne rouvre pas ce correctif amont sans reproduction distincte.

## Travail attendu

Reprendre le montage sur un worktree privé, conserver le log rouge et reproduire le
point de préparation défaillant. Rendre l’injection d’erreur de persistance déterministe
par une primitive publique ou une fixture maîtrisée, sans dépendre d’une suppression
concurrente non synchronisée, sans sommeil/réessai aveugle et sans modifier la bibliothèque
si aucun défaut produit n’est établi. Le même contrat enfant reste vérifié : close
échoue réellement, le handle refuse ensuite l’indexation et Drop/processus se terminent.
Les tests n’ouvrent que leurs ressources privées ; aucun service, modèle ni index réel.

## Critères de fin

Témoin fiable qui atteint l’erreur close voulue, garde les assertions d’inactivité et de
sortie bornée, avec rouge/vert attribués aux bonnes sources ; contrôles adaptés au jeu CI
puis passe requise, revue et intégration. Ni ignore/exemption, ni suppression d’assertion,
ni rerun de CI pour tenter la chance. La CI future se lit sur son SHA exact ou la première
exécution terminée plus récente selon la procédure.

## Diagnostic borné de recherche

Worktree neuf `cx-recherche-fts-close-deterministe`, branche homonyme, base exacte898c9e0dc. Extrait CI relu sur le portable (pas sur luciepc), SHA256 `408e11c4d50197c700e8ebb7c50112405ad421bc1c5174444b6ab0b79fe3d663`, copie conservée hors worktree dans `~/.cache/rag3weaver-build/journaux-cx-recherche-fts-close-12nov/rouge-ci-080.log`. Le panic à la suppression et le message de fold sont bien ceux consignés ci-dessus ; close n'a pas été atteint dans cet enfant. Aucun rejeu de ce run ni boucle visant à retomber sur ENOTEMPTY n'a été lancé.

Comparaison f96→080 : fts_handle.rs identique ; comparaison080→898 : pas de diff source Rust/workflow. La concurrence du fold est cohérente avec le montage qui détruit le dossier de writers vivants, mais l'interleaving exact n'est pas reproduit localement. Ce constat ne démontre pas un défaut produit de Lucivy ni une causalité1a.

## Témoin proposé avant exécution

L'API publique du paquet registry lucivy-core4.3.3, relue à luciepc (`sharded_handle.rs`), expose ShardStorage, RamShardStorage et ShardedHandle::create_with_storage. Une fixture locale au test enveloppe RamShardStorage : création/ouverture/lectures déléguées, seules écritures de racine refusées une fois un drapeau atomique explicitement armé. Les shards et leurs folds restent sur un vrai stockage RAM privé ; aucune suppression/permission/renommage du filesystem sous les acteurs, aucun nom interne de fichier à connaître.

Création, indexation et commit doivent réussir avant armement, compteur d'erreurs injectées encore nul. Après armement, close doit retourner l'erreur distinctive de la fixture et le compteur doit être positif : la preuve établit que close a réellement atteint le refus de persistance voulu, au lieu de réussir ou d'échouer dans la préparation. Pas d'hypothèse sur le nombre interne d'écritures. Ensuite les assertions existantes sont conservées : index_document refuse avec « closed », Drop du handle, marque de sortie, enfant terminé avec succès sous la borne existante20s. Le parent n'a plus besoin d'un TempDir de stockage ; protocole enfant et nom du test inchangés.

La fixture compose la frontière publique de stockage ; elle n'ajoute aucun mécanisme runtime, patch de dépendance, format, ignore ou exemption. Le précédent ticket du vrai close4.3.3 reste fermé. Aucun service/modèle/DB/index utilisateur touché.

**Joué à cette étape :** relecture du log et des APIs, git diff --check, parsing Rust avec rustfmt --emit stdout vers une copie hors dépôt (sans reformater le fichier source). **Non joué :** compilation Rust, exécution ciblée/FTS/lib, matrice ou CI. Aucune réussite native du nouveau montage n'est encore revendiquée.

## Plan fermé soumis avant tout build lourd

Aucune réservation CPU pour cette préparation. Arbre1b et coeur C++ ont signalé leur coordination ; l'envoi intersessions reste indisponible dans l'inventaire de recherche, donc aucune réponse envoyée ne leur est revendiquée. L'orchestration arbitre le créneau après revue.

Proposition de premier préfixe, à autoriser : luciepc, poste lourd, -j8, mémoire32G, target unique recherche `~/.cache/rag3weaver-build/target-97k` recréé/guardé et plafonné32Go, lib complète29a7bc52a épinglée, pas de MOTEUR_ANCIEN. Pas de modèle, GPU, service ou DB : jeu exact CI `rag3db-native,code,daemon,openai-llm`, sans burn-embedder/burn-ocr ni postgres.

1. Un appel Cargo --locked --lib sous ce jeu avec le nom exact du témoin et --exact --nocapture, compilation incluse, borne1500s. Attendus : un test parent joué, enfant terminé normalement, préparation avant armement, erreur racine distinctive, inertie et Drop réellement observés. Pas de stress/retry pour tenter la chance.
2. Si vert seulement, module existant fts_handle:: sous le même jeu, borne120s, avec recherche/indexation/réouverture et Drop privés déjà présents ; les noms et comptes seront lus dans le binaire.
3. Si vert seulement, lib correspondant exactement à la CI, borne300s, avec les DEUX filtres existants de ce workflow (aucun nouveau) : --skip the_extension_next_to_the_loaded_lib_is_loaded_without_any_variable et --skip the_test_helper_loads_the_extension_of_the_loaded_lib_not_of_the_tree. Comptes réels enregistrés, pas de promotion d'un zéro test ni reprise du compte2089 ancien.
4. Attester source/manifestes et binaire, lib native, sortie enfant et logs, borne120s ; global2400s, arrêt au premier rouge et conservation des preuves. Une seule compilation cible à la fois. Aucune matrice complète/e2e/banc ni relance GitHub CI dans ce préfixe.

Après ces preuves seulement : retour pour revue et définition de la passe finale requise avant intégration ; aucune autorisation de lancement, tour master ou nettoyage de ressources d'autres lots n'est déduite de ce plan. Ticket toujours ouvert, montages et preuves anciennes conservés.

## Transmission effective du plan

Après vérification du remote (HEAD5dddf215c6625743fb1e6b51b26b9f77d8016787), le transport disponible `multi_agent_v1.send_input` a accepté l'envoi à l'orchestration : soumission01a1cf2f-10c0-7033-9b71-6b7bbb6d0d94. Message : même ticket, code5a83eee91/base898, montage publicRAM, assertions conservées et recette/bornes ci-dessus. L'ancien namespace TUI reste absent, mais le livrable n'est plus seulement publié sans transmission. Aucune lecture/revue ni autorisation de build ne sont déduites de l'accusé technique d'envoi. Les sessions arbre1b et coeur C++ ont reçu confirmation qu'aucun créneau/build recherche n'est actif ; le créneau C++ libéré par C7 n'est pas concurrencé.

## Verdict du préfixe autorisé le12novembre

Après revue explicite du plan5dddf215c et du code5a83eee91, orchestration a autorisé le préfixe et ajouté check_features5 ; coeur C++ a confirmé son créneau terminé avant le lancement. Source propre figée5dddf215c6625743fb1e6b51b26b9f77d8016787, luciepc sous poste lourd, -j8/32G, target97k privé plafonné32Go, lib29 complète ci-dessus. Départ11:04:38 CET, ciblé terminé11:06:45 ; module terminé11:06:46 ; lib terminée11:07:22 ; matrice terminée avant11:11. Global2400s respecté, aucun rouge, aucun retry.

- Ciblé exact CI : **1 réussi,0 échec,0 ignoré**. Sortie enfant : préparation réussie avant armement ; `close="persistance racine refusée par le témoin de fermeture" index_apres=Err("handle is closed")` ; handle détruit et sortie enfant. Assertions du compteur exécutées, borne parent20s conservée.
- Module fts_handle : **10 réussis,0 échec,0 ignoré**, avec recherche, résolution des positions et réouverture/Drop existants.
- Lib sous rag3db-native,code,daemon,openai-llm : **2090 réussis,0 échec,64 ignorés,2 filtrés**, exclusivement les deux exclusions workflow documentées. Les comptes proviennent de cette exécution.
- Attestation : source/manifestes propres, identité native29 exacte, **220 gabarits présents**, SHA256 du binaire `76a78be049c85c1f07fd26d6c46c4a348812d037933ef9b3ca0d8c9788efc9b1`, inchangé après la matrice.
- `check_features.sh` : **CHECK_FEATURES VERT5/5**, y compris configurations Burn/OCR/Postgres en compilation seulement ; aucun service, GPU, modèle ni PG utilisé. Aucun navigateur/e2e supplémentaire ni relance CI.

Preuves brutes conservées sur les deux postes dans `~/.cache/rag3weaver-build/journaux-cx-recherche-fts-close-12nov/` : rouge-ci-080.log, source.json, cible.log, module-fts.log, lib-ci.log, attestation-binaire.json, identite-native.txt, check_features.log, features-detail/, resume.log et verdict-final.log. Le rouge CI ancien reste un échec de préparation ; ce vert prouve le nouveau montage, sans prétendre reproduire son interleaving filesystem ni diagnostiquer une régression produit amont.

Reste à orchestration : revue de ce verdict, tour master explicite puis intégration et lecture de la CI future automatique sur son SHA. Ticket maintenu ouvert jusque-là ; aucun push master ni rerun aveugle autorisé par ce résultat.

## Clôture après revue

Orchestration a relu les logs bruts, les assertions et l’attestation, confirmé que5dddf→cb3 ne change que deux Markdown et que master898 est inchangé, puis donné explicitement le tour master. Correction et validation acceptées ; ticket clôturé avec déplacement dans le même commit. Intégration sous poste push avec dernier fetch, arrêt si nouveau code, garde VERT et trois contrôles. Nettoyage immédiat limité aux ressources propres, avec archives administratives fidèles et garde65 fraîche ; remotes et preuves conservés. CI future automatique, aucun rerun ancien ni binaire de service à poser.
