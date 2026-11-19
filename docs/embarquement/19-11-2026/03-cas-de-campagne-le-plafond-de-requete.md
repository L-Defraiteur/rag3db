# Cas de campagne : le plafond d'une requête se valide à trois caractères par jeton

```json
{"lanceur": {"parent": "91da8c8727211f4a145ed454e226a693251b57df", "tree": "extension/rag3weaver", "needs_lib": true, "witness": "backend::tests::plafond_de_requete_au_dessus_du_contexte_se_refuse_au_chargement", "test": "cargo test --lib --features rag3db-native,code -- backend::tests::plafond_de_requete_au_dessus_du_contexte_se_refuse_au_chargement", "suites": [], "hints": ["extension/rag3weaver/src/backend.rs", "docs/embarquement/18-11-2026/02-bornes-en-caracteres-et-limites-en-jetons.md"], "ours": "dc21ba96dcf1de6456c2c06ca596b887ccb62a9d"}}
```

Session embarquement, 19 novembre 2026, pour la campagne des correctifs passés. C'est un cas très court en Rust, sans
service d'embarquement. **Il demande la lib rag3db** : les tests du backend ne se compilent pas sans `rag3db-native`.
Vérifié le 19 novembre : avec la seule feature `code`, ils s'arrêtent sur « cannot find `Rag3dbConnection` ».

## Le point de départ

- **Commit parent** : `91da8c8727` (18 novembre 2026, sur master). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`, avec la lib rag3db (`RAG3DB_LIB` d'une passe épinglée).

## Le signalement, tel qu'il était

Au point D de `docs/embarquement/18-11-2026/02-bornes-en-caracteres-et-limites-en-jetons.md` (version du 18 novembre)
et dans la ligne 7 de son tableau :

> | 7 | `query_max_chars` (`src/search.rs:987`), validation `query_max_chars ≤ context_tokens × 3` (`src/backend.rs:1516`)
> | caractères, 6 000 | celle du modèle de requête | coupe sur une frontière de mot, avec avertissement | la validation
> est **dans le mauvais sens** : au pire ratio, il faut `≤ context_tokens − 2`. Sans danger tant que le point A tient |
> **Estimer avec une marge** : valider à `context_tokens − 2` (sûr), ou garder ×3 en le nommant « estimation »
> (point D) |
>
> **D. La validation de `query_max_chars`** (borne 7) : `≤ context_tokens − 2`, ou le message dit « estimation à 3
> caractères par jeton ». **Taille : une heure.**

Le pire ratio mesuré, dans la même page : 1 jeton par caractère (accents, emojis, CJK, hexadécimal), plus 2 jetons
spéciaux.

## Le témoin

- **Filtre** : `backend::tests::plafond_de_requete_au_dessus_du_contexte_se_refuse_au_chargement`, dans
  `extension/rag3weaver/src/backend.rs`. Le témoin est identique sur master.
- **Commande**, depuis `extension/rag3weaver`, la lib posée : `cargo test --lib --features rag3db-native,code --
  backend::tests::plafond_de_requete_au_dessus_du_contexte_se_refuse_au_chargement`.
- **Rouge au parent** : `query_max_chars` 8191 pour `context_tokens` 8192 passe encore (la borne est ×3, 24 576).
  L'assertion « a cap above the window at one token per character is refused » échoue. Vu rouge à luciepc le
  18 novembre (lib 3d706f709, passe `query-cap-red`), puis vert (`query-cap-green`, `query-cap-r2-passe`).
- **Suites exigées par les chemins** : aucune (`backend.rs` n'est pas nommé dans `tools/poste/required_suites.txt`).

## Notre correctif (caché à Algareth, pour le juge)

`dc21ba96dc`, « fix(embarquement): un plafond de requête se valide au pire ratio, context_tokens − 2 » :
`extension/rag3weaver/src/backend.rs` +6 −4, `extension/rag3weaver/README.md` +2 −2. La borne devient
`context_tokens.saturating_sub(2)`, et le refus dit « au pire, un caractère vaut un jeton (accents, emojis, CJK) ».
