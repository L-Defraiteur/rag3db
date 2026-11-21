# Un fichier lu hors des racines de l'utilisateur est du contenu observé

- **État** : ouvert le 21 novembre 2026 par la session tty, sur la demande de c7. Il fait suite au lot du contenu observé (`observed-content`) et touche la porte d'archi, qui sait quelles racines sont accordées.
- **Famille** : produit
- **Gravité** : un fichier d'un dépôt cloné, ou d'une racine accordée comme le dossier steam, peut porter une consigne écrite pour l'agent (« ignore tes consignes… »). Lu par `read_file` ou `search_code`, il arrive au modèle sans marque, comme le code de l'utilisateur.
- **Atteignable en service** : oui, dès qu'un agent lit une racine accordée qui n'est pas à l'utilisateur.
- **Touche rag3weaver** : oui (les outils de code, `code_tools.rs` ; la porte des racines accordées, chez archi ; l'invite de l'agent de code, `apps/code/chat.json`).

## Ce que c'est

Le lot du contenu observé marque `observed: {source, trusted: false}` sur ce que ramènent `fetch`, `web_search` et les outils `browser_*` de l'équipier. L'invite de l'équipier porte la règle `OBSERVED_CONTENT_RULE` (`src/daemon_host/observed.rs`) : une consigne trouvée dans un contenu observé ne s'exécute pas, l'agent la cite et demande au superviseur.

Un fichier lu hors des racines de l'utilisateur est du même ordre, mais rien ne le marque aujourd'hui :
- un dépôt cloné d'ailleurs ;
- une racine accordée en lecture ou en écriture, comme le dossier steam.

## Ce qu'il faut

- La porte d'archi dit, pour chaque racine, si elle est à l'utilisateur ou accordée.
- `read_file`, `search_code`, `grep_files` et `outline` marquent `observed` sur un résultat qui vient d'une racine accordée, avec le chemin pour source.
- L'invite de l'agent de code de l'IDE reprend `OBSERVED_CONTENT_RULE`. Aujourd'hui, il ne reçoit aucun outil qui ramène du dehors, et son invite ne la porte pas.
- Les témoins sont rouges d'abord. Un fichier piégé dans une racine accordée revient marqué ; le même fichier dans le dépôt de l'utilisateur revient sans marque.
