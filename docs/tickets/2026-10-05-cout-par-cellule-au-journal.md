# Au journal, chaque cellule réécrit son type

- **État** : ouvert — confort, après le basculement du `COPY` journalisé (classé par l'orchestration, 5 octobre 2026)
- **Gravité** : volume et temps (journal plus gros, validation et rejeu plus longs) ; aucune réponse fausse
- **Atteignable en service** : toujours
- **Touche rag3weaver** : oui, en volume de journal
- **Ouvert le** : 5 octobre 2026, session cœur C++
- **Pour** : cœur C++

## Ce que c'est

Une ligne va au journal comme une suite de `Value` : chaque cellule y réécrit son type, sa nullité et son nombre d'enfants avant son contenu (`ValueVector::serialize`, `Value::serialize`). Les tableaux de numériques ont reçu une forme brute (`1c232f318`) ; les autres cellules gardent ce coût fixe.

## Mesuré

- Octets de journal, côté moteur : une relation sans propriété, 75 (trois identités internes, soit environ 25 par cellule) ; avec deux chaînes courtes, 143 ; une ligne clé + texte de 200 caractères, 270.
- Première indexation du dépôt (62 Mo de texte), `COPY` journalisé, plein texte en fichiers : 466 Mio de journal. Par table : les relations 168 Mio (1 111 300 lignes, 158 octets par ligne) ; `Scope_Chunk` 137 Mio et `Scope` 129 Mio, où le texte est écrit deux fois (un fait de schéma, environ 124 Mio) ; le reste, environ 175 Mio, est le coût par ligne hors texte (`Scope` : environ 900 octets par ligne au-delà de son texte ; `Symbol` : 329 octets par ligne).

## Le correctif envisagé, non fait

Étendre la forme brute aux colonnes à largeur fixe (entiers, flottants, dates, identités internes : une colonne = ses octets bout à bout) et aux chaînes (longueur puis octets), sous le même mécanisme que la forme compacte des tableaux : une seconde génération de numéros d'enregistrement, ou un octet de forme par vecteur, l'ancien décodage gardé. De l'ordre du lot de la forme compacte.

Gain attendu, **par calcul, non mesuré** : les relations divisées par deux à trois, le hors-texte des nœuds par deux ; 466 Mio vers 300 environ.

## Pour le fermer

La forme, un témoin de parité au bit près, un journal de la forme d'avant gardé lisible, et la mesure avant/après sur la même indexation.
