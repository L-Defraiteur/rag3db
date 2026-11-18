# pas-va-au-bout-3 : le landing, avec un superviseur dans e2e_equipier — le 18 novembre 2026

Branche `pas-va-au-bout-3`, worktree `~/.cache/rag3weaver-build/wt-delegate`. Elle reprend mes deux
commits du 16 (`3cd8e88f0`, `cd53b41bc`), reposés sur master `560b2d403`.

## Conception, en quatre lignes

1. **Le rouge** est celui que Codex a vu le 16, sous Vertex et la lib 59ac (page 04 de sa branche
   `cx-archi-pas-va-au-bout-r2`, ticket `le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur`) :
   l'équipier demande l'accord `rm Cargo.lock`, personne ne répond, et le délai de 900 s tombe.
2. **La cause est le témoin, pas le produit.** e2e_equipier joue le superviseur mais ne répond à
   rien. Une `question` ou une `approval` attend donc, et c'est voulu : la garde B et la priorité
   des accords ne changent pas.
3. **Le correctif** : dans la boucle d'attente du témoin, le superviseur répond à toute `question`
   ou `approval` sans réponse, par une ligne `answer` qui porte `reply_to`.
   - Une `approval` reçoit un refus (`approved: false`), quelle que soit la commande.
   - Une `question` reçoit « décide toi-même et finis la demande ».
   - Aucune commande n'est accordée.
   - Le témoin compte et affiche les réponses données.
4. **La preuve** : e2e_equipier sous Vertex, deux tirages verts, dans le rejeu complet du lot (lib
   entière, témoins du module, e2e_demon_hote, e2e_lectures, e2e_hooks, navigateur), lib 59ac
   épinglée.

Le témoin unitaire de Codex (refus d'abord, puis l'équipier finit) se reprend dans un lot à part.
