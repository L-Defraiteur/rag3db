# Reprendre les périmètres après lecture des diffs récents

- **État** : en cours, 16 novembre 2026 ; bibliothèque et pin livrés, ordre stable intégré d395 ; migration des originaux retenue après dette469 sur copie, API PC rendue, capacité redistribuée.
- **Famille** : produit
- **Gravité** : planification ; reprendre les travaux autorisés sans dupliquer les corrections du week-end ni provoquer des collisions de fichiers ou de ressources.
- **Atteignable en service** : non, coordination des travaux.
- **Touche rag3weaver** : oui, indirectement par ses lots de code, ses validations et ses ressources communes.
- **Pour** : orchestration, avec les treize porteurs de périmètre.
- **Signalé par** : Lucie, le 16 novembre : « on peut attaquer », puis lecture des diffs récents de chacun avant de commencer.

## Travail et preuves

Les treize passations ont été transmises et leur lecture confirmée. Les audits demandés comparent la passation, les changements de contrat du 13 au 16, les tickets et les branches en attente au master `927941a67a5805450d49f1e04b39fdd542a6059b`.

Le [rapport de reprise](../16-11-2026/03-reprise-coordonnee.md) porte la référence, les relevés des verrous et les priorités. L'orchestration attend des constats sourcés pour attribuer les fichiers et ordonner les validations ; elle ne relance pas un test rouge sans diagnostic.

Les audits ont notamment écarté une duplication du témoin ADD COLUMN déjà livré. La retenue initiale des lourds pendant la fenêtre G est historique : TSan fini, API PC rendue après abandon de la fenêtre froide sur dette469. Aucun original migré ; anciennes gates LIVE historiques. Parent bascule enrichi, ticket fils de correctif demandé avant tout diff selon le rappel de Lucie. Cœur a une admission exacte10800 active ; TTY a vu ses deux rouges sous admission600 close ; les autres lots préparent leurs couples surd395 sans ancien GO réactivé.

EDd395 est intégré et son tour fermé après trois contrôles lus aux deux postes. Un défaut nouveau du parseur de garde pile a son ticket ED241c53a5dd, attribué à G avant correction ; prochaines intégrations pile retenues, validations privées permises. Le blocage Archi par observations opaques et les corpus/autorisations manquantes de lucivy restent conservés. Le rapport distingue activité, attente de pilote/revue, dépendance levée et lot livré ; aucun travail n'est créé pour occuper les sessions.

## Critères de fin

- Audits des treize périmètres reçus, écarts et éventuels blocages consignés.
- Premiers lots attribués avec leurs tickets existants, leurs chemins et leurs dépendances.
- File des intégrations et capacité des validations annoncées, sans ancien GO réactivé.
- Une attente ou un simple message de réception ne valent pas réalisation du ticket.
