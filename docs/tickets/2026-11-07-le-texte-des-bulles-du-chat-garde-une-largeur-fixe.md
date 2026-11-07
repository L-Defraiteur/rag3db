# Le texte des bulles du chat garde une largeur de lecture fixe sur un écran large

- **État** : ouvert (7 novembre 2026), retour de Lucie sur la pose de 21 h 51 ; pour hooks-essai,
  après le lot appel-a-delai.
- **Famille** : produit
- **Gravité** : gêne (l'espace perdu, le texte compacté).
- **Atteignable en service** : oui, sur tout écran plus large que la colonne de lecture.
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`).

## Ce que c'est

Lucie : « texte limité en largeur alors que pas nécessaire selon taille écran, faut que ça
s'adapte ». La colonne du chat est fluide depuis 3d09b32c2, mais le texte des bulles garde une
largeur de lecture fixe (`max-w-[100ch]`, sur `.body` et `details.outil`).

## Ce qu'il faut pour le fermer

Le texte d'une bulle prend la colonne : plus de largeur de lecture fixe, ou une largeur relative à
l'écran. Témoin navigateur à 2 560 px : le texte d'une bulle occupe la colonne du chat.
