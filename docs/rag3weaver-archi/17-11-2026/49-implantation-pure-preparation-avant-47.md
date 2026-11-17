# Validation pure du contrat : implantation préparée, 47 non joués

Autorisation09726, confirmée C7f104d58c02 après suffisance des preuves rouges
f74e7f2cb580567d0ce22c20d8ea1c9d8748e52c. Rouge nominal réel antérieur :
1FAIL/0ERROR/skip/rc1, bruts neuf fichiers réhashés LOCAL/PC0écart, manifeste
`d977e605e15ad599ccf9f1869e06305c171e013f85d615bc7ae60bceb94ebadb`.
Ces bruts et sources baseline sont conservés intacts ; aucun test rejoué.

Le module partagé `extension/rag3weaver/scripts/execution_entry.py` prépare
maintenant la validation PURE de la séquence définie au contrat5cae3 :
plan indépendant, sélection, enfant, exec_stop, copie, attestation CP distincte,
libération, wait invocation, RAW, wait racine directe Common, clôture extérieure.
Une erreur interrompt au premier fait invalide et conserve le returncode enfant
s'il était déjà acquis ; les faits cleanup ultérieurs ne masquent pas la cause.
L'attestation peut manquer avant published, jamais au stade published.

Types contrôlés explicitement, sans confondre bool/int dans PID/start/UID,
séquences, tailles d'identité, références et champs numériques. Horloges finies,
start/issued/operation_end/end/réserve liés, tous faits après issued et monotones ;
phases de travail avant operation_end, clôtures sous END. Aucun nouveau budget
créé par une attestation, une observation ou le verdict de retour.

La fonction n'utilise ni fichier, hash de fichier, clock réelle, proc, processus,
signal, réseau, CP réel ou publication. Elle ne modifie pas plan/faits/attestation.
Les références SHA y sont opaques et comparées, PAS calculées depuis le noyau ou
un reçu physique. Les booléens EXEC_SUCCESS/stable/detach/quiet du modèle restent
des données de fixture ; aucun ne confère une capacité d'OS. Retour validé =
invocation_complete, pas jugement que rc0 signifie tests verts ; images futures
UNKNOWN et SDK NOT_ATTESTED restent constants. Aucune portée20 réintroduite.

Les **47 fonctions** prépubliées sont inchangées depuis la baseline2b040 ;
l'adaptation du seul nominal conserve les six assertions positives et ne
convertit que l'ancien Refusal capability-not-implemented en FAIL. Aucun témoin
n'a été importé/parsé/joué pendant cette préparation ; aucun47verts anticipé.
L'unique rouge acquis reste celui du nominal de la baseline, pas47rouges.

[Pins proposés pour un futur GO47](execution-entry-contract/pure-47-pins.json)
ferment module réel, test et stub ainsi que l'interpréteur. Commande proposée :
unittest -v -f test_execution_entry, une invocation bornée TERM58/KILL2,
PYTHONPATH limité au scripts/ du WT/cwd des témoins, -B. Ce fichier n'est PAS un
GO. Une RO source totale est demandée avant décision C7 et toute exécution.
Si première erreur future : stop sans retry et conservation avant correction.

La topologie Common/son vrai wait, les ACK CP, la capture d'entrée/stabilité,
SDK, propagation Cargo et quatre appelants restent NON IMPLÉMENTÉS/non qualifiés.
Aucun adaptateur ou backend opérationnel n'est ajouté. Canon, cibles Cargo,
services, corpus et anciennes fenêtres n'ont pas été touchés. Vérification de
cette préparation : lecture de source et git diff --check seulement.
