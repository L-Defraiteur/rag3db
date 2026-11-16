
## Reprise du générateur — lot G autorisé après garde PILE

Branche `cx-optimiseur-return-generator-16`, base df2a578988. Localisation RO :
source réellement utilisée `docs/procedures/scripts/instantanes.sh`, identique à
`~/.cache/rag3weaver-notes-A/instantanes.sh`, SHA
f4ab49a97dea7983af91ddb52523a4a71e4cca4eadd0283313f065ce3d94464e.
Appelée par `docs/procedures/scripts/instantanes-bascule.sh` ; le défaut est bien
la liste développée à la génération de RETOUR, pas le helper V2 déjà qualifié.
Filets existants : cinq tests `bascule/test_restore_snapshot.py` sur le helper,
aucun témoin trouvé du vrai générateur. Aucun code Rust/Catalog concerné.

Périmètre avant diff : fixture de fichiers privés, faux systemctl exclusivement,
invocation du vrai générateur puis de son RETOUR après création d'un nouveau WAL.
Correctif prévu : émettre une recette autonome épinglant le helper V2 et un
inventaire du snapshot, en réutilisant sa conservation dynamique des voisins.
Les snapshots existants, recettes, attestations72h et caches posés restent
inchangés. Aucun moteur, writer CLI, fermeture ou opération API autorisé ici.
