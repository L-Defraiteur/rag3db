# Sous Windows, une base qui existe ne se rouvre pas en écriture : la garde B2 et le fichier de données se verrouillent l'un l'autre

- **État** : corrigé (12 octobre 2026, cœur C++, 65cb9a1f3) et vu tourner sous Windows : le verrou d'un fichier est pris une fois par processus, sur une poignée à lui (`LocalFileSystem::acquireProcessLock`), la garde B2, le FileHandle des données et le rejeu des pages fantômes s'y joignent ; sous Windows l'octet verrouillé est 2^62, hors des données ; sous Unix flock remplace fcntl. Témoin Linux `FileLockRegistry`. **Preuve Windows** : run 38262590228 (paquet-npm-2 6d5258699, master 0fc7dd10f, windows-x64, lancé par L-Defraiteur) — bâti vert (la branche `_WIN32` compilée par MSVC), épreuve du cycle du démon verte : phase 0 (création), **phase 1 (réouverture d'une base existante par un hôte seul)** et phase 4 (hôte lancé par le client sur la même base), 0 échec ; macOS vert aussi (run 38257008641).
- **Gravité** : blocage — sous Windows, toute base se crée une fois, puis ne s'ouvre plus en écriture
- **Atteignable en service** : oui, sur tout poste Windows, dès la deuxième ouverture d'une base
- **Touche rag3weaver** : oui, par l'hôte du démon et tout backend ; le correctif est dans le moteur

## Ce qu'on voit

Le message est « Could not set lock on file : …\boutique.rag3db (waited 5000 ms for the database
file held by another writer process) ». Aucun autre processus ne tient pourtant la base, et chaque
nouvel essai rend le même refus.

Run 38242681341 (paquet-npm-2 a7c119b1a, windows-x64, lancé par L-Defraiteur),
`tools/build-images/epreuve_demon.py` :

- **phase 0** : un premier hôte (`rag3weaver-backend daemon --address`) crée `boutique.rag3db`,
  l'ouvre pour un client, puis s'arrête par `POST /arret` avec le code 0 : **vert** ;
- **phase 1** : un second hôte lancé à la main, seul (aucun client lanceur, donc aucun second hôte),
  rouvre la même base. Elle reste refusée pendant 90 s, quatre essais du client : **rouge** ;
- même échec en phase 4 (runs 38232395492 et 38236456277), où la base venait de la phase 1.

Sous macOS (même épreuve, même run) et sous Linux (18 OK en local), tout est vert. Sous Linux, un
témoin le confirme : après 20 écritures puis `/arret`, le `.wal` est soldé et la base fermée.
L'arrêt n'est donc pas en cause.

## La cause, par lecture du code

Le run prouve que c'est la réouverture d'une base existante qui échoue, et non le lancement de
l'hôte par le client. Le mécanisme ci-dessous vient de la lecture du code, il n'a pas encore été
vu tourner sous Windows.

`Database::Database` (`src/main/database.cpp`, la garde de f126e0fbf et 6df47a543, lot B2) ouvre,
**si le fichier existe**, une garde `writerGate` sur le fichier de données avec
`FileLockType::WRITE_LOCK`, et la garde ouverte. Ensuite, le `FileHandle` des données
(`src/storage/file_handle.cpp`, `WRITE_LOCK` lui aussi) rouvre le même fichier par une seconde
poignée et le verrouille à son tour (de même `shadow_file.cpp` à la reprise des pages fantômes).

- Sous Unix, `fcntl(F_SETLK)` verrouille **par processus** : la seconde prise du même processus
  réussit, et le défaut ne se voit pas.
- Sous Windows, `LockFileEx` verrouille **par poignée** : la seconde poignée du même processus est
  refusée par la première. La boucle B2 attend alors `fileLockWaitMs` (5 000 ms) un verrou que le
  processus tient lui-même.

Une base neuve n'a pas de garde, puisque le fichier n'existe pas encore : sa création passe. C'est
ce que montre la phase 0.

## Pistes (au cœur d'en décider)

- Sous Windows, que le `FileHandle` des données reprenne la poignée de la garde, au lieu d'en
  ouvrir une seconde qui verrouille ;
- ou que la garde verrouille une autre plage d'octets que le fichier de données (par exemple
  l'octet 1, et non l'octet 0). Un second processus serait toujours arrêté par la garde, et les
  deux poignées du même processus ne se croiseraient plus ;
- dans tous les cas, que le message ne dise pas « another writer process » quand le verrou est
  tenu par ce processus lui-même.

## Pour le fermer

L'épreuve du cycle du démon, verte sous Windows avec sa phase 0 (le workflow `paquet-npm.yml`, job
windows-x64), et un test du moteur qui ouvre, ferme puis rouvre une base en écriture, joué sous
Windows.
