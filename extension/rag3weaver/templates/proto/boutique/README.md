# Boutique — le backend jouet du proto « tout déclaratif »

Trois entités (`Category`, `Product`, `Purchase` — une commande ; pas `Order`, mot réservé de Cypher, voir le ticket du 11 octobre 2026), aucun embarquement, des outils
génériques liés par le manifeste, deux nœuds scriptés (`nodes/`), trois vues
(`views/`) et des routes. Tout ici est une déclaration : rien ne se compile.

```text
rag3weaver-backend serve --manifest templates/proto/boutique/backend.json
```

- `GET /products` — la liste (une vue) ; `POST /products` — le formulaire,
  par `ProductForm` ; `GET /products/{key}` — la fiche, par `Identity`.
- `GET|POST /categories`, `GET|POST /orders` — en JSON (sans vue).

Le chemin `vector_extension` est relatif à ce dossier dans le dépôt ; une
copie ailleurs le réécrit (ou pose l'extension à côté).
