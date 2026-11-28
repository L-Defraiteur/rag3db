# L'échec du tour lancé par des notes laissées ne s'affiche pas dans la page

- **État** : ouvert, le 28 novembre 2026
- **Gravité** : message perdu à l'écran : la personne ne voit pas que sa phrase n'a pas été traitée
- **Atteignable en service** : oui (une note laissée à la fin d'un tour, puis un tour suivant refusé : modèle éteint, prix manquant, plafond)
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `start_the_turn_of_left_notes`)
- **Famille** : produit
- **Pour** : recherche
- **Signalé par** : recherche, à la fin du lot `chat-note-mid-turn`

## Le pas exact

Quand le tour lancé par des notes laissées échoue, l'hôte l'écrit sur sa sortie d'erreur et publie `NotesTurnRefused`
sur le sujet `chat` du flux de l'IDE, avec l'id du backend comme clé de `backend`. La page filtre les événements du
flux par `d.backend === backend.manifest` (le chemin du manifeste) : elle ne voit jamais celui-là, et elle ne sait
pas non plus le dire (aucun traitement de `NotesTurnRefused` dans `ui/ide/src/views/Chat.tsx`).

## Pour le fermer

Publier avec le manifeste de l'entrée du chat (`ChatEntry::manifest`, comme `NoteReceived`), et que la page ajoute
une ligne d'erreur au fil, avec le texte de la note qui n'est pas partie. Un témoin navigateur : un modèle qui refuse
le second tour, et la page qui dit pourquoi la phrase n'a pas été traitée.
