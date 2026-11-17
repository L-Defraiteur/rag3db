# Snapshot prepared et première cause : candidat SOURCE, 63 cas non joués

Après les cinq rouges6f47 précis conservés deux hôtes (rapport11), correction
SOURCE des deux familles M/TTY sous le même ticket, complété avant le diff.
Aucun import/parser/test du candidat ou ancien paquet, aucun effet de phase,
acteur, SSH, natif, Q global, target ou corpus exécuté. Les preuves précédentes
et leurs sources restent figées ; aucun rouge de montage CA1C n'est requalifié.

## Une lecture prepared liée au pin, pour tous les consommateurs

`publication_loader.read_prepared(path,expected_sha)` ouvre un FD non suiveur de
symlink, vérifie type/UID sur ce FD et lit un seul payload. SHA puis json.loads
portent sur CES octets. Les chemins env/vault/autorité ne sont pas des prepared.
verify_entry, relay_main, gate_main et pilot.main passent par cette primitive.
Le noyau compare aussi le contexte fourni au contexte lié au pin indépendant,
avant les contrôleurs/imports et opérations runtime ; le même SHA ne couvre pas
un autre dictionnaire sélectionné depuis une seconde lecture.

Le pilote garde son pin d'admission (contexte Q ou argument indépendant) ; ses
codecs n'en fabriquent plus un nouveau depuis le fichier découvert. Ses lectures
et données sont confrontées à ce pin. Le loader reste épinglé littéralement dans
les trois préludes ; leurs seuls changements sont ses nouvelles empreintes.
Les captures de bytes de contrôleurs et leur exécution depuis le snapshot sont
conservées. Cette correction ne transforme pas l'import SOURCE en admission Q.

## Capture avant IO, secondaires conservés, cleanup encore tenté

La primitive commune capture_failure conserve l'objet d'exception initial et un
failure_record en mémoire AVANT le premier journal. La clé first_error n'est
pas remplacée, les secondaires restent séparés, remote_quiet est UNKNOWN sans
attestation correspondante dans ces montages. Le record n'est pas une archive
publiée, un succès de fsync, un reçu CP ou une capacité LIVE.

persist_failure protège construction du constat, sérialisation et écriture.
Un OSError du premier journal n'empêche plus abort ; un échec abort est retenu,
puis une panne cleanup-refused reste secondaire. De même, l'échec stop et celui
de local-stop-refused ne masquent pas failure déjà acquis. Les fichiers existants
ne sont jamais réécrits. Quand le disque refuse, le record mémoire et les notes
portées par la vraie exception restent disponibles ; aucune fausse preuve RAW
ou de publication disque n'est fabriquée.

SSHTransport.stop capture le premier échec reap, continue les fermetures de logs
et conserve leurs erreurs comme secondaires ; le contrôle de deadline n'écrase
pas non plus l'objet initial. Les notes secondaires imbriquées sont conservées
lors de la composition jusque Relay. Le même objet est réémis. Les bornes et
limites e15 restent celles déclarées : il ne s'agit pas d'une preuve physique
qu'un callback ou un IO arbitraire serait interruptible dans son budget.

## Portée de validation à venir

Les cinq corps de contre-exemples sont INCHANGÉS depuis6f47. Les36/3/8/10/1
précédents et leurs assertions ne sont pas relâchés. La seule évolution de
sélection est l'ajout des cinq cas au bundle, soit **63** attendus ; cette
sélection n'a pas été importée ni exécutée sur le candidat.

ARRÊT PRIORITAIRE Lucie du 17 novembre : le fichier de nouveaux pins
`publication-snapshot-cause-pins.json` n’a PAS été produit. La préparation,
la revue totale et tout nouveau GO sont suspendus ; voir la passation99.
Une reprise explicitement autorisée devra produire ces pins pour RO TOTALE
M/TTY/C7 avant un GO CPU vert DISTINCT. Ni les anciens GO rouges ni un ancien GO58 ne sont réutilisés.
Un éventuel vert ne qualifiera que les modèles et outils sélectionnés, pas
l'origine native des watchdogs, les six rôles globaux, les SDK, les Q existants,
le producteur d'image d'entrée Archi ou le port CP LIVE. Le ticket reste ouvert.
