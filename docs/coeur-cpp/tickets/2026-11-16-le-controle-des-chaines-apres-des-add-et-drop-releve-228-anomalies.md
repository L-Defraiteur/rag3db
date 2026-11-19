# Le contrôle des chaînes après des ADD et DROP relève 228 anomalies

- **État** : ouvert (16 novembre2026), constat natif, cause à établir
- **Famille** : moteur
- **Gravité** : valeurs contrôlées fausses jusque après réouverture ; portée réelle à établir
- **Atteignable en service** : à établir, fixture privée de réécriture et changements de schéma
- **Touche rag3weaver** : indirectement, conservation des chaînes et marqueurs
- **Signalé par** : cœur C++, qualification globale888
- **Pour** : cœur C++, coordination C7, hotfixs

## Constat

[Passe et preuves](../16-11-2026/11-qualification-globale-arretee-sur-deux-rouges.md), source/moteur888c81825e4cd6556261233f0e238af3b95375c9, pilote0de1ed3. StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops dure0,973s et relève wrongRows=228 aux trois contrôles after-automatic-checkpoints, after-checkpoint et after-reopen (attendu0), premières clés balayées c-896..c-900. Aucun crash observé. Le comparateur refuse ces étiquettes nouvelles ; aucune suite suivante ni réessai.

Le témoin charge5000lignes, réécrit cinq lots avec ADD/DROP sous lecteur, active auto_checkpoint et seuil4096. wrongRows combine balayage texte/marqueur, cardinalité et sondage par clé : **ne pas traduire228 en228lignes distinctes perdues**. Les valeurs effectivement lues ne sont pas détaillées dans le résultat ; texte, marqueur, oracle ou stockage restent à départager. L'ancien passage ASAN vert14nov de cette forme ne prouve pas une régression du correctif append. Lien avec [C1 crash](2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md) non démontré.

## Suite et fin

[Référence75/59 sans sonde](../16-11-2026/24-reference-chaines-verte-une-fois-sans-attribution-causale.md) : cas unique passé1/1 sous profil du crash, aucune anomalie de valeurs dans cette exécution. Ne localise pas les228 contrôles ni leur rapport au SIGSEGV ; comparaison unique sans attribution au delta append, aucun reclassement ou fermeture.

La [paire diagnostique50e2](../16-11-2026/15-paire-diagnostique-arretee-sur-sigsegv-et-sonde-non-activee.md) s'arrête sur SIGSEGV natif avant résultat complet ; drapeau de sonde effacé par le pilote, donc aucune valeur instrumentée acquise ni localisation du228. Trois autres cas non joués, référence non construite, aucun retry. Deux tickets distincts suivent signal et défaut de pilote, sans attribution au présent défaut.

Relire oracle et opérations/valeurs, puis témoin réduit sous recette et admission neuves si nécessaire. Ne pas modifier known_red, exclure le cas ou fermer C1 pour obtenir du vert. Fin : cause documentée, rouge avant correction, qualification pertinente puis intégration autorisée. Aucun nouveau lourd autorisé par ce ticket.

## Non reproduit le 18 novembre (cœur c++)

Le même test, `StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops`, joué 400 fois à
luciepc sur master `05c54aab8` (avec le correctif des offsets `9fa16affff`), chaque tour dans son
propre processus : tel quel, sans le fil lecteur, sans les DROP, et sous l'environnement de la
qualification de Codex (MAX_NUM_THREADS=2, OMP et OPENBLAS à 5, `taskset -c 0-4`). 400 verts : aucun
`wrongRows`, aucun contrôle `after-automatic-checkpoints`, `after-checkpoint` ni `after-reopen` rouge.
Journaux : `~/.cache/rag3db-moteur-notes/c1-rate/tours.txt` à luciepc.

Les 228 lignes fausses de la qualification de Codex (une passe, source `888c81825` sur base
`68590270e`) ne se revoient donc pas sur master, à 400 tours. Restent possibles un défaut corrigé
depuis (master a reçu depuis le correctif de l'ADD COLUMN refusé à mi-chemin et celui du pool), une
course très rare, ou un écart de son pilote. À garder ouvert une semaine : si aucune passe ne le
revoit, le fermer comme non reproduit.
