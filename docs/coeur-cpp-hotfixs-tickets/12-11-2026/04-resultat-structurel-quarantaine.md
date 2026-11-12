# Résultat : le commit déjà ouvert atteint le chemin interne de checkpoint

Porteur hotfixs, [même ticket ouvert](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
**Observation structurelle confirmée ; aucune corruption, innocuité générale ou
récupération démontrée. Aucun checkpoint réel ni réouverture dans ce témoin.**

## Périmètre effectivement joué

Source moteur privé `0511dece74f94db6937fb03e6ec45f0c58828c5a`, archive assortie
attestée, pas la bibliothèque distribuée. Source témoin `8cc47439f` après seule
correction autorisée de la constante maxDBSize (`-1ull` vers `-1u`). Les flags,
archives, lanceur et logique sont inchangés. Compilation et lien de ce source
ont réussi dans un staging neuf03 ; la première erreur de compilation et le
premier runtime inconclusif restent conservés séparément.

Une seule nouvelle exécution structurelle, explicitement autorisée après cette
correction de prémisse et relecture ELF/quatre SHA. Aucun run supplémentaire
pendant ou après la coordination du service GPU.

## Trace observée

[steps.tsv brut](preuves/runtime-02/steps.tsv) :

1. Douze opérations de préparation réussissent. TA et TB sont ouvertes avant
   faute ; TA a ses deux lignes COPY dans le groupe partagé, TB une insertion
   locale dans une autre table. La fixture vérifie que TA tient des verrous.
2. TA annule par le vrai chemin Connection/TransactionManager. L’index de
   fixture lève une fois après l’entrée du handler d’undo. Le refus cite la
   faute injectée et la nécessité de fermer/rouvrir la base.
3. Vérifications de fixture passées : contexte/identité/offset attendus,
   callback one-shot, TA inactive, zéro verrou tenu pour TA, TB encore active.
4. BEGIN et CHECKPOINT publics depuis la connexion de contrôle refusent la
   base par le nom attendu de la quarantaine, sans atteindre la fabrique.
5. COMMIT de TB déjà ouverte, rendue forcée par l’API existante de fixture :
   **une entrée dans la fabrique interne** puis exception sentinelle avant
   construction de tout Checkpointer. Classification `internal-path-reached`.
6. Aucun second callback TA. L’enfant termine par `_exit`, pas par les
   destructeurs Database/Connection ; le parent le récolte, statut0.

Cela confirme l’arc du contrôle de flux sous ces préconditions : les refus
publics ne couvrent pas ce chemin interne depuis une transaction déjà ouverte.
Le forçage de TB est explicite dans le test ; ce résultat ne prouve pas à lui
seul une DDL réelle, une saturation du pool, un checkpoint réellement écrit,
une lecture fausse ou une corruption persistante.

## Empreintes et limites attestées

- Source : `14d99084010e0b8f5f530f2faa5926ca724382d44ab9deca24a31545baf71afd`.
- Binaire : `37ccee63d54555552a4541bf6f2aa8f55c1b97210e0b1d43f1332a1c28dcddd7`.
- Rapport de build : `9f780e3f07427d65bb6eb1acc21a0b583b2e6785b1c40b05e9ef70faf28a4432`.
- Lanceur : `a110d161f027b67f2464112058fde9250c1ebf801bbe0b2b557d888c21fb1e6f`.

[Rapport compile03](preuves/compile-03/build-report.json),
[ELF](preuves/compile-03/elf-dynamic.log) : cinq dépendances système attendues,
sans RPATH/RUNPATH/lib projet ; pack830fichiers/17archives rehashés avant,
pack/entrées principales après. Unité build cgroup1Gio/swap0 vérifié,
compilation/lien3,834s de contrôleur, service3,861s, pic affiché542,9M/swap0.

Runtime sous `poste lourd`, unité
`cx-hotfixs-quarantaine-runtime-8cc47439f-02.service` ; cgroup1Gio/swap0
vérifié avant lancement, limite20s et arrêt de tout le groupe, délai intérieur15s,
alarme enfant12s et requêtes1000ms. [Accord exclusif](preuves/runtime-02/runtime-approval.json)
lié aux quatre empreintes, [rapport runtime](preuves/runtime-02/runtime-result.json),
[journal superviseur](preuves/runtime-02/runtime-controller-02.log).
Service84ms, CPU67ms, pic affiché43M et swap0. Ces durées ne sont pas une mesure
de performance du rollback. Sortie143octets, aucun timeout ni dépassement.

[Parent](preuves/runtime-02/parent.tsv) : waitpid1880650, statut0 ;
[preuve de fin](preuves/runtime-02/runtime-finished-02.json) : cgroup disparu,
quatre empreintes inchangées après l’essai. Lecture systemd : unitéinactive/dead,
MainPID0, ControlGroup vide. Programme et wrapper sortent0 : diagnostic observé,
aucun verdict « base saine » implicite.

## Artefacts conservés et suite

À luciepc :
`~/.cache/rag3db-hotfixs-notes/cx-quarantaine-0511-structurel-03/` contient
source, objet, binaire, recette et rapports de compilation ;
`runtime-aymvkbea/artifacts/released-az2teC/` conserve la base privée,
le CSV, les steps et le statut parent. Aucun fichier de base copié dans les docs,
ni nettoyage de cette preuve. Les contrôles et traces textuelles seuls y sont copiés.

Le ticket reste ouvert. L’arc précis est établi ; le contrat de protection des
transactions déjà ouvertes et les conséquences d’un éventuel checkpoint réel
restent à traiter sous autorisation distincte. Les volets lecteurs, automatique,
verrous gardés, récupération et correction moteur ne sont pas joués ici.
Aucun code produit changé, aucune bibliothèque/service utilisateur activé,
aucun push master de ce lot.
