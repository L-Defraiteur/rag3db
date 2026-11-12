# Build privé corrigé : paquet attesté, scénario non lancé

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Build02 autorisé après correction précise du premier échec ;
source `35ea70d3a4a5cc4917586de02f6bfc862c76a02c` sur
cx-coeur-diagnostic-saturation. Source0511 et bibliothèque distribuée non modifiées.

**Les trois cibles compilent et lient. Aucun scénario, test, découverte GTest ni
binaire du paquet exécuté.** Les déclarations GTest de cette cible sont PRE_TEST ;
le build ne les déclenche pas. La revue ELF utilise readelf, pas ldd.

## Exécution bornée et poste rendu

Unité cx-coeur-build-saturation-12nov-02 : rc0, service154,588s,
CPU1183,530s, pic5,1G tel que rapporté par systemd, swap0. Le build seul dure
149,865871s ; staging/configuration/revalidation/sauvegarde inclus dans154,514299s
mesurées par le contrôleur. Aucune hausse des plafonds ni reprise d'objet ancien.

Limites effectives : cgroup8G/swap0, tmpfs scratch dur2G, réseau isolé/sourceRO,
seules sorties privées build/tmp/extensions/outputRW, poste lourd -j8, enveloppe
1795s+5s d'arrêt. Scratch utilisé en fin de contrôle678850560octets. Seules cibles
concurrence_test, rag3db_vector_extension, rag3db_geo_extension ; pas d'installation.

Avant compilation : commit/status propre, [delta exact de qualification](preuves/build-saturation-02/correction-qualification.json),
commandes normalisées et system_config identiques au préflight ; multiensemble des
1181 commandes, doublons helpers compris, sans ajout/retrait. Les seules sources
modifiées depuis6bf sont les qualifications common:: du document17. Aucun autre
écart utilisé pour résoudre le premier échec, qui reste conservé séparément.

État après sortie : inactive/MainPID0/Resultsuccess ; poste libre0/4, aucune mesure
réservée. [Limites](preuves/build-saturation-02/limites.json),
[journal unité](preuves/build-saturation-02/unite.log),
[état final](preuves/build-saturation-02/etat-final.txt).

## Paquet privé conservé hors worktree

À luciepc, chemin stable :
`~/.cache/rag3db-moteur-notes/annulation/cx-build-saturation-12nov-02/package/`.
834 fichiers,110723932octets : moteur statique, binaire, deux extensions,
811headers source,2headers générés et17archives tierces du même build.
Tous les fichiers sont rehashés après copie dans le paquet, puis une seconde fois
après fin d'unité : aucun écart sur834. Aucun mélange avec l'ancien paquet0511.

| Artefact relatif au paquet | Octets | SHA256 |
|---|---:|---|
| lib/librag3db.a | 65614590 | 2ae0c5d32bb605e60b04813ba570571be13721ea2564e8cba3ac98a796759837 |
| bin/concurrence_test | 31358560 | 6e31ea0322988e12079a38b996d65c3732aaec3734f2112b3c883bce96dbba35 |
| extension/vector/build/libvector.rag3db_extension | 958160 | 8371a69015feaec963ad075cf30436bd51cfd41674d7d32f820c23de06cce371 |
| extension/geo/build/libgeo.rag3db_extension | 393872 | 9126962bc8ac9c0abf2214ef9038dae8269d196d40f482ff7f1b7863ce8178d4 |

[Manifeste intégral](preuves/build-saturation-02/manifest.json) : source, cache et
compile_commands SHA256, racines compilées, tous fichiers/tailles/empreintes,
headers générés, runtime_authorized=false.

BuildID trouvé dans l'en-tête généré **et** dans les octets du binaire et des deux
extensions :

`rag3db-build-35ea70d3a4a5cc4917586de02f6bfc862c76a02c-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`

Les chemins source/build compilés sont éphémères :
`~/.cache/rag3weaver-build/cx-build-saturation-12nov-02/{source,build}`.
Un futur runtime doit remonter le paquet en lecture seule avec l'extension vector
visible au **chemin source compilé**, sans substitution par une extension partagée.
Le futur fichier CX_SATURATION_OUTPUT sera neuf dans une sortie privée persistante.
Aucun montage runtime ou lancement n'est effectué ici.

## Revue ELF sans exécution

[Revue et empreintes des dépendances directes](preuves/build-saturation-02/revue-manifeste-elf.json).
Aucun RPATH/RUNPATH dans les trois ELF, aucune dépendance dynamique à une lib du
projet. Le binaire complet de test a **neuf** dépendances directes : quatre GTest/
GMock système1.18.0, libstdc++, libm, libgcc_s, libc et ld-linux. Ce n'est pas le
pilote autonome historique à cinq dépendances. Les extensions ont les quatre
bibliothèques C++/m/gcc_s/C ; elles utilisent les symboles exportés par l'hôte.

Toutes les dépendances directes existent dans /usr/lib, résolues et empreintes
consignées. Aucune whitelist élargie silencieusement : GTest système est celui
trouvé à la configuration et nommé dans les commandes de lien. La résolution
récursive du chargeur et le chargement effectif des extensions ne sont pas testés ;
la revue doit les distinguer de l'inspection statique réussie.

## Preuves exportées et limites du verdict

[Archive de preuves](preuves/build-saturation-02/cx-build-saturation-12nov-02-preuves.tar.gz),
475624octets, SHA256
`d8f25939471ea2a741bed4a23db11fdc5327a80dfd1661e1c9e0fca42a98ea9c`.
35 fichiers du manifeste de preuves rehashés après export local, sans écart ;
[reçu](preuves/build-saturation-02/recu-export.json). L'archive contient configure/
build logs, argv, scripts exacts, correction, caches/compile_commands,3listes Ninja,
références du préflight, revalidation, ELF, manifeste et limites/état final.
Les binaires110Mo restent dans le paquet privé distant, pas dans cette archive
ni publiés comme bibliothèque distribuée.

L'échec01 est conservé à côté, sans réécriture. Cette seconde tentative est celle
explicitement autorisée après correction ; aucun troisième build et aucune
exécution du scénario. Le runtime1200 doit encore faire l'objet de la revue du
manifeste et d'une autorisation distincte. La compilation ne valide ni le chemin
runtime de capture, ni le départage des causes de saturation, ni une récupération.
Ticket toujours ouvert ; correction de sûreté0511 et coût restent distincts.
