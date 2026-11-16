# Scripts de fixture : rouge déterministe puis écriture isolée

Lot `cx-memoire-script-fixtures-r1`, base927941a67a5805450d49f1e04b39fdd542a6059b.
[Ticket](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
Réservation : tests de `src/codex_threads.rs` uniquement ; `livrer` inchangé.

Prépublication rouge `bc07b0cb85` puis correction `40e7f2c37c`, branches poussées.
Le témoin système Linux conserve un writer par un enfant forké, referme le writer
parent puis exige l'exécution : errno26 déterministe. Renommer le même inode ne
supprime pas ce writer. Trois cas en0,005s, un échec attendu sur le lancement.

Les deux scripts fictifs sont maintenant écrits par un enfant `/bin/sh` attendu
au plus5s. Le parent multi-thread ne détient aucun writer du fichier exécutable :
ses autres enfants ne peuvent donc pas en hériter. Aucun retry, changement du
pont, réglage de délai existant, appel au vrai programme ou service.
La commande shell de test reçoit le chemin et le corps comme deux arguments,
sans interpolation dans le programme shell ; le printf est à format fixe.

Le vert système exécute la commande exacte extraite de la constante du helper
Rust : 3/3 en0,007s. Ce n'est PAS l'exécution du helper Rust compilé. Un nouveau
juge Rust de chemin et corps littéraux est écrit mais NON JOUÉ. Les deux juges
historiques, filtre codex_threads, lib entière deux fois, F8 et NAV restent
NON JOUÉS en attendant FINI G et arbitrage de la recette après bascule59ac.
Pas de réemploi de373/lib ancienne ni de budget/GO antérieur.

Journaux horsW conservés :
`~/.cache/rag3weaver-build/journaux-cx-memoire-script-fixtures-16nov/rouge-bc07.log`
et `vert-40e7.log`. Le témoin autonome est borné par timeout20 ; son enfant
attend au plus10s le relâchement. Il ne compile rien et ne touche que son temporaire.

La preuve système établit le mécanisme du descripteur hérité, pas que c'était
l'unique cause historique du rouge sous charge. La qualification Rust ultérieure
reste nécessaire ; ticket ouvert, aucun GO master ni nettoyage.

Dernier témoin système sur `f2e7a6980f` : 3/3 en0,008s, journal `vert-f2e7.log`.
Toutes les preuves ci-dessus sont hors passe Rust et ne valent pas VERT du lot.
