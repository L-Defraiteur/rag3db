# Laya pour l'archiviste : le protocole (14 novembre 2026)

Le mot de Lucie, par l'orchestration : « oui bien sûr, faut expérimenter avec, et pas seulement en
ce sens [la porte du rappel] : aussi si possible pour choisir fiche ou choisir créer nouvelle ».
Cette page dit ce qu'on va mesurer et comment, avant tout chiffre et avant tout branchement.

## 1. La question

L'**archiviste** range un texte (par exemple une section du journal des chantiers) dans la
mémoire longue. Pour chaque section, un rappel lui propose des fiches candidates. Un modèle, Gemini
Flash aujourd'hui, payé à chaque décision, répond l'une de trois choses :
- rejoindre la candidate N ;
- créer une fiche nouvelle ;
- rien (« none », la section ne mérite pas de fiche).

On mesure si **Laya**, un petit classifieur local, reproduit cette décision. Ensuite seulement, on
le mesurera en **porte du rappel** : faut-il rappeler une fiche à ce tour, et laquelle.

## 2. Le modèle

- **convaiinnovations/laya-multilingual** : ModernBERT multilingue (mmBERT-base), 322 M de
  paramètres, licence Apache 2.0 lue sur sa carte, 647 Mo. Il est déjà à luciepc dans
  `~/.cache/rag3db-banc-notes/hf/laya-multilingual/`, depuis le 8 novembre. Aucun nouveau
  téléchargement, donc pas besoin du jeton HF de `.vault`.
- Le paquet `laya` 0.4.29 est dans le venv du banc, `~/.cache/rag3db-banc-notes/decision-venv/`, à
  luciepc.
- **La forme de la question** : `choice`. Une option par candidate (`candidate_1` … `candidate_5`,
  chacune « nom — résumé »), plus `create`, plus `none` sur le vrai jeu. Laya rend une probabilité
  par option. Son option la plus probable est sa décision.
- **Sa limite connue** : en zéro-coup, il ne vaut rien sur nos jeux. Le 8 novembre, sur les fiches
  de mémoire, il était au hasard (page 03 du banc). Sa carte dit elle-même qu'il est fait pour
  être spécialisé. La mesure qui compte est donc la **spécialisation** (`laya.train.finetune`) sur
  une part du jeu, jugée sur une autre part.

## 3. Le matériel

- **Le CPU d'abord**, à luciepc, 8 fils, sous `poste lourd`. Laya y fait une décision en 0,3 à
  1,4 s (mesuré le 8 novembre), et le jeu compte quelques centaines de décisions.
- **card2 seulement pour un essai court**, qui rend la carte aussitôt. La VRAM est lue avant et
  après, et les services 7878-7880 sont interrogés comme sentinelle. Le 8 novembre, une passe de
  Laya sur card2 a tenu la carte 56 minutes et ralenti les services voisins. **Jamais card0.**
- **Aucune donnée de la mémoire de Lucie** n'est lue avant la trace de l'archiviste (§ 5).

## 4. Les métriques

- **L'exactitude de l'étiquette**, en tout et par décision attendue (rejoindre, créer, rien).
- **Le rang de la bonne réponse** parmi les options : la médiane, et la part au rang 1.
- **Le temps par décision**, p50 et p95, et le matériel.
- **Le découpage** : par fiche (et par source sur le vrai jeu), avec une graine fixe. Aucune fiche
  n'est à la fois en mise au point et en test : sinon, le modèle reconnaîtrait la fiche au lieu
  de juger la section.
- **La comparaison** : Gemini, la décision d'aujourd'hui ; Laya zéro-coup ; Laya spécialisé.

## 5. Les deux jeux

**Le jeu synthétique**, pour éprouver la mécanique de bout en bout, dès maintenant :
- il est tiré de la page de survol de la session mémoire,
  `docs/memoire/14-11-2026/02-survol-des-fiches-du-journal-des-chantiers.md` ;
- 61 fiches, 143 sections, et pour chaque section la fiche où l'archiviste l'a rangée ;
- chaque section donne deux cas à cinq candidates : « rejoindre » (sa fiche est parmi elles) et
  « créer » (elle n'y est pas). Soit 286 cas, dont 116 en test sur 18 fiches ;
- l'état n'est que le **titre** de la section, pas son texte : c'est un jeu plus dur et plus pauvre
  que le vrai. Il sert à vérifier que la chaîne tourne et que la spécialisation apprend quelque
  chose, pas à décider ;
- le script `p_jeu_synthetique.py` est au portable, dans `~/.cache/rag3db-banc-notes/decision/laya/`.

**Le vrai jeu**, quand la trace existera :
- la session mémoire fera écrire, par l'archiviste lui-même, une trace par section : le texte, les
  candidates dans l'ordre proposé (alias, nom, résumé), l'étiquette brute du modèle
  (`candidate_N`, `create` ou `none`), le sujet proposé, le kind final, et la voie du rejoint ;
- puis elle rejouera, sur copie, le journal des chantiers (152 sections) et la passe réelle de
  13 h 45 (95 décisions) ;
- le format, convenu avec elle, compte une ligne JSON par décision :
  `{section, texte, candidates[{nom, resume}], decision, fiche, etiquette, sujet, voie, kind_final, source}` ;
- **Laya est jugée sur `etiquette`**, la décision du modèle, et non sur la décision finale de
  l'archiviste, qui passe encore par le nom exact et la voisine.

## 6. Ce que Lucie lira

- **Pour chaque jeu** : l'exactitude en zéro-coup et spécialisé, par décision ; le rang de la
  bonne réponse ; le temps par décision ; et Gemini sur le même jeu.
- **Une recommandation** : branchable ou non, derrière le nœud Decide comme source de modèle
  déclarée, et à quel seuil de confiance renvoyer à Gemini.
- **Rien n'est branché** avant que Lucie et l'orchestration aient lu ces chiffres.

## 7. Les fichiers

- **Le portable**, `~/.cache/rag3db-banc-notes/decision/laya/` : `p_jeu_synthetique.py`
  (le jeu) et `o_laya.py` (zéro-coup, spécialisation, rejugement, métriques).
- **À luciepc**, `~/.cache/rag3db-banc-notes/laya/` : le même script, le jeu, `laya-synth.sh`
  (la passe CPU sous `poste lourd`) et les sorties.
