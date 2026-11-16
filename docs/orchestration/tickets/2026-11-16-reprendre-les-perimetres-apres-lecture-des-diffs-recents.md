# Reprendre les périmètres après lecture des diffs récents

- **État** : en cours, 16 novembre 2026 ; treize audits reçus, premiers lots attribués, fenêtre de bibliothèque et migrations autorisée par Lucie et prise par G et arbre.
- **Famille** : produit
- **Gravité** : planification ; reprendre les travaux autorisés sans dupliquer les corrections du week-end ni provoquer des collisions de fichiers ou de ressources.
- **Atteignable en service** : non, coordination des travaux.
- **Touche rag3weaver** : oui, indirectement par ses lots de code, ses validations et ses ressources communes.
- **Pour** : orchestration, avec les treize porteurs de périmètre.
- **Signalé par** : Lucie, le 16 novembre : « on peut attaquer », puis lecture des diffs récents de chacun avant de commencer.

## Travail et preuves

Les treize passations ont été transmises et leur lecture confirmée. Les audits demandés comparent la passation, les changements de contrat du 13 au 16, les tickets et les branches en attente au master `927941a67a5805450d49f1e04b39fdd542a6059b`.

Le [rapport de reprise](../16-11-2026/03-reprise-coordonnee.md) porte la référence, les relevés des verrous et les priorités. L'orchestration attend des constats sourcés pour attribuer les fichiers et ordonner les validations ; elle ne relance pas un test rouge sans diagnostic.

Les audits ont notamment écarté une duplication du témoin ADD COLUMN déjà livré et confirmé que les WIP du pas de l'équipier et des ex æquo restent à intégrer. La répartition est inscrite au rapport. Les nouveaux lourds sont retenus pendant la fenêtre prioritaire de G ; cœur garde la maîtrise de son TSan. Aucun tour d'intégration de code n'est ouvert à ce stade.

## Critères de fin

- Audits des treize périmètres reçus, écarts et éventuels blocages consignés.
- Premiers lots attribués avec leurs tickets existants, leurs chemins et leurs dépendances.
- File des intégrations et capacité des validations annoncées, sans ancien GO réactivé.
- Une attente ou un simple message de réception ne valent pas réalisation du ticket.
