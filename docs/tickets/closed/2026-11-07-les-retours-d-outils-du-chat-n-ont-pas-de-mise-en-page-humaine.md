# Les retours d'outils du chat n'ont pas de mise en page humaine

- **État** : corrigé (7 novembre 2026, branche chat-lisible, a03de7d7f) : le texte reçu
  par le modèle est habillé sans rien en retirer, et le bouton « brut » rend le texte exact.
- **Famille** : produit
- **Gravité** : gêne (un résultat d'outil brut, difficile à lire pour un humain).
- **Atteignable en service** : oui, tout outil déplié dans le chat.
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`).

## Ce que c'est

Lucie : « les retours d'outils : quand possible un formatage spécial humain des retours, et que
ce soit lui qui soit vu par les humains ; le texte ne doit pas différer, on doit savoir quand il
manque une info, juste la mise en page peut changer ». Un outil déplié montre aujourd'hui son
résultat brut, dans un `<pre>`.

## Ce qu'il faut pour le fermer

Chaque résultat garde son texte intégral, celui que le modèle a reçu, et reçoit une mise en page
humaine par outil :

- `search_code` : la liste des résultats, avec le chemin cliquable, le score et l'extrait ;
- `read_file` : le code coloré, avec les numéros de ligne ;
- `grep_files` : les lignes groupées par fichier ;
- `list_files` : un arbre.

Un bouton « brut » montre le texte tel quel. La règle : le rendu humain ne retire ni ne résume
rien, il réordonne et habille. Si un champ manque dans le brut, le rendu le dit (« sans
extrait »). Témoins navigateur par outil : le rendu humain contient chaque élément du brut, et le
bouton brut rend le texte exact.

Témoin : `le_retour_d_un_outil_se_lit_et_le_brut_reste_exact`, rouge sur 1bfcb9741 (les lignes
numérotées absentes), vert sur a03de7d7f.
