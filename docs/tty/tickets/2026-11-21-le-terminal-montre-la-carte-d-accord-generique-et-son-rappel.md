# Le terminal montre la carte d'accord générique et son rappel

- **État** : ouvert le 21 novembre 2026 par la session tty. Il attend l'accord générique d'archi (mode auto, étage 3, `docs/rag3weaver-archi/21-11-2026/03-le-mode-auto-contextuel.md`), puis passe avant le lot 2 de l'outil navigateur.
- **Famille** : produit
- **Gravité** : sans lui, le terminal montre une carte vide (il ne lit que `command`) pour tout outil autre que `run_command`, et ne dit rien du rappel.
- **Atteignable en service** : oui, dès que l'accord générique atterrit.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty` : `boite.rs`, `vue.rs`).

## Ce que c'est

Archi publie les lignes de la carte d'accord sur le bus de la tâche, comme toute ligne de boîte :

1. La carte :
   `{"from":"teammate","kind":"approval","id":"L12","tool":"run_command","args":{"command":"git push origin lot"},"class":"outbound","scope":"once","recall":"pending"}`
   - Elle s'affiche tout de suite, avec accepter et refuser. L'outil et ses arguments sont dans `tool` et `args`.
   - `recall: "pending"` veut dire : montrer « recherche d'une autorisation… ». Sans ce champ, il n'y a pas de rappel.
2. Le rappel trouve une autorisation :
   `{"from":"recall","kind":"answer","reply_to":"L12","approved":true,"authorization":{"id":…,"quote":"pousse quand c'est vert","at":…,"turn":"run:…#14"}}`
   - La carte se ferme seule, en disant « validé par ton autorisation du <date> : « <quote> » ».
3. Le rappel ne trouve pas, ou s'efface :
   `{"from":"host","kind":"recall","reply_to":"L12","status":"not_found"|"option_off"|"abandoned"|"citation_unverified"}`
   - La carte reste ouverte, à l'utilisateur.
   - `abandoned` : l'utilisateur a répondu avant, et sa propre réponse prime.

## Ce qu'il faut au terminal

- `Attente` porte `tool`, `args`, `class` et `scope`. La ligne d'état dit l'outil et ses arguments, par exemple `browser_click ref=…` ou `run_command git push …`, et non le seul `command`.
- « recherche d'une autorisation… » s'affiche à côté de la carte tant que `recall` vaut `pending`.
- Une `answer` de `recall` ferme la carte, avec sa ligne « validé par ton autorisation du … : « … » ».
- Une ligne `host`/`recall` laisse la carte ouverte. Sur `not_found`, la ligne d'état le dit, sans bruit dans l'historique.
- `scope: "once"` se dit (« pour cet envoi seulement »), pour que l'utilisateur sache que le « oui » ne vaut pas pour toute la tâche.
- Les témoins sont rouges d'abord, dans `vue.rs` ou un fichier à part, sur les trois formes de lignes ci-dessus. La séance du harnais suit, avec un faux hôte qui les publie.
