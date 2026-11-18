# Le témoin documentaire Vertex attend un accord sans superviseur

- **État** : fermé le 18 novembre 2026 par `9057cad4e` (pas-va-au-bout-6, sur master) ; e2e_equipier vert sous Vertex, deux tirages, dont un avec l'accord `rm Cargo.lock` refusé puis la fin.
- **Famille** : produit
- **Pour** : archi
- **Gravité** : bloque le landing de `pas-va-au-bout` : e2e_equipier est une suite exigée pour tout lot qui touche l'équipier.
- **Atteignable en service** : non, c'est le témoin qui ne joue pas son rôle de superviseur ; le produit attend un accord comme il le doit.
- **Touche rag3weaver** : oui, `tests/e2e_equipier.rs`
- **Signalé par** : Codex, dans le rôle d'archi, le 16 novembre 2026. Rapport 04 et première version de ce ticket sur `cx-archi-pas-va-au-bout-r2` ; version relue par le banc sur `cx-banc-lucivy-indexed-ro-20261116`.

## Ce que c'est

Codex a joué `e2e_equipier::l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges` sous Vertex,
avec la lib 59ac et les deux commits de `pas-va-au-bout`. Le test a échoué sur son délai interne de
900 s :
1. l'équipier (Gemini) demande l'accord `rm Cargo.lock` ;
2. il rend compte du README écrit ;
3. il attend la réponse du superviseur.

Le témoin joue le superviseur (`c7`) pour la demande, mais il ne répond jamais à une `question` ni à
une `approval`. L'attente est voulue : la priorité d'un accord et la garde B sont le contrat. Le
défaut est donc dans le témoin. La sortie d'un vrai modèle varie, et le même témoin était vert le 16
au matin sur `6b672afa6` avec la lib 373b5477b.

## Ce qui ne se fait pas

- Accorder `rm` ou toute autre commande.
- Filtrer le témoin.
- Rejouer jusqu'à obtenir un tirage sans demande d'accord.
- Une fixture qui ne refuse que `rm Cargo.lock` (la version de Codex, jamais jouée) : elle rougirait
  sur toute autre demande.

## Le correctif (pas-va-au-bout-3, f20762821)

Dans la boucle d'attente du témoin, `answer_pending` répond à toute `question` ou `approval` de
l'équipier qui n'a pas de réponse :
- une approval reçoit un refus (`approved: false`), quelle que soit la commande ;
- une question reçoit « décide toi-même et finis ».

Le témoin vérifie qu'aucune ligne ne porte `approved: true`, et le journal compte les réponses
données. La conception est à la page `docs/rag3weaver-archi/18-11-2026/02-pas-va-au-bout-3.md`.

## Fin

e2e_equipier vert sous Vertex et la lib épinglée, deux tirages, dans le rejeu complet du lot. Puis le
lot est poussé sur master, et ce ticket est fermé avec le hash.

## Fermeture

Poussé sur master le 18 novembre 2026 : `4b9926dea`, `09ed83d33`, `9057cad4e`. e2e_equipier a été joué sous Vertex sur `f20762821` (base `560b2d403`, lib 59ac épinglée), en deux tirages à 2/0. Au tirage 1, Gemini a demandé l'accord `rm Cargo.lock` : le superviseur du témoin l'a refusé, puis l'équipier a fini (« fait sans plan », 2 pas, 41 s). Au tirage 2, aucune demande d'accord. Le rejeu court sur la tête poussée a été joué à luciepc.
