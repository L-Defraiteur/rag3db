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

La [paire diagnostique50e2](../16-11-2026/15-paire-diagnostique-arretee-sur-sigsegv-et-sonde-non-activee.md) s'arrête sur SIGSEGV natif avant résultat complet ; drapeau de sonde effacé par le pilote, donc aucune valeur instrumentée acquise ni localisation du228. Trois autres cas non joués, référence non construite, aucun retry. Deux tickets distincts suivent signal et défaut de pilote, sans attribution au présent défaut.

Relire oracle et opérations/valeurs, puis témoin réduit sous recette et admission neuves si nécessaire. Ne pas modifier known_red, exclure le cas ou fermer C1 pour obtenir du vert. Fin : cause documentée, rouge avant correction, qualification pertinente puis intégration autorisée. Aucun nouveau lourd autorisé par ce ticket.
