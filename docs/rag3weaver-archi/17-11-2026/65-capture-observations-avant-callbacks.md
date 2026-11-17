# Capture des observations avant callbacks : correctif préparé avant GO30

Réserve Mémoire10f753 et deux témoins41179 prépubliés avantfix. Deux rouges
égalité réellement acquis sous GO C7-ENTRY-OBSERVATIONS-RED-4117-17nov60 ;
preuveb5113587e7d9dd5a2c07e8ca2a047b9756ab32c6, neuf bruts LOCAL/PC réhash0écart
AVANTfix, manifeste
c13bb055e8ceaa1507f466e5fdf9ee701e745874223d6e49eb4d8ccd8aa43e12.
Aucun repeint, aucun GO28 consommé ou reporté.

Le runner capture désormais des copies récursives PRIVÉES des DONNÉES renvoyées
par les ports, sans exposer ces copies : mappings/listes/tuples copiés, scalaires
et bytes immuables conservés ; types étrangers refusés, jamais deepcopy d'un
objet-capacité. Les handles launch/held/seal/permit restent exactement ceux des
ports. Les copies de données ne sont jamais repassées à un callback.

Le helper operation effectue la capture immédiatement après le retour du port
AVANT le check d'horloge, pas après : entrée, seal, permis, finalisation. Le wait
est copié avant décodage du rc et toute lecture de clock ; le rc acquis reste
préservé si le check suivant refuse. Cleanup capture aussi les données avant la
clock finale, sans écraser l'erreur initiale. Les types de conteneurs de données
sont conservés, pas une conversion de liste en tuple altérant les gardes.

Les30témoins sont BYTEidentiques à41179 (fc300...), NONJOUÉS sur ce correctif.
Snapshots des attendus et checkEND après résultat/hash restent inchangés. Les
cas de buffers facts et RAW réutilisés doivent maintenant conserver les valeurs
ACQUISES (D et SHAmarkerLF), pas refuser artificiellement un buffer port réemployé.
Les erreurs de forme demeurent de vrais refus et les exceptions de fixture ne
sont pas maquées en rouge comportemental.

[Pins GO30 proposés](execution-entry-native-protocol/runner-effects-30-pins.json)
ferment source/test/pur565b/interpréteur, cwd/import/PYTHONPATH et UNE suite-v-f
TERM58/KILL2. executed=false/go=null ; demandeRO TOTALE source/pins avant décision
C7 et toute exécution. Aucun30vert anticipé ; baselineb40/rougesb3ab/b511 et
preuves puresf74/2a/892bc sont intactes. Pur565b non modifié/rejoué.

MODELED seulement : cp_ack/global_admissionfalse, SDKNOT_ATTESTED et imagesfutures
UNKNOWN. LIVE/Commonréel/CLInatifunsupported. Postvalidation des callbacks, PAS
nettoyageOSinterruptible/borné qualifié. Aucun backendOS/PQS/capture/printf/
CP/Common réel/SDK/appelant/Cargo/service/target/privilège/sandbox ajouté ou lancé.
