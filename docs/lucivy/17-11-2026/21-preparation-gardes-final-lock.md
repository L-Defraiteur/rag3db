# Gardes final-lock groupées — préparation après le rouge

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Les deux RO v3 sont regroupées/réservées sur P1FINALLOCK. Le vrai contre-exemple
est observé et conservé deux postes37beb72055 **avant** ce correctif : drift
accepté par166d, contrôle inchangé accepté,1FAIL/1PASS/0ERROR. Aucun drift réel
du lock PC allégué ; Qv3/seeddbdb/T et trois vieux ELF restent intacts.

## Un patch groupé, deux observations dans les END existants

`qualify-fs-admission-red-v4.py` SHA
**4d37a7f3ec4a2c90bbdc4286ea1da10df394147611baed4a0bea4de38f31403d** :
après les sept juges et toutes les autres vérifications, checked_path puis
SHA du **W/Cargo.lock effectif** contre LOCK_SHA dbdb, budget avant/après,
immédiatement avant result.json. Le résultat publie final_lock_sha256 et
lock_invariant. Aucun nouveau délai, exception ou restauration du fichier.

`admit-fs-admission-red-v4.py` SHA
**d50856ccf19e1be02ba2a6c6e93fe4c88c6f236e10b70e8f0d4609265348647b** :
après Common réellement revenu et après les autres rechecks, checked_path et
SHA du même lock effectif contre copy_hashes[Cargo.lock.seed], puis vérification
du deadline et final-accepted. Celui-ci publie effective_lock_sha256 et
lock_invariant. L'assertion nomme précisément `effective Cargo.lock changed
before final acceptance`. Le seed reste lui-même rehashé par la boucle des
copies figées ; Gitclean ne remplace jamais le hash du fichier ignoré.

Ces deux points couvrent fin du worker puis dernier verdict après Common/
cleanup. Ils composent les primitives existantes sha/checked_path/budget/
publish. Pas de troisième relais, résolution Cargo ou exception large.

## Probes préparés, aucun vert nouveau

- `test-fs-final-lock-entry-v2.py` SHA
  **33aebb44da043a188b4990e7262b95d647efedd8e6a8e65a4739a58d09ba9693** :
  même vrai main, setup/observation/deux assertions que le rouge. Seul le pin
  d'admission devient un argument obligatoire pour viser les nouveaux octets.
  Prévu surd508 : deux PASS (drift refusé avec cause exacte, inchangé accepté).
  Common/Git/proc restent modèles, aucun Cargo/ELF réel.
- `export-fs-red-ledger-v4.py` SHA
  **7c58b87f74781019774a3be48b363e19f9620a979d8b026df0b118c84f56f8bd** et
  `test-fs-export-boundaries-v5.py` SHA
  **dc0dd7a3a1e8a126d1ef11d828dc94ccc11584ac53a2742d566c2eba692f2006** :
  les mécaniques START/END/ordre/durée/cleanup ne changent pas depuis les
  propositions ec619/d13 ; seuls les chemins Qv3→Qv4 et le manifeste de fixture
  suivent le nouveau paquet. Trois PASS proposés, aucun encore observé.

Q CPU final-lock : `~/.cache/rag3weaver-build/cx-lucivy-fs-final-lock-green-17nov-v1`,
UID1000/dev57/inode96720614 ; [pins](preuves/final-lock-v4-cpu-preparation/preparation.json).
Q CPU journaux : `~/.cache/rag3weaver-build/cx-lucivy-fs-export-journal-17nov-v2`,
UID1000/dev57/inode96720620 ; [pins](preuves/journal-v4-cpu-preparation/preparation.json).
Sources figées, SHA/AST seulement. Aucun import/runpy/test/sourceCpp/Rust/proc/
SSH/signal/processus par les fixtures, aucun corpus/target. Les deux futures
invocations ont besoin de revue et GO CPU exact distinct, sans vieux GO réarmé.

Les invariants du paquet complet et sa commande sont [22](22-paquet-fs-red-v4-final-lock.md).
Deux RO totales puis GO300 distinct avant tout Cargo ; source Rust0f reste
non compilée et ses six négatifs FS/positif non exécutés.
