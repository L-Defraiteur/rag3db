# La chauffe du service compte ses troncatures comme du trafic

- **État** : corrigé (19 novembre 2026, embarquement, branche `truncation-relay`, e3d8c01f05) ; ouvert (19 novembre 2026, embarquement ; relevé en lisant le compteur du point C)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : mesure faussée. Chaque démarrage du 7878 (granite-278m) ajoute 169 à `truncated_texts`, sans aucun
  texte d'un client : la mesure du point C (combien de morceaux réels granite coupe à 512 jetons) en est faussée.
- **Atteignable en service** : oui, à chaque démarrage d'un service dont la fenêtre est sous 762 jetons
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, `EmbedDaemon::chauffer`)

## Ce que c'est

`EmbedDaemon::chauffer` (`src/daemon/embeddings.rs:411`) chauffe 16 formes : 40, 90, 190 et 380 « mots » (`m12 m45 …`),
en lots de 1, 8, 32 et du lot conseillé. Mesuré le 19 novembre avec le tokenizer de granite : 40 mots font 82 jetons,
90 en font 182, 190 en font 382, et **380 en font 762**. Granite tronque à 512 : les 1 + 8 + 32 + 128 = **169 textes**
de la forme la plus longue sont coupés et comptés dans `truncated_texts` (lot `token-cap`, 18 novembre).

Relevé du 19 novembre, au 7878 :
- 9 h 17, après la pose de G : 172 = 169 de chauffe + 3 des vérifications de G ;
- 11 h 27, après un redémarrage à 10 h 47 : 236 = 169 de chauffe + **67 du trafic réel en 40 minutes**.

La chauffe de bge-m3 (fenêtre 4 096 au 7879) ne coupe rien.

## Ce qu'il faudrait

Au choix, à décider :
1. **Remettre le compteur à zéro après la chauffe** : `truncated_texts` ne compte alors que le trafic. Le plus simple,
   mais l'embarqueur ne sait pas remettre son compteur à zéro aujourd'hui.
2. **Compter la base après la chauffe** dans le service, et ne rendre que la différence dans l'identité.
3. **Borner la forme la plus longue à la fenêtre du modèle** : la chauffe ne chauffe plus une forme que le modèle ne
   verra jamais entière ; elle n'est alors plus comptée.

Témoin, rouge d'abord : un service dont l'embarqueur tronque à 512 a `truncated_texts` à 0 juste après sa chauffe.

## Taille

Une à deux heures.

## Correction (19 novembre 2026)

Lot convenu avec c7 : la voie 2 (une base comptée après la chauffe), et le relais des troncatures au client.
- `truncated_texts` ne compte plus la chauffe ; la chauffe dit ses textes et ses coupes (`Chauffe::texts`,
  `::truncated`, et la ligne du journal du service).
- Chaque `/embed` dit les textes tronqués par le modèle ; `DaemonEmbedder` les compte et s'en souvient ; le journal
  d'index nomme les morceaux coupés (fichier et ligne, dix au plus).
- Témoins vus rouges puis verts : `the_warm_up_truncations_are_not_counted_as_traffic`,
  `the_service_says_which_texts_its_model_truncated_and_the_client_relays_it`,
  `the_index_journal_names_the_chunks_the_model_truncated`.
- **Reste** : la pose du binaire des services (G) ; puis une journée de trafic relue au journal d'index, pour savoir
  si les morceaux coupés par granite sont du code légitime (le découpage en amont est alors en cause).
