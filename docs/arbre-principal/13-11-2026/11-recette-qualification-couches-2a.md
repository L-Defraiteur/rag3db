# Recette concrète2a — 3000 secondes uniques

Préparation après arbitrage C7, PAS GO d’exécution. Application EXACTE
69dc3fc25910f3e834652264edf6c71cca862ce0, WTcx-wt-couches-epinglees figé.
Pilote dans branche/WT séparés cx-arbre-couches-pilote-r1/cx-wt-couches-pilote.
Les deux HEAD propres sont contrôlés avant entrée et entre chaque commande.

## Admission et répertoires

Sur luciepc uniquement, après revue+GO exact du couple application/pilote :

```bash
python3 /home/lucied/.cache/rag3weaver-build/cx-wt-couches-pilote/docs/arbre-principal/13-11-2026/couches-2a/lancer.py SHA_PILOTE_EXACT
```

Avant gel PC : checkout du sous-module codeparsers EXACT au gitlink app69
(requis même pour résolution Cargo de dépendance optionnelle, aucune édition
de grammaire). Le lanceur refuse sous-module absent/mauvais SHA et gdb absent.
Aucun sous-module n’est initialisé sur une ancienne source ou un autre lot.

P=cache/cx-validation-couches-2a-v1, Q=cache/cx-pilote-couches-2a-v1,
S=cache/cx-confinement-couches-2a-v1, T=cache/cx-target-contenu-derive.
Ces quatre destinations doivent être ABSENTES, jamais effacées/réutilisées par
le pilote. T privé et neuf ; aucun target ED partagé. Q reçoit copies hashées
driver/finaliseur/native-check et primitive commune confiner_validation.py byte-identique
à l’application. Superviseur/driver/finaliseur et cwd lanceur /var/tmp horsW ;
seuls Cargo et ELF prennent cwdcrate comme Cargo.

Avant poste, le lanceur initialise XDG_RUNTIME_DIR=/run/user/UID et
DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/UID/bus pour le manager utilisateur
EXISTANT, vérifie propriétaire/socket et Ping busctl. Aucun redémarrage ou
création de service : bus absent/illisible refuse avant admission, preuveQ
conservée. La liste blanche du driver vient APRÈS poste et confiner, avec
DBUS_SESSION_BUS_ADDRESS remplacée par le bus privé unique.

Le lanceur prend poste lourd réel Max32GiB/HIGHmax, puis superviseur commun
--delai3000 et bus privé UNIQUE. AUTH.echeance est l’unique borne, départ
=echeance−3000. Chaque phase=min(cap,tempsrestant−15), aucune nouvelle fenêtre
ni reset. jobs8 et quatre rôles BURNCPU ; buffer1GiB et SDK29 exact+hash,
flag ancien explicite seulement pour le CHECK natif. Les juges CPU ne
sourcent PAS exporter_la_lib, ne reçoivent aucune variable SDK et n’exigent
pas de maps librag3db.so. Aucun C++/SDK rebuild, DB, modèle, service ou benchmark.
Coordination ED autorise seulement deux slots lourds isolés, pas mesure perf.

## Commandes et assertions

1. Env construit par liste blanche, aucun secret/service/PG/Gemini hérité.
   verifier commun prouve reçu/poste/cgroup effectifs.
2. cargo test --lib --no-default-features --no-run
   --message-format=json-render-diagnostics, cap1800. ELF extrait UNIQUEMENT
   compiler-artifact réel rag3weaver/profile.test/executable, sousT. Source69
   cherchée dans ELF, copie horsT+SHA avant runtime. Aucun glob/nomdeviné.
3. Inventaires ELF layers::tests et publication:: (+listignored) cap30 chacun.
   Layers doit être exactement les4 noms attendus et0ignored ; inventaire
   publication complet avec ignorés comptés, jamais promus verts. Ledger
   source/pilote/features[]/ELF/SHA/deadline/inventaires écrit, PAUSE. Copie/preuve deux
   postes et publication du SHA ledger AVANT marqueur inventaire-publie.
