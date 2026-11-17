# Proposition : garde post-writer distincte du readonly

Préparation après les deux RO preuves65eb (Recherche et ED4cb7cbd277).
Ticket : ../tickets/2026-11-17-la-garde-readonly-est-utilisee-apres-le-writer-du-cycle.md.
PROPOSITION NON IMPLÉMENTÉE, aucun nouveau test joué ni GO natif demandé ici.
stable_readers reste inchangé et conserve son contrat readonly.

## Frontière et séquence proposées

Avant une future écriture autorisée : persister l'inventaire COMPLET avant
writer (fichiers et répertoires/modes), l'identité/source/profil exacts, la
base privée canonique et les bornes de l'autorité. Ne pas reconstruire cet
inventaire pour Qv8 : sa variable RAM demeure une limite historique.

Après le vrai writer : exiger les preuves déjà produites par writer(), pas
une simple permission booléenne « autoriser retrait ». Relier process
PID/start/exeSHA/cgroup, profil réel et maps SDK, réponse publique/nonce/run,
SHA publication persistée puis demande shutdown, ACK framed et rc0 du même
enfant attendu. Rejouer le validateur Session sur les lignes persistées est
une vérification pure possible, sans renvoyer aucune commande ni ouvrir la DB.
Une absence/erreur/discordance de preuve refuse ; pas de valeur par défaut.
Les fichiers de preuve locaux sont des observations du lanceur, sans prétention
de certificat anti-forge contre un autre acteur du même UID.

Toujours vérifier END/premier rouge avant/après collecte et sauvegarde des
inventaires. Une recette RO séparée pour examiner Qv8 serait un autre contrat,
avec un GO distinct : pas de prétendue reprise verte du cycle déjà rouge.

## Deltas physiques admissibles, sous toutes ces préconditions

- DB principale : présente avant et après, régulière, mode conservé, taille
  positive ; SHA/taille avant/après consignés, changement permis seulement
  comme delta physique du writer, sans conclusion d'intégrité logique.
- WAL exact et marque .ecrivain exacte : présents ou absents avant ; absents
  après shutdown. Aucune permission générale de suppression. Un WAL résiduel,
  même inchangé, refuse avant tout lecteur froid ; une autre variante/suffixe
  ou un temporaire reste un ajout interdit.
- Tous les autres fichiers : mêmes chemins/modes/tailles/SHA, y compris FTS,
  extensions et métadonnées annexes. Suppression ou modification refuse.
- Exception readonly existante .readers : seulement ajout initialement absent,
  vide/mode0644/SHA vide ; un fichier déjà présent reste strictement identique,
  aucun retrait/modification autorisé. L'absence de ce fichier ne prouve pas
  la quiescence : conserver le vrai contrôle FD/exe/cwd attribué séparé.
- Répertoires/modes : identiques. Doublons de chemins, inventaire incomplet,
  entrée non régulière/symlink/protégée ou champ mal typé refusés, pas d'écrasement
  silencieux par dict comprehensions.

Le retrait de la marque est cohérent avec marque_d_ecrivain::Garde::drop qui
retire seulement sa propre marque ; cela ne prouve pas que CE retrait Qv8
est causé par ce Drop. De même, aucun raisonnement WAL ne remplace un juge
logique ni n'attribue la cause native du changement de SHA DB.

## Juges séparés après admission physique

Quiescence privée fraîche puis lectures froides autorisées dans la même
fenêtre : bits des97247 vecteurs et marqueurs/claims, graph/config/générations,
cinq FTS (IDs/fichiers/générations) et dette0, sources strictes et dernière
quiescence. Les contrôles de lecture readonly utilisent stable_readers sur
le NOUVEL inventaire post-writer, sans effacer les deltas de l'écriture.
Un seul écart refuse le cycle entier ; finaliseur END/premier rouge inchangé.
Ces suites Qv8 sont NON JOUÉES : aucune projection rétroactive de leur succès.

## Témoins CPU à prépublier avant correctif

Le harnais doit extraire le vrai bloc post-writer de lifecycle.driver et
appeler les vrais validateurs ; seuls inventaires, horloge et observations
du processus sont synthétiques. Aucun autre modèle de la garde dans le test.
Contrôle positif readonly intact et cas post-writer avec retrait exact des
deux auxiliaires ; ce dernier doit voir la garde readonly actuelle refuser.
Capturer le refus puis juger le contrat pour un FAIL explicite, pas ERROR.

Mutants à couvrir sur ce même raccord : suppression FTS ou fichier arbitraire,
octets/mode FTS altérés, DB absente/mode modifié, WAL conservé ou nouveau,
marque résiduelle, fichier/répertoire ajouté, .readers non vide/modifié/retiré,
doublon de chemin, preuve ACK absente/false, rc non nul, mauvais PID/start,
nonce/run/publication SHA discordants, profil/maps non attestés, deadline
dépassée/premier rouge intervenant pendant la collecte. Un faux fix qui
ignore tous les retraits, ou qui saute la garde post-writer, doit être refusé
par ces juges ; un refus préalable ne qualifie pas une branche ultérieure.

Ces témoins sont spécifiés ici, NON ÉCRITS/NON JOUÉS. Après revue du contrat,
publier le vrai harnais avant correction et demander son GO CPU exact distinct.
Puis correctif minimal du raccord, publication, RO/GO CPU vert distinct et
paquet neuf/deux RO totales avant toute décision native. Aucun nettoyage,
DB/service/health/writer/recovery/SDK/Cargo/target autorisé par ce document.
