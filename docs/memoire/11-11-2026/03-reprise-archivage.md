# Reprise mémoire du 11 novembre : le 400 de l’archivage

Le contact avec orchestration-sessions est établi depuis la session mémoire `01a1cb5c-4f39-7421-9a9f-4aa288d69402`. Les rapports précédents et les six tickets actuels ont été lus. Le lot des refus 4xx a déjà atterri sous `2d6a08a65` : ne pas relancer son ancien script de push.

## Essais réellement joués

Le premier document figé (`06-novembre-2026-orchestration/01-plan-du-week-end.md`) a été rejoué sur des copies neuves distinctes de l’instantané avant-29a7bc52a. Les fiches du juge sont incluses, `allow_create=true` ne touche que les copies. Aucun changement de l’hôte ni de sa mémoire en service.

- Binaire ancien copié, provenance non certifiée, SHA256 `ff61a5b97f6b8ec119b4f4f15928828a3e2202ccc2254b84ba1be51ee19c92ad` : **2/7**, cinq HTTP 400, 44,8 s.
- Même binaire et alias courts `candidate_N` dans les étiquettes du Decide, avec noms et descriptions conservés dans le prompt et table inverse pour les écritures : **7/7**, zéro refus, 86,8 s. Diagnostic provisoire.
- Binaire release fourni par optimiseur, provenance `d1345a115a8044c5b5f5947e26d5aaba8d340053`, SHA256 `597f6a29de486a9161996e8e4f460731802e090c71f0859a7ad80808ad7da7ee`, gabarit original : **2/7**, cinq HTTP 400, 46,9 s. Chaque refus garde effectivement sa requête et le corps complet.

Les maps des deux processus originaux ont confirmé `librag3db.so` et `libvector.rag3db_extension` depuis `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`. La copie du premier binaire avait besoin de `LD_LIBRARY_PATH` vers cette lib ; son premier échec de chargement est conservé à part, sans être compté comme un essai de l’archiviste.

Les durées sont celles du processus entier (ouverture, recherches, modèle et écritures). Elles ne sont ni des mesures exclusives sous `poste mesure`, ni des durées par section. Ne pas comparer 46,9 s avec 86,8 s comme un gain ou une régression : cinq appels sont refusés dans le premier cas.

## Comparaison achevée avec le binaire vérifié

Sur une quatrième copie neuve, le même binaire d1345a115 avec les alias courts traite **7/7 sections, zéro erreur, zéro requête refusée, en 113,5 s** au total. Les noms et descriptions des fiches du juge restent présents dans le prompt. Les maps de ce processus confirment également la lib épinglée complète ; preuve `validation-alias-d1345a115/lib-chargee.txt`.

La comparaison avec le gabarit original (2/7, cinq refus) reproduit donc le diagnostic sous un binaire de provenance connue. C’est un vert de l’essai isolé, pas une passe de production ; le changement n’existe que dans le gabarit temporaire. Le ticket reste ouvert jusqu’au correctif et à ses témoins.

## Diagnostic et reste

Les enum refusés contiennent les noms des bilans, de 1 811 à 2 371 caractères. Vertex renvoie seulement INVALID_ARGUMENT, sans nommer le champ ni un seuil. L’essai d’alias change les valeurs enum et la présentation des choix ; il appuie cette piste sans établir une limite précise du fournisseur.

Ticket ouvert : `docs/memoire/tickets/2026-11-11-les-noms-des-fiches-du-juge-font-refuser-les-decisions-de-l-archiviste.md`. Aucun correctif de production ni passe complète n’est livré par cette note. Le prochain lot doit poser un témoin sur les identifiants courts, leur résolution vers le véritable nom, les noms réservés (`none`, `create`) et les noms longs ; puis jouer la passe à luciepc avant de demander son tour de push.

L’archivage des cinquante documents attend ce diagnostic et un gabarit validé. Le code de M8 attend la relecture d’everything declarative ; le classeur local, le ménage des témoins et l’archivage réel restent soumis aux décisions de Lucie.

