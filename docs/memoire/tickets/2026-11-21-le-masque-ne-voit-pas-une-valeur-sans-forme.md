# Le masque des secrets ne voit pas une valeur sans forme

- **État** : ouvert, le 21 novembre 2026.
- **Famille** : produit
- **Gravité** : un mot de passe, ou une clé maison sans préfixe connu, sorti par un outil ou écrit en mémoire, reste
  lisible. La règle de Lucie (« affichage de secret toujours interdit quoi qu'il arrive ») n'est donc tenue que pour
  les formats reconnus.
- **Atteignable en service** : oui. Tout secret sans forme reconnue que lit un outil, ou qu'un appel écrit en mémoire.
- **Touche rag3weaver** : oui. Le masque (`src/script/secrets.rs`, `mask_secrets`), son point unique dans la boucle
  d'agent (lot 5b), l'écriture en mémoire (`writes.mask_secrets`, lot 5a), et le coffre (`secret_store`, d'archi).
- **Pour** : archi, qui le ferme en branchant son coffre ; la session mémoire (96) tient le masque.
- **Signalé par** : c7, le 21 novembre 2026 (v1 du mode auto, page 07 de rag3weaver-archi, lot 5).

## Ce qu'on voit

`mask_secrets` ne reconnaît que des motifs déclarés : un préfixe connu suivi d'un corps assez long (`ghp_`, `sk-`,
`AKIA`…), un jeton `Bearer`, un bloc de clé privée PEM. Une valeur sans forme reconnaissable, comme un mot de passe ou
une clé maison, passe telle quelle.

## Ce qu'on attend

Le masque prend aussi les valeurs connues du coffre de la session (le `secret_store` d'archi, et les `.env` qu'il
charge). Il les compare sans jamais les écrire ni les journaliser, au besoin par empreinte pour une grande liste. Le
masque garde un crochet pour ces valeurs, `mask(texte, motifs, valeurs)`, qui attend le coffre pour être rempli.

## Témoin attendu (rouge d'abord)

Une valeur sans forme reconnaissable, posée au coffre de la session, puis sortie par un outil et écrite dans une
fiche : ni le modèle, ni le journal, ni le flux, ni l'IDE, ni la ligne de mémoire, ni son miroir `.md` ne la voient.
Tous lisent `[masked secret: <nom de la clé du coffre>]`.
