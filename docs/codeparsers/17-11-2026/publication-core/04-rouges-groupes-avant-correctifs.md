# Dix témoins groupés, moteur 558 inchangé, avant correction

Cadrage C7 après les RO totales Mémoire et TTY. Ticket ledger complété avant
ce diff. Nouvelle branche `cx-codeparsers-publication-guards-grouped` depuis
8421500d9a ; 558/9e653/842 et leurs états non exécutés sont conservés.
Aucun import/parser/test, processus testé, signal, SSH, acteur ou natif lancé.

Le paquet `test_publication_guards_grouped.py` charge explicitement :

| Sélection | Cas | État |
|---|---:|---|
|PublicationGuardCounterexamples, fichier original inchangé|3|NONJOUÉS|
|AdditionalPublicationGuards, nouveau fichier|7|NONJOUÉS|
|Total exact demandé au futur GO|10|Aucun rouge ni vert observé|

Les trois cas précédents restent : budget Common/deux watchdogs, hardEND final
local et moteur importé avant validation par le wrapper.
Les sept compléments sont :

- Procstart BOOTTIME décalé de5000, CLOCK_MONOTONIC indépendant : demande
  Common ne dépasse pas la fenêtre homologuée (le watchdog phase peut serrer).
- Même décalage : identité procstart ne devient pas une date de file et ne
  refuse pas une fenêtre MONOTONIC valide.
- Vraie entrée Gate chargeant un moteur dont seuls les octets ont changé.
- Vraie entrée pilote chargeant legacy dont seuls les octets ont changé.
- Vraie entrée pilote chargeant physical_guard dont seuls les octets ont changé.
- Vrai SSHTransport.call, Popen fictif : communicate consomme son budget et
  récolte lente ne reçoit pas deux grâces supplémentaires ; première cause reste
  celle du RPC, pas celle du nettoyage. Aucun appel SSH n'est exécuté.
- Vrai SSHTransport.stop : échéance locale originale explicite, aucune grace
  nouvelle. L'absence exacte du paramètre est rendue FAIL nommé, pas importERROR.

Les vrais fichiers source relay/gate/pilot/legacy/physical_guard sont copiés
comme octets de fixture. La seule mutation hostile est une sentinelle top-level
sur la dépendance non épinglée. Les bonnes empreintes restent dans prepared.
Seul ce RuntimeError exact est traduit en FAIL nommé ; toutes autres erreurs
restent ERROR. Les imports adverses de ces cas ne sont exécutés QUE par un futur
GO CPU, jamais par cette préparation. Les modules précédents sont restaurés.

## Enveloppe d'origine proposée pour lecture des témoins

Les fixtures ajoutent `whole_window` au bootstrap et des champs clock/soft_end/
kill_end au reçu outer-supervision : identité PID/start d'un côté, origine et
fins CLOCK_MONOTONIC indépendantes de l'autre. Ce sont des faits MODÉLISÉS du
producteur de confiance, pas un calcul depuis procstart ni une preuve d'hôte.
Le moteur558 les ignore encore : les deux contre-exemples montrent ce manque.
Le schéma/émetteur et la preuve de l'origine AVANT le vrai watchdog restent à
relire avant les correctifs ; une observation tardive ne redonne pas de temps.
Ces noms constituent une proposition documentaire de fixture, pas une API OS
qualifiée, ni des valeurs acceptées depuis un ledger ou programme testé.

Pour stop, `deadline` est une fin absolue dans l'horloge MONOTONIC locale,
jamais transmise numériquement à l'autre hôte. Les grâces doivent tenir DANS le
budget/END original. Un refus ne prouve pas l'absence : absence non constatée
reste UNKNOWN, les preuves d'arrêt et la première cause ne sont pas effacées.
Les assertions des 36/3/8 précédents ne sont pas modifiées ni exécutées ici.

## Suite

Pins source/test/entrée dans `publication-guards-grouped-pins.json`. RO ciblée
Mémoire/TTY du montage et des attentes avant tout GO distinct. Une erreur de
montage reste une erreur à conserver, jamais un rouge attendu du mécanisme.
Prévision dix FAIL seulement ; ce n'est pas un résultat. Aucune correction
n'est intégrée dans cette branche, le moteur reste strictement558.
