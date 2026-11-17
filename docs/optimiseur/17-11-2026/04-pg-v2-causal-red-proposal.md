# V2 — deux réserves groupées, rouges réels à observer avant activation

C7b13df07a23/FTS302. Aucune CPU, compilation, connexion ou lecturecredentials
exécutée par cette préparation. Fixture02 expirée/restituée, aucun PG alloué.

## État des sources, sans réécriture d'histoire

Le défaut Rust a été consigné AVANTdiff6f0c604ca7, puis un correctif SOURCE
c8d5e340d1 a été préparé sous l'autorisation antérieure de préparation. Il n'a
jamais été compilé ni appliqué au runtime. La nouvelle consigne C7 de ne pas
fixer avant rouge observé interdit de le déclarer validé/actif avant sa preuve.
La source 6fdb et paquets01/02 sont conservés avec leurs états exacts.
Le défaut Pythonstatusbool/int est consigné, témoinprépub7aa3644e39 ; le
prédicat `status==dict(...)` N'EST PAS encore corrigé. Aucun vert proposé sur
cette source tant que les rouges et correctifs n'ont pas leurs étapes admises.

## CPU causal legacy secret, et CPU causal types de fin

Deux invocations distinctes, NON JOUÉES, sous futurs GOs exacts timeout58TERM+
KILL2 (60MAX), rien d'autre découvert/exécuté :

1. `python3 -B -m unittest -v -f test_pg_v2_legacy_secret_red.LegacySecretBoundaryRed.test_legacy_b_artifacts_must_not_contain_stdin_canary`.
   Attendu1FAILassertNotIn(canary)/0ERROR0SKIP/rc1, vraiB_run/request.json avec
   fakeOS, preuve que credentials ne doivent pas transiter par le dumper B.
2. `python3 -B -m unittest -v -f test_pg_v2.V2Fixtures.test_finish_status_rejects_boolean_integer_aliases`.
   Attendu1FAIL«Refused not raised»/0ERROR0SKIP/rc1, vraiobserve+FakeTransport
   renvoyant reaped1/quiet1/returncodeFalse, acceptés par l'ancienprédicat.
   Ce n'est pas un refus simulé ou une erreurcompile/import : l'assertion
   constate le succès indu de la vraie fonction de production candidate.

Bruts/rc/temps/source/pins deuxpostes avant toute suite ; ERROR ou PASSinattendu
STOP0retry. Aucun fichiercredential original, proc/FLOCK/processus/natif réel
par les fixtures ; canaris et fichiers temporaires privés uniquement.

## Natif causal Tokio : extraction non corrective du vrai finisher6fdb

pg_ro_client_v2_join_red.rs dérive EXACTEMENT du fichierRust6fdb : seul le
corps de Owner::join_driver est extrait en join_red_baseline, et la méthode
réelle délègue au helper. Corps ancien inchangé modulo le nom du receiver :
`matches!(timeout_at(...),Ok(Ok(Ok(()))))`, puis si false abort+deuxièmeawait.
Il ne s'agit pas de FakeClose bool : le test passe un vrai JoinHandle Tokio
annulé (ReadyJoinError déjàrécolté) au helper qu'utilise réellement Owner.
Les dépendances/SQL/API/protocole demeurent ceux du candidat6fdb source.

Cette variante a trois tests SOURCE additionnels, tous NONCOMPILÉS :
`causal_join_red::ready_ok_control`, `causal_join_red::elapsed_control`,
`causal_join_red::ready_join_error_must_not_be_polled_twice`.
Un futur GO NATIVE distinct doit d'abord compiler --test cette variante sur
les quatreexterns archivés (serde_core inclus), en sortieprivéeneuve, code0
requis. Une erreur de compilation n'est PAS le rouge causal recherché et
arrête l'étape sansfallback/Cargo. Aucune compile actuellement autorisée.

Séquence native proposée sousPOSTE32G/Common/OS à fixer dans son GO : les
deuxcontrôles exacts attendusPASS/rc0, puis le casReadyJoinError exact attendu
1FAIL/rc101 avec «JoinHandle polled after completion». Uniquement ces trois
cas, --test-threads=1, --exact ; autres8tests historiquesNONJOUÉS. Bornes
compile60/cas5MINEND proposées, aucun budget renouvelé ; première surpriseSTOP.
Le testbin --test n'appelle PAS main/--capture-readonly-v2 ni codeconnexionPG.
Les témoignages prévus ne sont pas décrits comme des résultats observés.

Après ces rouges réellement acquis : correctifs sources groupés et nouvelle
RO/source/paquet/pins, CPUvert+nativeverts distincts. Le c8dSOURCE préparé est
une proposition de correction, pas une dérogation au rouge à acquérir. Aucun
clientC/GOcontact/lease n'est admis par ce protocole de reproduction.
