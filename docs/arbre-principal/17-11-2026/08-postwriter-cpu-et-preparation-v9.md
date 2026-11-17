# Contrat post-writer : CPU et préparation v9

Source exécutable préparée : **e9f037be2aa25da2c7f6c76f79e2eb57b6c676c9**.
Ce rapport documentaire ne change pas le SHA préparé. Ticket distinct et
proposition07 portent le contrat ; preuves65eb/Qv8 restent immuables ROUGES.

## Rouges avant corrections puis ensemble vert

- GO C7-POSTWRITER-CAUSAL-RED-48A2-17nov60 : source48a2 propre, seul nominal
  deux retraits .wal/.ecrivain, 1 FAIL readonlycopychanged capturé→assert,
  0 ERROR/skip,0.008s ; rc1,monotonic local878244.750755155→878244.894996222.
- GO C7-POSTWRITER-COLD-RED-336C-17nov60 : source336c propre, seul positif
  COLD .readers vide,1 FAIL sourcesorcoldfixturemutated capturé→assert,
  0 ERROR/skip,0.007s ; rc1,monotonic878728.377602733→878728.488411714.
- GO C7-POSTWRITER-GREEN-E9F0-17nov60 : sourcee9f0 propre, une invocation
  timeout TERM59s/kill1s, python3 -B -m unittest -v -f des trois modules
  test_postwriter_inventory, test_prewriter_persistence, test_postwriter_cold_reads,
  cwd debt-copy-a, PYTHONPATH coordinations16/17. **19 PASS,0 FAIL/ERROR/skip**,
  0.356s unittest ; rc0,monotonic878918.276162522→878918.753494766.

Quinze POST +un PRE +trois COLD :19 juges distincts,21 invocations avec les
deux rouges initiaux. Les sept juges Common/HB/final antérieurs sont conservés
sans rejeu ; aucune nouvelle preuve native par ces fixtures CPU.

Bruts stdout/stderr/reçu et manifeste sous ~/.cache/rag3weaver-build sur
les deux hôtes ; trois fichiers référencés par archive réempreintés PC après
copie, taille/SHA zéro écart :

| Archive | SHA256 manifeste |
| --- | --- |
| postwriter-causal-red-48a2-20261117-01 | 046c89cf87dbc61fd319f01ba024f47b48dc198a3f503cc29e53c6be2e5c1d64 |
| postwriter-cold-red-336c-20261117-01 | 0ece970a239988394f7ebe05952b0a8b066c957e8ce13ab88271026f17f4488f |
| postwriter-green-e9f0-20261117-01 | bc6272eb53a009634285f56b4310aba5c72365d81381b95c634494bf025af557 |

## Portée du correctif et des juges

Inventaires fichiers/dirs fsync avant appel writer ; contexte lie base/pilot/
prepared/nonce/END/cgroup/profil et SHA inventaires. Admission physique locale
postwriter distincte du readonly, preuves PID/exe/cgroup/maps/profil et
publication→shutdownACK/rc0 recoupées avec deux frames brutes via Session.
WAL/marque exacts absents, DB mode conservé et présente, autres fichiers/dirs
stricts sauf .readers vide initialement absent, doublons/types refusés.
END/premier rouge contrôlés, erreur de preuve convertie en refus explicite.
Le raccord COLD utilise ensuite stable_readers sur le nouvel inventaire,
DB comprise. Le validateur readonly lui-même est inchangé.

Les preuves writer et inventaires sont synthétiques au vrai bloc AST ; les
mutants d'attestation sont injectés sur inventaire neutre pour atteindre leur
branche, avec compteur de l'appel réel. Le juge PRE vérifie l'ordre des vrais
assignment/save/writer sélectionnés, autres actions explicitement exclues :
ce n'est pas une exécution complète du driver ni une preuve de durabilité OS.
L'égalité des artefacts locaux ne protège pas d'un acteur hostile du même UID.
L'admission physique reste explicitement sans qualification logique.

## Paquet PC métadonnées seulement

WT /home/lucied/git_workspaces/rag3db-cx-arbre-cycle-postwriter-20261117-r1,
HEAD e9f0 exact propre. Qv9 sous
~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-control-v9.
Prepared SHA256 **96ebc22e7fae474f812273e0bf2f8d41b3f24b5205c6e4f21d4594fbbd0f3099**.
Vingt-deux copies SHA conformes, préparation/guard rc0. P lifecycle-copy-v9,
S lifecycle-scopes-v9, allocation.json et PROVIDER_GO.json absents après
préparation. Aucune DB ouverte, aucun provider contacté. Ancien Qv8 conservé.

Deux RO TOTALES source+paquet demandées, sans GO900/auditfreeze/rewrite/native
implicite. Les suites POSTgraph/FTS/bitdigest Qv8 restent NON JOUÉES ; intégrité
FIN après rouge, innocuité des écarts et durabilité/LIVE restent non qualifiées.
Aucune réactivation de l'ancien budget ni retrait/nettoyage autorisé.
