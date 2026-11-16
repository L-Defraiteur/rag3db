# La recette ouvrir-bases lit une route status inexistante

- **État** : ouvert (16 novembre 2026), relevé pendant la préparation de bascule
- **Famille** : produit
- **Gravité** : preuve de reprise incomplète, les états des backends ne sont pas lus
- **Atteignable en service** : non (recette opérationnelle)
- **Touche rag3weaver** : oui

`docs/optimiseur/scripts/ouvrir-bases.py` appelle GET `/status` après les ouvertures.
Sur l’hôte posé, le 16 novembre à11:15, cette route rend404 ; le routage de
`daemon_host.rs` sur927 expose GET `/etat`. Le script ignore cette erreur et ne
produit aucun état de backend. Les réponses de POST `/backend/open` restent distinctes.

Pour la bascule, le porteur contrôle `/etat` directement sous authentification,
avec projection des champs état/manifeste et sans affichage du jeton ; reçus dans
`~/.cache/optimiseur-notes/bascule-16nov-927/`. Cela ne corrige pas la recette versionnée.

Suite optimiseur : témoin rouge de recette sur serveur HTTP factice, puis employer
la route réelle et refuser une lecture d’état manquante. Ne pas provoquer de pose
ou d’ouverture réelle pour ce filet ; aucun changement produit requis.
