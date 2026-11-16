# C1 : le refus des offsets modifie une ancienne chaîne — rouge acquis

Ticket parent : [point de reprise dans le dictionnaire](../tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md). Branche cx-coeur-c1-dictionary-diagnostic, source du témoin seul b82c06f2572892d2124c71dce87eba287f5e9f2b ; moteur59ac9c86bc2b3529b590a7462ab7051e3ad26b8a. Aucun correctif256 ou autre changement produit. Admission corrective distincte C7 et lanceur4e3773250d639f2d7b396e7d4ff8611912f282ba publiés avant entrée, après diagnostic de l’arrêt runner01.

## Résultat exact, sans corriger le rouge

Cible dictionary_append_exception_test, unique cas DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged. Build1000étapes réussi ;1casjoué/1échec/0skip/0disabled/0error, XML conservé, code1 sans timeout. Processus test récolté après0,007654s (GTest arrondi0ms). BUILD_ID **émis par le test**, conforme au header généré et à l’ELF : rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.

Toutes les assertions préalables passent : trois chaînes alpha/beta/gamma, trois offsets, espace64octetspréalloué, refus de BufferManagerException sur la réservation attendue48octets (offsets3→6) atteinte une fois. Après le refus, trois offsets subsistent, mais :

- getString(2) retourne **gammadelta**, au lieu de **gamma** ;
- stringDataChunk.numValues vaut **19**, au lieu de **14**.

Les deux autres chaînes ne déclenchent pas d’échec. Ce rouge porte sur la valeur ancienne réellement lisible et le compteur d’octets, pas sur une exception de setup ni sur un crash probabiliste. Il montre qu’un ajout refusé n’a pas la garantie de préserver les anciennes valeurs. Aucun lookup/hash après erreur n’a été exécuté ; l’état réel des buckets n’est pas mesuré.

## Mécanisme et limite de causalité

appendString fait avancer le compte des octets après memcpy de delta, puis tente offsetChunk.resize. reserve refuse cette allocation et lève avant nouvel offset. getStringLength pour la dernière entrée calcule fin−début à partir du total des octets, d’où gamma+delta. Les compteurs d’offsets inchangés ne protègent donc pas la valeur de cette dernière chaîne. C’est distinct du correctif256 (ordre du compteur de lignes de StringChunkData), qui n’est ni importé ni exécuté.

Le SIGSEGV C1 pendant finalize n’est toujours **pas attribué** à ce défaut. Il reste à relier une exception de ce type à un dictionnaire effectivement réutilisé pendant le checkpoint, ou à établir une autre cause avec TSan et la lecture des durées de vie. Le test ne crée aucune Database et ne simule pas une transaction/annulation complète ; il isole l’invariant du dictionnaire. Il ne justifie aucune clôture C1 ni aucun vert global.

Proposition de correction à relire, pas codée : réserver d’abord les capacités nécessaires des offsets et des octets, avant de publier les octets/offsets/comptes, de sorte qu’un refus BufferManager n’altère aucune ancienne chaîne. Garder séparée la question d’une allocation std::unordered_set::insert qui lèverait après publication : le seul cas joué ne valide pas une garantie forte pour toutes les exceptions possibles. Le même témoin devra passer vert sur un commit correctif distinct, puis les autres tests pertinents dans une nouvelle recette ; aucun de ces passages n’a encore été lancé.

## Preuves, ressources, arrêt

[Preuves02](preuves/c1/rouge-02/) : scripts et recette réellement exécutés, limites, sourcecheckout, configure/build/test logs et résultats, XML, header, manifeste, inventaireSHA et quiescence. L’ELF et l’archive statique ont été sauvegardés horsscratch **avant** le contrôle d’identité puis rehashés après arrêt et après copie locale :

- ELF219d423ce9a9a72c121d7a3d5cca59d35cad0a7153436837388467a7b3eaa299,27032312octets ;
- archivec7648463b200c0108a5163584a9ebe63e3397e93484e68a65715548a9581093a,65763804octets ;
- header1cf920608514b5f3c5e90952ecc580ca6170398f288dd554d00bb0bb32b03110.

Paquet complet29fichiers gardé aux deux postes sous ~/.cache/rag3db-moteur-notes/c1-offsets-02/ ; grosELF/archive/source.bundle et compile_commands non ajoutés au dépôt, leurs SHA figurent dans l’inventaire. Aucun binaire du magasin remplacé. L’arrêt01 et ses artefacts manquants restent documentés séparément ; aucune preuve reconstituée comme si elle avait été observée.

Unité02 failed/exit1 en155,730s, CPU1167,423s, pic5,3G indiqué par systemd, swap0. Ancien arrêt01 144,848s : total des unités300,578s, ne confondre avec le temps du test. Scratch dur34359738368octets, utilisés680005632 ; sourceRO, sortiesprivées, réseau namespace sans route. Pas de GPU/service/PG/basevivante/SDKposé/cachepartagé/7970. Le poste réel affichait TSan seul avant entrée02 ; après fin notre place est rendue, TSan et G poursuivent leurs propres travaux.

Quiescence relue après sortie : MainPID0, ControlGroupvide, aucun PID attribué du contrôleur02 ou du test ; tous les processus test ont été récoltés. Premier rouge arrête, aucun retry ni correctif lancé. TSan existant n’a été ni arrêté ni relancé. Les anciennes admissions ne se réactivent pas ; prochaine étape soumise à coordination C7, sans nouvelle attente Lucie inventée.
