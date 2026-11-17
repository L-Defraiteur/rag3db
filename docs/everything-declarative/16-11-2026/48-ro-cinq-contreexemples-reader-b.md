# RO ED — cinq contre-exemples avant correction B

Référence3240c0a4693aecabf45ddafbeadd5547752766b2. Avis favorable à la
préparation des cinq rouges, pas GOCPU ni B. Aucun témoin exécuté par ED.
Les trois hashes du red-boundary-pins recoupés aux blobs Git ; deux corps
reader_runtime/reader_exec strictement inchangés depuis4e3, cinq noms AST
égaux à la liste sélectionnée. Python/timeout sont des pins locaux annoncés,
pas réattestés physiquement dans cette RO. Les douze autres non sélectionnés.

Causalité relue : pin lecteur figé avant substitution, faux capture reflète
l'ELF courant (seul attendu indépendant peut détecter promotion) ; hash et
open font avancer horloge au-delà de END sans vraie exécution ; double now
100→101 doit montrer deadline186 au lieu de185 ; command.json fait franchir
END avant subprocess. Dans ce dernier cas ValueError tardive ne suffit pas :
assert zéro départ détecte le faux enfant déjà invoqué. Les échecs attendus
sont donc des assertions de contrat, pas des erreurs de module manquant.
Fake typed consume volontairement neutre pour ne pas confondre pin/budget et
framing. execvpe et dup2 remplacés, fichiers/temp/hash réels seulement, aucun
GDB/native/DB/proc/service. Patchs restauration contextuelle, références hash/
open originales conservées avant patch, pas récursion ajoutée.

R1/R2 sont fondées : première query promeut actuellement SHA observé en attendu,
et reader_exec ne revérifie pas l'échéance après hash/open ; first_query utilise
deux lectures now et n'a pas de contrôle juste avant subprocess. Ces bords
avaient échappé à mon avis précédent rapport47, qui demeure historique.
4e3/Q1d144 retirés du GO ; aucune permission de B ne survit à cet avis.

Cinq FAIL/0ERROR/0skip/rc1 sont une attente statique, pas un résultat acquis.
C7 doit donner un GO exact CPU local60/TERM58/KILL2 sans failfast avant run,
archiver les vrais rouges puis publier correction groupée/nouveaux pins/Q et
soumettre l'ensemble. A/anciensQ/rouges restent conservés ;17écrits NONJOUÉS
à ce stade. Parent ouvert, aucune exécution ni nettoyage déduit.