## Fichiers pour reprendre

Racine temporaire : `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/25b7ff4b-dabf-42ef-8580-86661c846fb9/scratchpad/blanc/`.

Les dossiers `reprise-11-11`, `diagnostic-etiquettes-courtes`, `validation-d1345a115` contiennent chacun `lancer.py`, `templates/`, `copie/`, `resultats.jsonl` et `etat/`. `validation-alias-d1345a115` rejoue les alias sous le binaire vérifié. Chaque dossier pointe exclusivement sur sa propre copie. Les `lib-chargee.txt` gardent les chemins lus dans `/proc` ; les refus complets restent privés dans `etat/requetes-refusees/`, leurs en-têtes ne sont pas publiés.

Les preuves résumées ci-dessous omettent les en-têtes et le texte des fiches. `/tmp` disparaît au redémarrage : reconstruire depuis l’instantané et les scripts du gabarit, sans réutiliser une copie déjà écrite. Le gabarit original est inchangé ; les deux scripts modifiés pour le diagnostic sont dans `diagnostic-etiquettes-courtes/templates/backends/memory/scripts/`.

```json
[
  {
    "essai": "reprise-11-11",
    "document": "extension/rag3weaver/docs/06-novembre-2026-orchestration/01-plan-du-week-end.md",
    "secondes_totales": 44.8,
    "sections": 7,
    "ok": 2,
    "indices_erreur": [
      0,
      1,
      2,
      4,
      5
    ],
    "refus": [
      {
        "fichier": "1794414542291-400-3752411.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          2202,
          11,
          2371,
          1811,
          2264,
          4,
          6
        ]
      },
      {
        "fichier": "1794414543589-400-3752411.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          2371,
          2264,
          11,
          2202,
          1811,
          4,
          6
        ]
      },
      {
        "fichier": "1794414549869-400-3752411.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          11,
          1811,
          2371,
          2264,
          2202,
          4,
          6
        ]
      },
      {
        "fichier": "1794414567931-400-3752411.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          17,
          2371,
          2264,
          24,
          2202,
          4,
          6
        ]
      },
      {
        "fichier": "1794414578178-400-3752411.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          1811,
          2264,
          2371,
          2202,
          11,
          4,
          6
        ]
      }
    ]
  },
  {
    "essai": "diagnostic-etiquettes-courtes",
    "document": "extension/rag3weaver/docs/06-novembre-2026-orchestration/01-plan-du-week-end.md",
    "secondes_totales": 86.8,
    "sections": 7,
    "ok": 7,
    "indices_erreur": [],
    "refus": []
  },
  {
    "essai": "validation-d1345a115",
    "document": "extension/rag3weaver/docs/06-novembre-2026-orchestration/01-plan-du-week-end.md",
    "secondes_totales": 46.9,
    "sections": 7,
    "ok": 2,
    "indices_erreur": [
      0,
      1,
      2,
      4,
      5
    ],
    "refus": [
      {
        "fichier": "1794414687141-400-3758692.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          2202,
          11,
          2371,
          1811,
          2264,
          4,
          6
        ]
      },
      {
        "fichier": "1794414688237-400-3758692.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          2371,
          2264,
          11,
          2202,
          1811,
          4,
          6
        ]
      },
      {
        "fichier": "1794414692782-400-3758692.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          11,
          1811,
          2371,
          2264,
          2202,
          4,
          6
        ]
      },
      {
        "fichier": "1794414710576-400-3758692.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          17,
          2371,
          2264,
          24,
          2202,
          4,
          6
        ]
      },
      {
        "fichier": "1794414723413-400-3758692.json",
        "status": 400,
        "corps": "[{\n  \"error\": {\n    \"code\": 400,\n    \"message\": \"Request contains an invalid argument.\",\n    \"status\": \"INVALID_ARGUMENT\"\n  }\n}\n]",
        "longueurs_enum": [
          1811,
          2264,
          2371,
          2202,
          11,
          4,
          6
        ]
      }
    ]
  }
]
```
