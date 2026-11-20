# L'équipier finit un correctif sans porte de vérification

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : sérieuse. L'équipier dit « implémenté et vérifié » sur un arbre qui ne compile pas, et
  l'hôte clôt la tâche (« fait sans plan »). Un humain qui le croit fusionne un arbre rouge.
- **Atteignable en service** : oui, toute tâche de correctif de l'équipier
- **Touche rag3weaver** : oui, la fin d'une tâche de l'équipier (`daemon_host/teammate.rs`) et le gabarit
  de la tâche
- **Signalé par** : banc et c7, au jugement du rejeu de nuit A, le 21 novembre 2026

## Ce qui s'est passé

Le rejeu A (mémoire : une réponse coupée refusée en le disant, depuis 0766144818) a fini en 3 pas et 83
appels au modèle. Le dernier rapport de l'équipier dit les deux volets « implémentés et intégrés ».
L'hôte a clos la tâche sur « fait sans plan ». Le diff (3 fichiers, +41 −4) ne compile pas :
`self.opts.max_tokens.unwrap_or(0)` sur un `usize`, `llm_nodes.rs:283`. Aucun témoin n'a tourné,
alors que le ticket les nommait. La fonction partagée `cut_reply` et un gabarit manquent aussi.
Dossier : `~/.cache/rag3weaver-build/campagne/A-la-reponse-coupee-refusee/20261120-233105` (tablette).

## Le pas exact qui bloque

Rien entre le dernier rapport de l'équipier et la fin de la tâche ne compile ni ne joue un test. La fin
est déclarée par le modèle et acceptée telle quelle.

## Ce qui est attendu

1. Une tâche « correctif » finit par une **porte de vérification** déclarée : compiler, puis jouer les
   témoins nommés par la demande (ou par la tâche), par la garde et dans le bac à sable.
2. Rouge ou arbre qui ne compile pas : le pas continue, avec la sortie de la porte comme suite. Jamais
   « fini » sur un arbre qui ne compile pas. Après les relances bornées, la tâche finit **en échec**, en
   le disant.
3. Un témoin qui exige la lib, quand la porte ne peut pas la fournir, est dit, et le cas est écarté. Il
   n'est jamais compté vert.

## Comment le prouver

Rouge d'abord, par le faux modèle (`fake_sse`) : un équipier qui dit « fini » sur un arbre dont le
témoin échoue reçoit la sortie de la porte et un pas de plus, puis finit en échec si le rouge reste ;
un « fini » sur un arbre vert clôt la tâche.