4. Quatre invocations --exact NOM --nocapture --test-threads=8, cap60 chacune,
   chacune exige1passed/0failed/0ignored. Le premier exact est exécuté via gdb
   --nx --batch --return-child-result : start au main de CETTE invocation,
   relève /proc maps/exe/env/cgroup/starttime puis continue le même processus.
   Aucun test supplémentaire/faux ELF/délai injecté. debuginfod désactivé.
   Ce point déterministe évite qu’un test pur trop court échappe au polling.
5. Même ELF, filtre publication::, cap120 : passed=nombreinventorié−ignorés,
   failed0 et ignored exactement attendus. Les4exact sont rejugés dans ce
   module : distinguer fonctions distinctes et invocations brutes, pas faux
   double compte. Tous les logs nocapture ; premier FAILED/rc nonnul stop.
6. cargo check --lib --no-default-features --features rag3db-native, cap900
   intersecté même échéance. S’il manque le budget, natif NON QUALIFIÉ et
   composite nonVERT : aucune déduction depuis CPU seul. Compile adapter
   n’est PAS un runtime physique/natif ni une ouvertureDB. Export magasin
   limité à cette commande ; native-check.py enregistre le vrai env exporté
   (SDK/LD/include/shared/pool), hash .so et cgroup puis exec ce même Cargo.
7. SHA ELF original+copie inchangés, maps/profilréels à main, mémoire/events,
   target<=32GiB, quiescence, sources/pilote/copies figés.

## Observations et fin

Chaque commande descend du superviseur ; inventaire filiation PID+starttime
tousUID avant attribution, cgroup/memory.max réel exigés. FDdevices/maps
observés uniquement pour descendants attribués. Profil ELF réduit aux clés
contrôlées, jamais dump complet d’environ. Maps CPU réelles peuvent n’avoir
AUCUNE ligne librag3db.so : ne pas inventer un chargement natif pour un check
de compilation. Toute ligne native observée doit désignerSDK29 exact.

PermissionError=UNKNOWN/champ/pid/starttime/cgroup/exe, poursuite seulement si
confinement déjà prouvé ; CG illisible ROUGE. Owned protégé vivant bloque
quiescence. UNKNOWN/devicesvide ≠ absenceGPU, FDdevice ≠ calculGPU.
Échantillonsmemory.current/peak/events scope et target du -sb conservés.
HIGH/MAX/OOM/OOMkill ne doivent pas augmenter. Aucun perfclaim.

Premier rouge STOP sans autre phase, TERM/KILL via pidfd des seuls owned
avec starttime vérifié, fixtures/logs/target conservés pour diagnostic.
Finaliseur horsW/horsbus vérifie superviseurVERT0/restes[]/nettoyé/busabsent,
quiescence indépendante bornée au lot (inconnus baseline consignés), délai
<3000, SHA ELF et trois statuts de qualification exacts. Seul alors composé
VERT ; natif_runtime_qualified restefalse.

Après fin/arrêt : copie intégrale preuves+superviseur sur deux postes et
manifestes SHA comparés, sources propres et quiescence relues. Aucun master
ou retrait/relance autorisé par cette recette. Quatre CPU et module restent
NON JOUÉS à la publication du pilote ;2b/L6/top-k/outils non qualifiés.


Delta après RO mémoire b38 : le finaliseur relit l’échéance APRÈS scan et
hashes ELF, puis juste avant publication du verdict composé. Dépassement
pendant ces contrôles coûteux donne ROUGE ; aucun VERT d’échéance antérieure
réutilisé pour certifier une fin tardive. Aucun changement de l’application.


Delta après RO ED5a0a : en phase exact/publication, détection en activité de
« test ... FAILED », FAILED autonome OU « panicked at », y compris préfixe
libtest mélangé avec stdoutnocapture. Aucun should_panic/catch attendu dans
ces modules ; arrêt avant rc pour ces marqueurs, log conservé. Pas de
ouverture d’autre phase. Le pont manager vérifie resolve==chemin/lstat type
dir+socket/UIDdesDEUX et stabilité dev+ino+mode+UID autour du Ping ; aucun
redémarrage ou repli vers un bus différent.
