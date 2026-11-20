# Le confineur juge les descendants avant que la session dbus se ferme : un faux ROUGE

- **État** : ouvert (20 novembre 2026, écrit par embarquement, à la demande de G)
- **Pour** : G (optimiseur), ou c7
- **Famille** : produit (l'outillage des passes)
- **Gravité** : basse. Un témoin vert est déclaré rouge, rarement ; le rejeu passe. Mais un rouge sans cause fait perdre
  une passe et la confiance dans le verdict.
- **Atteignable en service** : non, validation des passes seulement
- **Touche rag3weaver** : oui (`scripts/confiner_validation.py`)

## Ce que c'est

Le 20 novembre, à 18 h 50, à luciepc :
- **La passe** : `passe_epinglee.sh embarquement/20-11-2026-self-check self-check-green-a embedder_check temoins` (tête
  86479e3019).
- **Le résultat** : 3 tests passés, commande rendue à 0, et pourtant `validation : ROUGE (descendants laissés après la
  commande)`.
- **Les preuves** (`~/.cache/rag3weaver-build/validations/validation-49t_fkbw`) :
  - `verdict.json` : `"code": 0`, `"restes": []`, `"nettoye": true` ;
  - `enfants.json` : la session `dbus-run-session` (pid 1558975, avec ses enfants 1558977 et 1558983), dans le scope de
    la validation.
- **Le rejeu**, identique, est VERT (`validation-7gexm_dx`).

**La cause, lue par G** : `scripts/confiner_validation.py`, vers la ligne 332. Dès que l'enfant a rendu
(`enfant.poll()`), la garde appelle `groupe.vivants()`, et refuse si un descendant vit encore. Mais la session
`dbus-run-session` se ferme à peine après sa commande. La garde a relevé ses descendants avant qu'ils ne finissent, puis
le nettoyage n'a plus rien trouvé. C'est une course, pas une fuite.

## Ce qu'il faudrait

- Avant de juger, attendre brièvement (1 à 2 s, borné) la fin des descendants de la session de `dbus-run-session`,
  comme le nettoyage le fait déjà. Ne refuser que ceux qui vivent encore après ce délai.
- **Un témoin rouge d'abord** : une commande dont un descendant se ferme 200 ms après elle. Le confineur la déclare
  ROUGE aujourd'hui ; elle doit être VERTE. Une commande dont un descendant vit encore au-delà du délai reste ROUGE.
