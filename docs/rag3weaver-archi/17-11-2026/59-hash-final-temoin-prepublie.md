# Hash final : témoin causal prépublié sur runner inchangé

Réserve Mémoire5e3b consignée au ticket avant ce diff : les deux sha256 des RAW
sont calculés dans la construction de retour APRÈS le dernier check(final=True).
Une horloge franchissant END pendant ces hashes peut laisser un positif tardif.
Le runner8dd368ca6dc56a08d797998d15763a135b4ecd619409f68c524d70d893177c6f
reste STRICTEMENT inchangé, aucun correctif anticipé.

Un témoin ajouté aux24prépubliés appelle le vrai run_fixture_invocation avec les
mêmes ports modélisés. Il remplace seulement runner.sha256 par le vrai hash qui
avance ensuite l'horloge de fixture à END+1. Le refus attendu est final-deadline,
avec rc enfant0 conservé. Le helper des refus prérelease n'est pas utilisé : ici
le programme était déjà libéré/attendu, seule la publication positive finale
est interdite. Pas de syscall, clock réelle, OS ni faux runner dans le témoin.

Attendu contre le source inchangé :1FAIL «RunnerRefusal not raised», NONOBSERVÉ.
ImportError/erreur de fixture restent ERROR, jamais rouge. Pins proposés pour
UNE méthode timeout58+2, les24autres non sélectionnées ; aucunGOrouge reçu.
Total25fonctions écrites/NONJOUÉES sur ce source. Source/RO/GOdistinct puis
bruts2hôtes avantfix ; correctif ensuite limité au recheck après hashes avant
retour, pas un bornage interruptible des callbacks OS. LimiteTTY maintenue.
