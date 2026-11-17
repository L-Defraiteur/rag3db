# Verrou POSTE — préparation d'une observation distincte

Même ticket Recherche302/301. B01 et B-R3 sont clos ROUGE, aucun retry ni
3e essai de compilation autorisé. La sonde apprend les valeurs perdues ;
elle ne contourne pas le garde et ne qualifie pas un transport PG.

Source poste_lock_diagnostic.py, six témoins test_poste_lock_diagnostic.py
publiés avant le code (5d36dc88c9), NON JOUÉS. Les fichiers proc/FD/flock sont
fictifs et privés dans les témoins, Common.poste_tenu reste le vrai module6439.
Le chargement des fixtures ne doit pas faire découvrir leurs six autres tests
implicitement : le module est importé sous un alias, pas une classe TestCase.
Aucun rouge CPU inventé, les incidents B01/B-R3 restent les vrais refus acquis.

La sonde ne dépend d'aucune lib Rust, client PG, metadata compiler, Docker,
SDK ou service ; elle ne lance pas de subprocess. Avant sa collecte : vrai
Common.autorite (signature/ancêtres/cgroup/plafond/temps) puis poste_tenu,
racine attendue externe identique à celle utilisée par le Common. Aucun
changement du garde autorisant de4a/r3 ; les anciens paquets sont immuables.

## Capturer avant jugement

Sous futur GO diagnostique propre : output exclusif, identités superviseur et
GO hors secret, fdinfo FD9 préfixe<=16384octets+truncated, stat/fstat racine,
fiche/poste.lock et FD9 (ouverture RO de la référence proc), ficheSHA<=4096octets,
PID/start/UID/cgroup du poste et table relue avant comparaison, horloges début/fin.
Les tuples attendus major/minor/inode issus de stat et observés issus de fdinfo
sont explicitement conservés, y compris erreur de parsing. Aucun Env complet,
jeton Common ou DSN. Fd9 ouvert RO ne prend ni ne relâche le flock du poste.

Brut fdinfo.raw puis capture.json+SHA sont fsync AVANT comparaison. La comparaison
reste stricte : stat/fstat/UID/type, identité stable, fiche tient, FLOCKADVISORY
READ/WRITE/range, FD9==poste.lock et tuplekernel==tupleStat. Le moindre refus
conserve première cause dans first-red.json et les pièces déjà écrites, sans
nouvelle observation après rouge. Les exceptions de lecture avant collecte
complète conservent leur refus ; aucune capture complète n'est revendiquée.
Un MATCH éventuel n'est ni transport_qualified ni compile_qualified.

## Bornes proposées, pas de GO

Vrai POSTElourd32G puis Common6439 délai10s, timeoutOS12s TERM+KILL2 autour
Common/sonde/publication/retour. Toute E/S de sonde est dans ce processus enfant
Common ; dépassement d'une échéance absolue refuse. Aucune promesse qu'un read
Python bloqué est interrompu au sous-instant : l'enveloppe physique extérieure
et la quiescence finale portent la borne. Pas de phase interne5s ou nouvelle
allocation. Le délai GO conservatif est fixé une fois avant entrée,<=ENDCommon,
sans reset après attente de poste. Premier rougeSTOP, aucune seconde collecte.

Avant ce futur GO : RO totale source/tests/paquet/recette, puis GO CPUdistinct
pour UNEinvocation des six témoins ; lecture live exige encore un autre GO
diagnostique. Les verts18 de4a ne qualifient pas cette source. Aucun slot pris
ni vraie lectureproc/kernel lancée pendant la préparation.

## Paquet diagnostique01 épinglé

Source exacte ae2011135c8c74d06f555716928d5dcdb092d056.
P=`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1`.
PC P/diagnostic-packet-01/diagnostic-packet.json SHA256
`43cf8c537d7ad1fb6081594af08b0df39bc4ac8fc51de404de3892331ef94677`.
Dix sources pinées, dont Common6439 sous ses deux noms réguliers indépendants,
sonde, six témoins et modules de fixtures. Les modules de compilation sont
présents uniquement pour charger les fixtures historiques sans exécution.
La sonde n'importe pas ces modules et n'a aucun appel de compilateur/metadata.

Recette P/diagnostic-preparation-01/recipe.json copiée aux deux postes SHA256
`546986a6b79c1a70941351108600ea7067da513f888980d4617e2b325e079fb4`.
Commande absolue proposée POSTE32G→timeout12sTERM+kill2→Common10s→sonde,
sans-B devant Common, BYTEDONTWRITE par env ; -B autorisé devant la sonde.
GOinactif/runfalse/deadline0/noncezéro et sortie exclusive
P/diagnostic-runs/kernel-lock-01 absente, Commonproofs absentes.
CPU proposé distinct : timeout58s+kill2 python3-Bunittest-v-f
`test_poste_lock_diagnostic` uniquement, sixcas attendus. Aucun CPU joué.

Source+paquet+recette soumis à RO totale avant GO CPU et GO diagnostique
séparés. Aucune lecture live/proc/FLOCK, allocation, compiler ou SQL effectués
pour la préparation ; fichiers r3/B01/OUT_T inchangés. Un futur diagnostic
MATCH/REFUS ne qualifie toujours aucun transport ou compilateur.
