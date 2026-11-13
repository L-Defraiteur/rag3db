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


## Reprise coordonnée — 13 novembre, contrôle périodique

Le travail durable reste ouvert et relève de Recherche. La récupération de R5
et la reprise ponctuelle du cache des couches ne corrigent pas les entrées
Cargo non gardées. Reprise autorisée de ce ticket existant, en lot isolé,
sans modifier les sources ni le target de la qualification union active.

Nouveau cas concret lié à la même frontière : application couches69dc3fc25910f3e834652264edf6c71cca862ce0,
piloteabaf460df4a254d825396051fffa9a648920d716, compilation Cargo directe dans
un target initialement absent ; après qualification CPU, le target ne portait
pas `.arbre-du-target`. Sur source union236e1e9f46f7e73eda21ac8cfa765f45a29f4595,
la branche « marqueur absent » de `garder_le_target` aurait adopté le nouvel
arbre sans invalider les empreintes locales. Réserve découverte en lecture
seule, avant exécution ; aucun faux vert produit n'est établi par ce cas.

Le pilote47861db8bc7321c9d8564ea784f20cb8538232a0 a exécuté une récupération
explicite sur ce cache privé : provenance69 prouvée, appel de la garde sous
l'ancien crate puis reprise sous le nouveau ; douze empreintes locales ont
été retirées, zéro restante, empreintes externes conservées. Preuve conservée
`cx-validation-couches-2a-union-v1/cache-reprise.json`, SHA256
f6e2cb53026b8af0a0e0cf60363f737bc0a72314b0240287a4a44c451e1a60ee.
L'arrêt F8 de ce pilote reste rouge ; cette attestation concerne la reprise
du cache seulement. La continuation8110 utilise le cache déjà attribué à
l'union, sans nouvelle invalidation. Ces ressources actives sont hors lot.

Prochain travail Recherche : relire les entrées Cargo actuelles et la garde
commune, écrire un contre-exemple CPU déterministe et proposer une correction
durable fondée sur les primitives existantes. Un target absent/vide doit
rester utilisable ; un target non vide sans provenance ne doit pas être
silencieusement attribué au nouvel arbre. Aucune origine ne sera inventée,
aucun partage adopté, aucune purge globale ni affaiblissement de provenance.
Conserver les dépendances externes quand une reprise qualifiée est possible.

Préparation, publication du plan/diff et témoins CPU sur fixtures privées
sont autorisées ; source/recette publiées avant exécution, premier rouge
inattendu STOP et diagnostic. Aucun Cargo/rustc de produit, navigateur,
service, modèle, ancien target ni lot union touché. Toute qualification
lourde ou intégration ultérieure attend un créneau et un couple exact revu.
Le ticket reste ouvert jusqu'à correction durable et critères initiaux prouvés.

## Lot Recherche repris avant diff — 13 novembre

Branche cx-recherche-garde-target, base940ad65bf, worktree isolé neuf.
[Plan et recette CPU](../13-11-2026/03-plan-garde-target.md) publiés avec les
sept témoins avant correction/exécution. Refus de l'origine inconnue et
raccord de cargo_magasin à la garde commune ; métadonnées Cargo simulées
uniquement. Aucun Cargo/rustc produit, target réel ou lot union touché.
Le ticket reste ouvert ; pas de garantie d'ELF réel avant qualification dédiée.

## Préparation et témoins CPU réalisés — lot isolé

Source des témoins29087a73d publiée avant exécution ; rouges attendus conservés
sur la garde initiale. Correctif302f9b827 puis neuf fonctions CPU vertes en
0,216s, métadonnées Cargo simulées, aucun Cargo/rustc réel. Raccords supplémentaires
599109a86 : F8 et compilations run_e2e utilisent aussi l'adaptateur de destination
réelle ; SDK et primitives runtime inchangés. Refus explicite des surcharges
--config non qualifiées. Fixture des journaux adaptée pour metadata, NON JOUÉE
car elle compile un adaptateur Rust. Détails, limites et recette dans le plan.
Revue requise avant qualification plus large ; ticket OUVERT, aucun master.

## Réserve Archi avant gel du pilote

RO7cb7dde8 : sur reprise connue, le second cargo metadata --all-features de
la garde oublie --offline/--locked/--frozen, pourtant transmis au premier
metadata de l'adaptateur. Il pourrait résoudre autrement avant la commande
finale. Nouveau dixième juge CPU sur cette branche, avant correction ; faux
Cargo uniquement. Qualification/pilote non lancés, ancien avis préparatoire
ne vaut pas validation de cette frontière. Transmettre les restrictions à
la primitive commune sans modifier les arguments finaux ni l'export SDK.

## Arrêt du pilote dédié AVANT correction de profil

GO698/bdff : préparation10CPU/toy8/journaux7/compilation/223gabarits ELFcontreGit
verte ; modulebuiltin8/1/0, juge state sousXDG_STATE_HOME refuse car pilote
fixe deux racines égales. Diagnostic source+panic663, pas défaut produitgarde
établi. STOP sans retry,252 fichiers SHA identiques deux postes, scopeROUGE
code1/restes[]/busabsent et quiescence vide. [Rapport05](../13-11-2026/05-arret-qualification-garde-target.md).
Préparer pilote à deux racines siblings et runtime manquant seulement sur
ELF figé, mêmeSTART/END ; nouveau couple/RO/GO avant toute exécution.

## Correctif pilote proposé après diagnostic publié

[Recette06](../13-11-2026/06-reprise-module-gabarits.md), nouveau pilote isolé :
state-fixture et xdg-state siblings, mêmes application698/ELF/SHA et deadline
originale911625.832473217. Préparations héritées, seul module9/attestations
à exécuter après nouveau GO exact ; pas Cargo/rustc/rebuild/NAV/service.
Anciennes preuves intactes, diagnosticf107 publié AVANT ce diff, ticket ouvert.

## Qualification ciblée composite obtenue, revue attendue

Application698b952ab / préparationbdff / reprisea5e sur GO distinct C7.
10CPU+8contrôlesRusttoy+7jugesjournaux+compilation et223blobsGit/chemins/ELF
acquisv1, rougeprofil8/1 conservé ; modulemanquantseul9/0/0 enreprise,
mêmeELF/SHA et borneinitiale1800 (649,830963sdepuisSTART). Scope/finaliseurVERT,
busabsent/quiescencevide,250preuvesv2SHAidentiquesdeuxpostes. [Rapport07](../13-11-2026/07-preuves-ciblees-garde-target.md).
AucunCargo/rebuildv2 ; ressources gardées. Ticket OUVERT avant revue/suffisance
et qualification F8/liblarge/NAV éventuelle, pas de master/retrait autorisé.

## Qualification plus large à préparer — avant témoin HTTP

C7 juge preuves67fc suffisantes pour le ciblé et demande l'union sur master
D17a (couches2a pures livrées), F8 canonique/liblarge/NAV40 et provenance des
223gabarits contre Git ET deux assets UI réellement servis. Nouvelle branche
cx-recherche-garde-target-union-r1 ; rebase des13 patches sans conflit.
Le témoin actuel ide::le_bundle_de_l_ide_est_celui_que_son_manifeste_decrit
compare seulement deux contenus embarqués entre eux. Ajouter un témoin
cfg(test) sur hôte HTTP privé qui compare app.js/app.css SERVIS aux fichiers
courants du worktree ; le pilote comparera ces SHA aux blobs du commit.
Ce témoin est une preuve du critère initial, aucune API de service ajoutée.
Préparation seule, recette/couple source figés et revue avant nouveau GO lourd.
