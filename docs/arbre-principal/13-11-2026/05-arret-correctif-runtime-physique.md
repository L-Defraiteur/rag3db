# Preuves de l’arrêt correctif runtime


## Étape corrective distincte : arrêt au premier nouveau rouge

C7 a donné GO exact au pilote e8721a5683166e8ff688e55977bc79cdcf50de11,
application figée 4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF hérités.
Portée DISTINCTE de 3600 s : départ monotone 898110.830219227, échéance
901710.830219227 ; aucune prolongation/réinitialisation de l’ancienne portée.
Aucun Cargo/F8/min/rebuild exécuté. Ledger neuf publié et quatre ELF copiés
avec SHA vérifiés AVANT runtime :
7fe4942919dd2ff3fede289c010be11509b5b1fb026b61c7c1e704bf5b9d209a.

Lib depuis le crate : ancien juge CWD passé ; nouveau premier FAILED :
daemon_host::atelier_fichiers::tests::the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
Arrêt immédiat, durée portée 85.917 s, runtime environ 8 s. Aucun résumé
lib complet, aucun corps de panique publié avant arrêt : cause précise NON
attribuée. Ni rejeu inchangé ni Cargo pour récupérer une panique. NAV/CLI/
lectures/BGE/PG-suites/TX non lancés. Le rouge global reste ROUGE.

Superviseur validation-ej1zgwoe : ROUGE/code1/restes[]/nettoyé ; bus
/tmp/r3v-py3oi02g absent. Scan RO exe/cwd target+deux WT : live[].
Les UNKNOWN restent non concluants sur absence GPU. PG u1 conservée et
PG u1c créée une seule fois, OID1989865 ; aucune suite PG jouée, aucun DROP.
Fixtures restantes conservées, sans ouverture native ni nettoyage.

Preuves PC : cx-passe-raccord-physique-union-complement-v1 ; copie locale
cx-preuves-raccord-physique-union-complement-v1-local/client : 2843 fichiers
identiques SHA au manifeste après arrêt. Superviseur aussi copié localement.
SHA lib.log : 8295e630e40de1f29d5b0a7df6eae5643cdb865e69722a66ce004acbbf3b71a5.
C7/ED poursuivent attribution RO ; aucun défaut production encore établi.
P14 ciblé reste vert dans sa portée ; aucune qualification globale obtenue.

## Diagnostic RO supplémentaire : borne du tampon

La lecture JSON décompressée de la copie
`tmp/lib/.tmpfv48xK/runs/2379c4490f05/graph-1a1d3a704a0-609.json.zst`
montre un NodeRun IndexNode ok=false : le petit dépôt demande au moins
1073741824 octets, le processus déclare 536870912. Le graphe worktree voisin
concerne open_branch ; ce n’est PAS une liaison établie au juge FAILED.
Les quatre graphes conservés ont été lus comme fichiers uniquement, aucune DB.

Contrat statique : estimate.rs64–72 calcule needed_bytes par
text_bytes.div_ceil(TEXT_BYTES_PER_BUFFER_GIB).max(1)*(1<<30).
IndexNode refuse buffer_refusal avant journal et spawn (index_nodes.rs714–718).
Le profil commun 512MiB est donc incompatible avec ce chemin d’indexation.
Cette preuve ne mesure pas le minimum réel du moteur et ne justifie aucune
relaxation de la garde. Correction du profil éventuelle à revoir/autoriser
exactement avant toute nouvelle exécution ; aucun retry lancé.
