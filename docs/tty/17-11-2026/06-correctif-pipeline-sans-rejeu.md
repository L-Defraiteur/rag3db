# Correctif groupé R1/R2 préparé après rouge réel, sans essai vert

Autorisation C7 préparation après rouge c004 et preuves8850/ROtotale,
suivi908bc8f457. Même ticket anglais OPEN. Les huit témoins et leur SHA256
d998cea23ccb9a92a555134fae983769020cb027da74dd37546bda952144fb6f restent
inchangés ; aucun test exécuté après le rouge, aucun résultat vert déduit.

## R1 : observation pendant l'appel

Le vrai stage conserve subprocess.run et son délai MINreste. Un observateur
lit les journaux pendant l'appel, y compris ceux de la vraie passe canonique.
Première cause unique par verrou : statut Rust FAILED ou diagnostic compilateur
error (texte/JSON), ou ROUGE du résumé canonique. Le texte d'un panic isolé
n'est pas un échec, notamment pour should_panic. Le transport fichier expose
également write pour le backend Python du témoin : même détecteur avant sa
continuation, sans lancer d'enfant OS dans le test hermétique.

Le raccord d'arrêt compose le protocole pidfd du harnais relu : autorité active,
PID/start/UID/cgroup vérifiés, première cause créée exclusivement et fsync,
RAW flush avant TERM de CETTE autorité Common. Le confineur garde le reap,
les résidus et le nettoyage sous END original. Une erreur d'observation n'est
pas un résultat positif ; la fin relit aussi les derniers octets. Aucun reset
ni seconde autorité, ni conversion d'un rouge en succès de scénario.

## R2 : inventaires avant exécution des tests

Les blocs et leurs noms doivent correspondre à un catalogue acquis avant le
jeu, pas au premier résumé positif trouvé dans le RAW. Pour le futur pipeline,
un no-run Cargo sous le jeu exact produit les vrais compiler-artifact et
executable/src_path ; chaque ELF lié au bon src/lib.rs ou tests/e2e/main.rs
est piné puis interrogé par --list/--ignored --list. Cela compile et lance des
ELF en mode catalogue : ces opérations sont NATIVES, proposées seulement dans
une future recette complète ; elles ne sont pas exécutées en préparation.

La phase canonique proposée contient désormais ces catalogues racine large et
navigateur avant le canon2150 inchangé, sous sa même fin MIN2700/END-650.
Catalogue tty vue avant sa lib, j4 ; racine/navigateur j8. Ces travaux ajoutent
un coût encore non mesuré. La borne globale3600 ne s'allonge pas et ne garantit
pas l'achèvement ; aucune autorisation native ancienne ne couvre cet ajout.

La validation exige blocs filets puis suite dans cet ordre, noms exacts sans
manquant/étranger/doublon, compte running et compte final réconciliés, statut
ignored égal au catalogue admis. Les noms à l'intérieur d'un bloc se comparent
comme ensembles : l'exécution parallèle n'impose pas un ordre de fin.
Les tests ignorés de la lib large restent explicitement ignorés dans la preuve,
jamais comptés comme réussis ; les témoins navigateur sont joués séparément.
Les résultats sont liés à l'ELF inventorié, avec la provenance source Cargo.
F8 exige huit jeux exacts dans l'ordre et leurs logs privés ; la preuve de
revérification exige le chemin source APP exact, plus le nom du package.

## Frontières et vérifications

Syntaxe AST des scripts, whitespace et SHA témoin inchangé seulement.
Pas d'import de driver, pas de CPUtest vert, Cargo, modèle, GET, SQL, service,
acteur, slot ou mutation target. Qv1 partiel et Qv2/c003 désarmé conservés ;
le futur Qv3 porte toujours execution_ready=false et admission refusée.
Les HOLDacteurPCLOCAL/politiqueTTY/identités/payload/PGSQL/release ne sont pas
résolus par ce correctif du pipeline. Source/pins puis revue totale et GOCPU
vert distinct avant toute exécution hermétique ; aucune demande GO3600 ici.
