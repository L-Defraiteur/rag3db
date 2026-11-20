# L'équipier invente une identité git dans le bac à sable

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : sérieuse. Un commit de l'agent porte une identité fabriquée, `Lucie <lucied@example.com>` :
  un nom qui n'est pas une signature, et une adresse qui n'est pas la sienne. Atteignable chez Lucie
  dès le premier commit d'un projet neuf.
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui, `src/backend_code.rs` (`avec_lecture_git`) et `src/commande.rs` (la garde)
- **Signalé par** : recherche, scénario 2 (un projet de zéro), la nuit du 20 au 21 novembre 2026 ;
  décision de c7, sans attendre, au titre de la sûreté

## Ce qui s'est passé

Le bac à sable des commandes pose `GIT_CONFIG_GLOBAL=/dev/null`, pour que git ne lise pas les réglages
personnels hors du domaine. Le premier commit a donc échoué faute d'identité. L'équipier a posé lui-même
`git config user.name` et `git config user.email`, par deux cartes accordées, et le commit porte cette
identité inventée.

## Ce qui est attendu

1. Le bac reçoit l'identité git de l'hôte : `user.name` et `user.email` de la configuration globale,
   lus une fois au démarrage. Ils sont passés en `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`,
   `GIT_COMMITTER_NAME` et `GIT_COMMITTER_EMAIL`. Une identité déclarée par le manifeste de l'atelier
   (`sandbox.env`) l'emporte.
2. Une carte `git config user.*`, `git -c user.*=…` ou `git config --global` est refusée en le disant,
   accord humain compris. L'agent n'invente jamais une identité.
3. Un hôte sans identité ne pose rien. Le commit échoue, et l'agent le dit à l'humain.

## Comment le prouver

Rouge d'abord : un commit fait dans l'environnement du bac (`GIT_CONFIG_GLOBAL=/dev/null`) porte
l'identité de l'hôte, et une identité déclarée par le manifeste l'emporte ; une carte
`git config user.email x` est refusée, même accordée par l'utilisateur.
