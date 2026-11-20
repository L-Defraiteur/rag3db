# L'agent invente une identité git, que le bac à sable des commandes cache

- **État** : ouvert (21 novembre 2026, session recherche, scénario 2 de la nuit).
- **Gravité** : identité. Un premier commit fait depuis l'IDE porte une identité inventée par l'agent
  (`Lucie <lucied@example.com>`), écrite dans la config locale du dépôt. L'identité git est une affaire légale pour
  Lucie (jamais l'adresse pro, l'adresse perso partout).
- **Atteignable en service** : oui. Tout commit depuis l'atelier, puisque le bac à sable pose `GIT_CONFIG_GLOBAL=/dev/null`
  (`src/backend_code.rs`, l. 659).
- **Touche rag3weaver** : oui (le bac à sable des commandes, le prompt de l'atelier).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

Scénario 2 de la nuit du 20 au 21 novembre : un développeur qui part de zéro, dans l'IDE d'un hôte isolé bâti de
master a60f9f7a71 (HOME à lui, avec un `.gitconfig` « scenario <scenario@localhost> »). Dans le chat de l'atelier :
« Crée le projet et son dépôt git (sans rien pousser) ». L'agent écrit les fichiers, demande `git init` puis
`git add .`, puis `git commit -m …`. Le commit échoue faute d'identité : le bac à sable cache la config globale.
L'agent demande alors, carte après carte, `git config user.name "Lucie"` et `git config user.email "lucied@example.com"`.
Accordées, elles écrivent cette identité dans `.git/config`, et le commit `16a017f` la porte (auteur et commiteur).

## Pourquoi

- `GIT_CONFIG_GLOBAL=/dev/null` est voulu : git ne charge pas les réglages personnels hors du domaine. Mais l'identité
  en fait partie, et rien ne la transmet autrement.
- L'agent ne demande pas qui commite ; il tire un nom du contexte et fabrique une adresse. La carte d'accord montre la
  commande, sans dire que l'identité est inventée.

## Le contournement

Refuser les cartes `git config user.*`, et donner son identité dans le chat (ou la poser soi-même dans le dépôt).

## Pour le fermer (décision à prendre, pas un petit lot)

- **L'identité transmise au bac à sable** : `GIT_AUTHOR_NAME/EMAIL` et `GIT_COMMITTER_NAME/EMAIL` lus par l'hôte dans
  la config globale de l'utilisateur, ou une identité déclarée dans le manifeste de l'atelier. À trancher par Lucie.
- **L'agent ne fabrique jamais une identité** : il demande, et la carte d'un `git config user.*` le signale.
- Un témoin : un commit depuis l'atelier porte l'identité déclarée, jamais une inventée.
