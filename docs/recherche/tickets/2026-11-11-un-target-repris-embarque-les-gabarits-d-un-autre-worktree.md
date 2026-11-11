# Un target repris embarque les gabarits d'un autre worktree

- **État** : ouvert (11 novembre 2026), défaut de provenance des artefacts constaté pendant R5 ; récupération ponctuelle autorisée, correction durable du garde à faire dans un lot distinct.
- **Famille** : produit
- **Gravité** : une validation peut mélanger du Rust récent et des gabarits anciens ; rouge navigateur observé, risque de faux vert sur des comportements non éprouvés.
- **Atteignable en service** : oui si un binaire ainsi construit est posé ; aucune pose réalisée ici.
- **Touche rag3weaver** : oui (`build.rs`, `scripts/garde_target.sh`, chemins Cargo et attestation des gabarits).
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

## Faits vérifiés

Source R5 `abb9a9559ca9a1ed874c92dbc73dba720952ff93`, worktree `cx-rag3db-arret`, target unique `~/.cache/rag3weaver-build/target-97k` à luciepc. Le navigateur a jugé `debug/deps/rag3weaver-4c704372471d580b` : 34 réussis, 2 échoués, zéro ignoré, 132,91 s. Échecs : lecture déjà exécutée non visible après erreur modèle (`chat/tests.rs:1803`) ; deux GET /ide/chat après un POST (`ide_par_le_flux.rs:80`).

Le fichier `test-lib-rag3weaver.json` de ce binaire référence le build-script d'empreinte `1391575270258181944`. Cette empreinte correspond exactement à `rag3weaver-d59c85ff98bf83cd/run-build-script-build-script-build`. Son `out/builtin_templates.rs` contient des `include_bytes!` absolus vers **cx-rag3db-arret-audit**, pas le worktree jugé, pour les gabarits embarqués.

Comparaison en lecture seule des 212 entrées de cette table avec les 212 gabarits de la source actuelle : aucun ajout/retrait, 210 contenus identiques, seuls `ide/app/app.js` et `ide/app/bundle.json` diffèrent (inventaire d'archi recoupé par recherche). Dans l'ELF navigateur, leurs anciens octets sont présents et les nouveaux absents. La comparaison de présence des 212 contenus courants trouve zéro absent dans le binaire de lib `rag3weaver-980f203206afbf94` et le binaire e2e `e2e-47ad151a79db32a6`, mais ces deux absences UI dans le binaire navigateur. Les contrôles précédents ne sont donc pas invalidés par une différence de gabarit constatée ; l'étape navigateur a ensuite été rejouée sur le bon artefact (voir récupération ci-dessous).

Empreintes SHA256 :

| artefact | courant | ancien réellement embarqué navigateur |
|---|---|---|
| app.js | d6597a5487d3043ab3b853d3426f9149d6aa6c7f5d4c93b5d9e066c76c36f302 | d25cb0d3aefbe5be82012cbd58cd9ac6c2cf46f94b68fccab08223136ea19f28 |
| bundle.json | 15094ccbbd1d91572bd887c869fe5469c6311e44c56cbb390ae40a2847e6e85b | 9a3fb42878bc2b270f750765334d58e1436830901bb870d1e911cc7567da473a |

Binaire rouge conservé hors target sous `journaux-cx-recherche-arret-11nov-r5/navigateur-rouge.elf`, SHA256 `6a10909cefa0f8359cc00f66107172290efd9c7c856c93234981d4f4dabf849f`. Même dossier : `diagnostic-bundle.txt`, `inventaires-elf-source.json`, `inventaire-source.sha256`, `builtin-templates-ancien.rs`, `build-script-ancien.json`, `build-script-ancien.fingerprint`, `test-lib-ancien.json`, et le journal rouge. Inventaire complémentaire : `cx-archi-diagnostic-r5-inventaire-20261111/{resume,inventaire}.json`.

## Frontière du garde

`run_e2e.sh` et `check_features.sh` appellent `garde_target.sh`. `cargo_magasin.sh` exporte la lib puis exécute cargo sans ce garde ; `temoins_navigateur.sh` passe par ce dernier. Les ciblés d'audit ont utilisé cargo_magasin directement avec le même target depuis le worktree d'audit. Une utilisation non gardée peut donc produire des artefacts sans mettre à jour `.arbre-du-target` ; un appel gardé ultérieur qui retrouve le propriétaire noté ne voit pas cette intrusion. Ce raccord des chemins est établi par les scripts et commandes ; les détails fins de fraîcheur Cargo restent à expliquer sans extrapoler.

Le contrôle qui compare bundle embarqué et manifeste embarqué passe avec un ancien couple cohérent : il ne prouve pas leur égalité avec le commit attendu. La validation doit aussi vérifier cette provenance externe.

## Récupération et correction à distinguer

L'orchestration autorise une seule modification de date de `build.rs`, target inactif, pour régénérer depuis le worktree correct, sans changement de contenu ni retouche manuelle du généré et sans nettoyage global. Après attestation des chemins et octets, deux ciblés inchangés puis le navigateur complet ; pas de nouvelle vague Gemini si ses gabarits ne diffèrent pas. Cette récupération n'est pas encore une garantie durable du garde, et le vieux rouge n'est pas repeint en vert.

Pour fermer : rendre la frontière de propriété effective sur tous les points d'entrée Cargo pertinents, éprouver le passage entre deux worktrees au même target, et refuser ou invalider explicitement un out/généré provenant d'un autre arbre. Attester tous les gabarits attendus et les octets UI servis contre le commit, pas seulement le manifeste qui voyage avec eux. Lot distinct du correctif d'arrêt.

## Résultat de la récupération ponctuelle

Une seule modification de date de `build.rs`, SHA source inchangé et aucun diff git, a reconstruit le navigateur en 48,19 s. Attestation : 212 chemins du bon worktree, 212 contenus courants présents, chemins UI `.d` conformes, ELF SHA256 `3e3770df4000178626a8c07631ab124ab6673c67b643b7ad2cae6f6e7cfc7063`. Archi a vérifié cette attestation indépendamment.

Les deux tests auparavant rouges passent inchangés séparément (1/0/0 chacun, 1,25 s et 67,85 s), puis le navigateur complet passe 36/0/0 en 132,35 s, ELF inchangé. Journaux nouveaux `nav-cible-erreur.log`, `nav-cible-flux.log`, `navigateur-reconstruit.log`, `attestation-nav-reconstruit.json` ; ancien rouge et ELF intacts. Le verdict agrégé R5 conserve les autres étapes vertes et remplace explicitement la seule étape navigateur. Aucune capture supplémentaire de la réponse HTTP n'a été ajoutée. Cette récupération confirme l'effet des gabarits périmés ; elle ne ferme pas le défaut de garde.
