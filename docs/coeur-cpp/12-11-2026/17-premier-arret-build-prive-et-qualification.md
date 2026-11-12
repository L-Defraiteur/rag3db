# Premier arrêt de build privé et correction de qualification

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. La première tentative est arrêtée et conservée ; aucun scénario.

## Échec 01 constaté

Source6bfba5b4c, configuration revalidée sans écart, trois cibles uniquement.
Première erreur à l'étape982/1072 : transaction.cpp:216 et :228 utilisent
private_saturation sans qualification. Ce fichier importe catalog mais pas
common ; les autres fichiers ajoutés avaient déjà accès à common. Le diff privé
manquait donc une qualification, pas une dépendance ni un changement de pool.

Ninja arrête au premier échec ; les travaux déjà en cours finissent jusqu'à989.
Aucune relance de cette source, aucun paquet final ni binaire lié exporté.
Les warnings Release sur advanced dans discardRange sont distincts et non fatals.
Service131,098s, CPU981,212s, pic3,9G/swap0, cgroup8G/swap0/scratch2G attestés.
MainPID0, poste rendu0/4 après cet arrêt.

[Archive intégrale échec01](preuves/build-saturation-arret-01/cx-build-saturation-12nov-01-arret.tar.gz),
421962 octets, SHA256
`46a3df31f9e5d6929341a64ed61683bbe9d0cb82332d3db749d15d082b1b54cf`.
27 fichiers rehashés sans écart après export. Sont conservés build.log,
configure, cache/compile_commands, trois listes Ninja, contrôleurs exacts,
limites, résultat/état final, source.pack et référence de préflight.
[Première erreur](preuves/build-saturation-arret-01/premier-echec.json),
[revalidation multiensemble](preuves/build-saturation-arret-01/revalidation-multiensemble.json),
[reçu](preuves/build-saturation-arret-01/recu-export.json).
Copie stable hors worktree à luciepc :
`~/.cache/rag3db-moteur-notes/annulation/cx-build-saturation-12nov-01-arret.tar.gz`.

La revalidation compare aussi les **1181 commandes comme multiensemble**, sans
perdre les deux fichiers helpers compilés chacun deux fois. Zéro ajout/retrait.
Le premier collecteur indexait les fichiers uniques : cette vérification
supplémentaire confirme qu'aucune variante n'était masquée.

## Delta exact autorisé et publié

Source corrigée : `35ea70d3a4a5cc4917586de02f6bfc862c76a02c`, branche
cx-coeur-diagnostic-saturation. Seul fichier changé depuis6bf :
`src/transaction/transaction.cpp`. Le [patch](preuves/build-saturation-arret-01/qualification-proposee.patch)
est appliqué tel quel, sans ajout d'un using namespace large :

- common::private_saturation::published ;
- common::private_saturation::Scope et common::private_saturation::Phase.

SHA256 du fichier transaction.cpp corrigé :
`bd5ee195d32a36812688911dc8dc9f45ca603f8e5ec5f918b31d19472bce23f8`.
Pack source depuis0511 :78914 octets, SHA256
`0a564f5614b37b7dfcf6c6c7c156012f5a76c38914699c40abe7d473dbaecef3`.
Contrôle textuel exact du delta et absence d'autres fichiers modifiés : conformes.
Aucun changement d'ABI, de commande ou de mécanisme recherché par ce delta ;
la compilation corrigée doit encore le vérifier.

## Tentative 02 distinctement autorisée

Après revue du défaut précis, un seul build corrigé est autorisé, sans nouvelle
étape administrative intermédiaire : staging neuf35ea70d3a, revalidation avec le
préflight, mêmes trois cibles, -j8/poste lourd/global1800/cgroup8G-swap0/scratch2G,
sourceRO avec sorties extensions privéesRW, réseau isolé. Premier autre rouge :
arrêt/diagnostic, pas de hausse ni réemploi d'objet ancien. Source0511 et bibliothèque
distribuée non modifiées. Cette autorisation ne couvre aucun scénario.

Au moment de cette note, unité cx-coeur-build-saturation-12nov-02 lancée ; résultat
et manifeste seront publiés séparément. Aucun succès de compilation corrigée
n'est présumé ici. Le ticket reste ouvert, correction de sûreté et coût distincts.
