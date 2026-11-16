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

## Rebase après FINI G et pin59ac

Branche NEUVE `cx-memoire-script-fixtures-r2`, base
`68590270e21d00285b29800bd9cdecf0cb921461` ; code repris à
`4e0d812f28498589224b022837f0cc51461445d1`. Quatre patches range-diff égaux,
cinq chemins portés mode/OID identiques r1 ; tout le reste identique à la base.
W local `~/.cache/rag3weaver-build/cx-wt-memoire-script-fixtures-r2`, propre.
Ancienne branche r1 conservée, aucun rebase poussé sous son ancien nom.

C7 a annoncé FINI G puis pin FINI : cache passe_epinglee des deux postes
SHA2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d,
mode755, témoins de copies verts. Lib nouvelle épinglée
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
Ce lot ne modifie aucun moteur, aucun MOTEUR_ANCIEN requis ni hérité.

Recette proposée à C7, **NON LANCÉE et pas un GO** : après Recherche et Archi,
préparer un W PC depuis r2 et le gitlink exact, target privé unique
`target-cx-memoire-script-fixtures` (neuf,32Go max), POSTE_MEM_MAX32G/j8.
Fenêtre neuve proposée7200s après acquisition réelle : passe épinglée canonique
SUITE_DU_TEMOIN=lib, filtrecodex_threads, SUITES=- (diff test seul ; aucune suite
code_sync requise par chemin), puis seconde lib entière sur le même code/target.
La passe donne déjà la première lib entière, F8/preuve et NAV : pas de second
F8/NAV ni suites mémoire à services. Le second passage lib est demandé pour
le caractère intermittent historique. Rester dans la fenêtre globale, arrêter
au premier rouge inattendu, lire nombres joués/ignorés et garder tous les logs.
Résumé canonique de passe inchangé ; preuve de seconde lib séparée, jamais
fabriquer un résumé vert par assemblage. `run_e2e.sh --lib` remplace les anciens
helpers du cache. Le créneau, la durée et les accès aux services éventuellement
utilisés par la lib/NAV attendent encore l'accord exact C7.
