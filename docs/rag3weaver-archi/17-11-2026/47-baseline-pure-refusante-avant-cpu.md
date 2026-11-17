# Baseline pure refusante, publication avant le nominal CPU

Autorisation de PRÉPARATION09726 lue au ticket commun avant écriture. Contrat
5cae3c31731dea3348be2c16fe96ed256fe567ff et ses trois RO DOC/API pure conservés.
Aucun CPU n'est autorisé par cette préparation, aucune exécution effectuée.

Le vrai module partagé `extension/rag3weaver/scripts/execution_entry.py` fournit
les types et Refusal(code, sequence, invocation_returncode), puis refuse
inconditionnellement verify_transcript avec capability-not-implemented. Aucun
fichier, processus, signal, clock réel, réseau, publication ou adaptateur n'est
utilisé par cette fonction. Les types de données ne sont pas une validation
implémentée : aucun témoin47 n'est dit vert ou rouge à ce stade.

Le seul nominal `EntryContract.test_confirmed_entry_result_belongs_to_invocation`
convertit UNIQUEMENT Refusal(code=capability-not-implemented) en self.fail ; ses
six assertions positives sont conservées. Autre Refusal, ImportError, faute de
fixture et exception non contractuelle restent ERROR. Le module réel existe
avant toute exécution : une erreur d'import ne sera pas comptée comme rouge.
Les46autres fonctions sont inchangées par cette adaptation du nominal.

[Plan/pins du nominal](execution-entry-contract/baseline-nominal-pins.json) :
empreintes du vrai module, du témoin, du stub, interpréteur absolu et commande
proposée. L'import doit résoudre LE fichier partagé épinglé ; PYTHONPATH indique
uniquement scripts/, cwd celui des témoins, python -B. Pas de copie alternative
nommée execution_entry.py dans le répertoire de témoins. Le nom exact et l'option
failfast limitent la future exécution à UNE fonction, pas la suite47.

La commande est une proposition NON EXÉCUTÉE. Sous futur GO distinct : borne
TERM58/KILL2, un seul lancement du nominal, bruts source/import/résultat/log
conservés et réhashés sur deux hôtes avant correction. Si ERROR ou pin/import
inattendu, STOP et diagnostic avant modification ; pas de retry ni faux rouge.
Le rouge attendu est celui d'une nouvelle API pure refusante, jamais celui d'un
observateur déjà livré. Les autres témoins ne sont pas exécutés par ce GO futur.

Suite : RO baseline+test+pins, décision C7 pour GO du seul nominal. Après rouge
réel conservé seulement, préparation d'implémentation pure complète, publication,
nouvelle RO et GO47distinct avant tests. Pas d'implémentation anticipée derrière
le refus ; CP/Common réel, barrière/stabilité/SDK, quatre appelants/Cargo runner,
canon, native/target/services restent hors du présent périmètre.

Vérification présente : lecture de sources et git diff --check seulement ;
aucun import du module, parser des témoins, test/CPU/processus de qualification,
traceur ou accès proc exécuté. Les outils de rédaction/hachage ne produisent
aucune attestation physique et ne lancent pas les sources écrites.
