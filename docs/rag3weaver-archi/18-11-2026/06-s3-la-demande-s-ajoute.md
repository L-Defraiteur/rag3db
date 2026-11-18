# S3 : une request de suite s'ajoute à la demande ouverte — le 18 novembre 2026

Branche `la-demande-s-ajoute`, worktree `~/.cache/rag3weaver-build/wt-delegate`. Ticket :
`docs/rag3weaver-archi/tickets/2026-11-16-une-request-remplace-la-demande-en-cours.md`.

## Conception, en quatre lignes

1. **La demande ouverte, ce sont les requests du superviseur depuis la dernière ligne `ended`**
   (ou depuis le début de la boîte). Quand il y en a une seule, rien ne change. Quand il y en a
   plusieurs, la frontière dit `Request (opened at <at>): <la première>`, puis une ligne
   `Follow-up: <texte>` par request suivante, dans l'ordre. Une request reçue après `ended` ouvre
   une demande neuve.
2. **Les pas faits** rappelés à la frontière se comptent depuis la première request ouverte, et
   non plus depuis la dernière : un « continue » ne fait plus oublier les `step_done` d'avant.
3. **Ce qui ne bouge pas** :
   - la garde B, qui suit toujours chaque request (chacune attend sa ligne) ;
   - le format de la boîte (le `kind` reste `request`) ;
   - `Step.request` et le nom de la fiche de session, qui prennent déjà la première.
4. **La preuve** : un rouge d'abord sur le faux modèle. Une request « Write a test for bonjour »,
   un pas sans fin, puis une request « continue ». La frontière du pas suivant porte les deux
   textes, la première d'abord, et le `step_done` du premier pas. Puis le module, la passe
   complète et e2e_equipier sous Vertex.
