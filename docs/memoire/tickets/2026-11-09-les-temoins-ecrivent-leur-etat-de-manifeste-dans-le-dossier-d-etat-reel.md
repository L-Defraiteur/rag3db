# Les témoins écrivent leur état de manifeste dans le dossier d'état réel du poste

- **État** : ouvert, le 9 novembre 2026. Relevé par la session mémoire (96) en écrivant la page de
  M8 ; le correctif est dans le même lot (le garde des témoins), le ménage est à Lucie.
- **Famille** : produit
- **Gravité** : une pollution qui grossit au rythme des passes, dans un dossier de l'utilisatrice.
- **Atteignable en service** : oui, sur tout poste où une session joue les témoins.
- **Touche rag3weaver** : oui, `src/builtin_templates.rs` (`state_dir_of`), `tests/common`, le
  binaire e2e généré.

## Ce qu'on voit

Le dossier d'état que le gabarit vivant d'ED tient hors des déclarations,
`~/.local/state/rag3weaver/manifestes/<empreinte du chemin du manifeste>/`, portait sur le poste
de Lucie **477 entrées au relevé du 9 novembre 2026 à 13 h 30** (428 une heure plus tôt). Chaque entrée est un
`livre.json` (le gabarit d'où vient un manifeste, son contenu d'alors), une marque
`premier-index` (le chemin d'un journal d'index), ou les deux. La plus ancienne date du 8 novembre à
21 h 43, vingt minutes après le commit du gabarit vivant (f16628f8b, 21 h 22) ; elles viennent par rafales, une trentaine
dans la même seconde, au rythme des passes de tests des sessions sur ce poste.

Une seule de ces entrées correspond à un manifeste réel : l'atelier de Lucie
(`~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json`, empreinte
`bb347ff93e835435`). Les autres ont pour empreinte le chemin d'un manifeste qui n'existe plus,
dans un dossier temporaire de témoin (calculée par blake3 sur le chemin canonique, comme
`state_dir_of` ; contrôlée sur l'identifiant du backend d'une tâche, qui est la même empreinte).

## Pourquoi

`state_dir_of` prend `RAG3WEAVER_STATE_DIR`, sinon `$XDG_STATE_HOME`, sinon `~/.local/state`.
Aucun témoin ne posait `RAG3WEAVER_STATE_DIR` : chaque `code_tree::add_files` (qui note le
gabarit d'un manifeste posé), chaque pose d'atelier et chaque premier index d'un backend de code
ouvert par un hôte de témoin écrivaient dans le dossier réel. Le témoin
`a_witness_never_writes_in_the_real_state_directory` le montrait rouge sur master 7ad0273c8 :
« le dossier d'état d'un témoin est sous le dossier réel /home/lucied/.local/state ».

## Le correctif (ce lot)

- Sous `cfg(test)`, sans `RAG3WEAVER_STATE_DIR`, la racine est un dossier temporaire propre au
  processus (`rag3weaver-etat-des-temoins-<pid>`) ; une racine sous le dossier réel fait
  paniquer en le disant (`refuser_le_reel`).
- Le binaire e2e pose `RAG3WEAVER_STATE_DIR` avant `main`, quand aucun fil n'existe encore
  (`common::poser_l_etat_des_temoins`, par `.init_array`, écrit par `tests/e2e/generer.py`) ;
  les hôtes et les binaires que les suites lancent en héritent. Témoin :
  `e2e_demon_hote::le_dossier_d_etat_d_un_e2e_est_temporaire`.

## Le ménage : à Lucie

Rien n'est effacé : c'est son dossier. L'entrée de l'atelier (`bb347ff93e835435`) est la sienne
et doit rester. Les autres peuvent partir. La liste au relevé : date, empreinte, gabarit noté,
marque de premier index, manifeste réel connu.

```
2026-11-08 21:43:03 | 10934a865b7510f7 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:03 | 42fc696ca825c207 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:03 | 7580e5223f0a4ad7 | backends/atelier/backend.json |  | 
2026-11-08 21:43:03 | a54d09f284929f07 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:03 | fbc126aa591baded | backends/code/backend.json | premier-index | 
2026-11-08 21:43:30 | 15edefd58a4034a8 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:30 | 36d99bc40afa0304 | backends/atelier/backend.json |  | 
2026-11-08 21:43:30 | 5e014ec11cf95947 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:31 | 0de81d5d8c7b5ba8 | backends/code/backend.json | premier-index | 
2026-11-08 21:43:33 | 5e417005353277d7 | backends/code/backend.json |  | 
2026-11-08 21:44:19 | 132031867245a6f7 | backends/code/backend.json | premier-index | 
2026-11-08 21:44:19 | 16252a9aca378955 | backends/atelier/backend.json |  | 
2026-11-08 21:44:19 | b798ca83610b3db8 | backends/code/backend.json | premier-index | 
2026-11-08 21:44:19 | d8521b544ec597eb | backends/code/backend.json | premier-index | 
2026-11-08 21:44:21 | a76ebd542bab0b00 | backends/code/backend.json |  | 
2026-11-08 21:44:57 | 332c0523ef07d264 | backends/atelier/backend.json |  | 
2026-11-08 21:44:58 | 58d981818e030183 | backends/code/backend.json | premier-index | 
2026-11-08 21:44:58 | 9c5ea1c4795c1782 | backends/code/backend.json | premier-index | 
2026-11-08 21:44:58 | c41441b7ae5c4fed | backends/code/backend.json | premier-index | 
2026-11-08 21:44:59 | 2b6b76c1d7e434ab | backends/code/backend.json |  | 
2026-11-08 21:48:26 | 667e0df465294b9c | backends/atelier/backend.json |  | 
2026-11-08 21:48:26 | c2bdedcdcefc9927 | backends/atelier/backend.json |  | 
2026-11-08 21:48:27 | 129cb649da5e6397 | backends/code/backend.json |  | 
2026-11-08 21:48:27 | d8af613fb7ac153d | backends/code/backend.json |  | 
2026-11-08 21:48:27 | f626b83ea42c3b0b | backends/code/backend.json |  | 
2026-11-08 21:48:30 | 90b779d90423b189 | backends/code/backend.json |  | 
2026-11-08 21:48:30 | d6d7941cd2c29441 | backends/code/backend.json |  | 
2026-11-08 21:48:30 | ee72ae640cea0b33 | backends/code/backend.json |  | 
2026-11-08 21:48:30 | f56caecc476a342d | backends/code/backend.json |  | 
2026-11-08 21:48:31 | 261b0a4ddc0060db | backends/code/backend.json |  | 
2026-11-08 21:48:31 | 3bc6d9e8273fea02 | backends/code/backend.json |  | 
2026-11-08 21:48:31 | 62b6ad6dac1f55db | backends/code/backend.json |  | 
2026-11-08 21:48:31 | 84e8d5f6f7f931ac | backends/code/backend.json |  | 
2026-11-08 21:48:31 | a4e79e2177843a06 | backends/code/backend.json |  | 
2026-11-08 21:48:31 | a8a7d7de0fd5c5eb | backends/code/backend.json |  | 
2026-11-08 21:48:31 | bc5818f5c1ef7155 | backends/code/backend.json |  | 
2026-11-08 21:48:31 | bd6e4c3ad1e51004 | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 0788822176a3558d | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 0a95ab8bf993fc10 | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 159da0104f2af29e | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 239e4266dfb61bff | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 3c8403ca4be3a702 | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 7ea902ed58724e86 | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 81ec1411c12d1500 | backends/code/backend.json |  | 
2026-11-08 21:48:32 | 8cde92a47575ad8f | backends/code/backend.json |  | 
2026-11-08 21:48:33 | 5782107ffe6cb8fb | backends/code/backend.json | premier-index | 
2026-11-08 21:48:33 | 726d7af769d42b87 | backends/atelier/backend.json |  | 
2026-11-08 21:48:33 | a3fea6e3ba1d5f6b | backends/code/backend.json | premier-index | 
2026-11-08 21:48:34 | 674d61280e6fee7d | backends/code/backend.json | premier-index | 
2026-11-08 21:48:34 | 72fff21189553f3e | backends/code/backend.json |  | 
2026-11-08 21:48:39 | 2fbc135ec398c20e | backends/code/backend.json |  | 
2026-11-08 22:16:19 | a68b704748d66d91 | backends/atelier/backend.json |  | 
2026-11-08 22:16:19 | ced427d7ad822459 | backends/atelier/backend.json |  | 
2026-11-08 22:16:22 | 12c7d06c0636bb8a | backends/code/backend.json |  | 
2026-11-08 22:16:22 | 54a43b3347853c8f | backends/code/backend.json |  | 
2026-11-08 22:16:22 | 6bba9659a661c018 | backends/code/backend.json |  | 
2026-11-08 22:16:22 | 7213000ad4602f52 | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 0215f68f1fcd3598 | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 1185c7b7348702fa | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 27656b7fdce4009e | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 6fa7f66e7c5a3617 | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 85bb89ac3a52cc0d | backends/code/backend.json |  | 
2026-11-08 22:16:23 | 916a3944b3e6c53f | backends/code/backend.json |  | 
2026-11-08 22:16:23 | cf2133e0f9bb5449 | backends/code/backend.json |  | 
2026-11-08 22:16:23 | e7bae81f18248da9 | backends/code/backend.json |  | 
2026-11-08 22:16:23 | ebf191c2732ccc3d | backends/code/backend.json |  | 
2026-11-08 22:16:23 | f2a30585e1b7b90a | backends/code/backend.json |  | 
2026-11-08 22:16:24 | 064ea4e4af340dce | backends/code/backend.json |  | 
2026-11-08 22:16:24 | 2d15d402e1300dd8 | backends/code/backend.json |  | 
2026-11-08 22:16:24 | 2f8de496a95dd97f | backends/code/backend.json |  | 
2026-11-08 22:16:24 | 7c2f4d29fecd48b5 | backends/code/backend.json |  | 
2026-11-08 22:16:24 | c6ed94ad45ced0f6 | backends/code/backend.json |  | 
2026-11-08 22:16:24 | cb92b285044fcc3a | backends/code/backend.json |  | 
2026-11-08 22:16:25 | b35cd36e8dfa5c9a | backends/code/backend.json | premier-index | 
2026-11-08 22:16:25 | d19b97a1c1bac03b | backends/atelier/backend.json |  | 
2026-11-08 22:16:26 | 88a9c99215d1f847 | backends/code/backend.json |  | 
2026-11-08 22:16:26 | c9f173ed7507e40f | backends/code/backend.json | premier-index | 
2026-11-08 22:16:26 | feefa369cc54a468 | backends/code/backend.json | premier-index | 
2026-11-08 22:16:29 | 9f69b81e8fe84a27 | — | premier-index | 
2026-11-08 22:16:31 | c70c580559a7e10d | backends/code/backend.json |  | 
2026-11-08 22:17:11 | bb347ff93e835435 | backends/atelier/backend.json |  | /home/lucied/.config/rag3weaver/atelier/templates/backends/atelier/backend.json
2026-11-08 22:18:44 | 0548ee83303c27e6 | backends/code/backend.json |  | 
2026-11-08 22:18:44 | 180c3449e8fd4f83 | backends/code/backend.json |  | 
2026-11-08 22:18:44 | b180bfa9ac8b4c0c | backends/code/backend.json |  | 
2026-11-08 22:18:44 | db67b1a3c7367b50 | backends/code/backend.json |  | 
2026-11-08 22:18:45 | 71c566d89345f16f | backends/code/backend.json | premier-index | 
2026-11-08 22:18:45 | 9552cd0e22bf9500 | backends/code/backend.json | premier-index | 
2026-11-08 23:09:26 | 1226cdc4892cb44c | backends/code/backend.json |  | 
2026-11-08 23:09:26 | 6962ae3de82d62ab | backends/code/backend.json |  | 
2026-11-08 23:09:26 | 8fe899345e2907a8 | backends/code/backend.json |  | 
2026-11-08 23:09:26 | ef729f1ed1ffbc4a | backends/code/backend.json |  | 
2026-11-08 23:09:28 | 0d7308823fe8e061 | backends/code/backend.json | premier-index | 
2026-11-08 23:09:28 | 0fb6dbd5bbb7997a | backends/code/backend.json | premier-index | 
2026-11-08 23:09:28 | 1fc7b6f80edfaebe | backends/code/backend.json | premier-index | 
2026-11-08 23:11:48 | 0399cc820670ab4b | backends/code/backend.json |  | 
2026-11-08 23:11:48 | 223c3eeee69344d5 | backends/code/backend.json |  | 
2026-11-08 23:11:48 | 8f29ee6e45cf81b5 | backends/code/backend.json |  | 
2026-11-08 23:11:48 | ed0c71956ee61d27 | backends/code/backend.json |  | 
2026-11-08 23:11:49 | 02c26645595670fc | backends/code/backend.json | premier-index | 
2026-11-08 23:11:49 | 5e02d9b19312fcf6 | backends/code/backend.json | premier-index | 
2026-11-08 23:11:50 | 95c4e14688a74122 | backends/code/backend.json | premier-index | 
2026-11-08 23:14:26 | 1b0fd4130b794b05 | backends/code/backend.json |  | 
2026-11-08 23:14:26 | 2a48fd4a19915bde | backends/code/backend.json |  | 
2026-11-08 23:14:26 | 8b3733b62772dfc4 | backends/code/backend.json |  | 
2026-11-08 23:14:26 | a5737ff7bf5da011 | backends/code/backend.json |  | 
2026-11-08 23:14:27 | 6cd402aaa29c1720 | backends/code/backend.json | premier-index | 
2026-11-08 23:14:27 | d1011dd75a5adea5 | backends/code/backend.json | premier-index | 
2026-11-08 23:14:28 | 9e2c1a9c63bb7b91 | backends/code/backend.json | premier-index | 
2026-11-08 23:21:56 | 1e33e67774a26d9f | backends/code/backend.json |  | 
2026-11-08 23:21:56 | 75695a34220ddfed | backends/code/backend.json |  | 
2026-11-08 23:21:56 | a36269d68c24f21d | backends/code/backend.json |  | 
2026-11-08 23:21:56 | f589fb7c7a262b10 | backends/code/backend.json |  | 
2026-11-08 23:21:57 | 2359d47724b8c370 | backends/code/backend.json | premier-index | 
2026-11-08 23:21:57 | 662fb246ea96bacd | backends/code/backend.json | premier-index | 
2026-11-08 23:21:57 | cb4b0c8698471fab | backends/code/backend.json | premier-index | 
2026-11-08 23:24:07 | 1d450d041df69e27 | backends/code/backend.json |  | 
2026-11-08 23:24:07 | 6c33946350c16388 | backends/code/backend.json |  | 
2026-11-08 23:24:07 | 76f38638b3ac0067 | backends/code/backend.json |  | 
2026-11-08 23:24:07 | d1dc75dce01a2e27 | backends/code/backend.json |  | 
2026-11-08 23:24:08 | b298e27a940aa846 | backends/code/backend.json | premier-index | 
2026-11-08 23:24:09 | 35ba52e42083d655 | backends/code/backend.json | premier-index | 
2026-11-08 23:24:09 | 88592d370baf8424 | backends/code/backend.json | premier-index | 
2026-11-08 23:26:18 | 355ef1526f419c31 | backends/code/backend.json |  | 
2026-11-08 23:26:18 | 62f44ae0b2210ccb | backends/code/backend.json |  | 
2026-11-08 23:26:18 | eeb44c6c91ec41cd | backends/code/backend.json |  | 
2026-11-08 23:26:18 | f5b90c59c5bac65f | backends/code/backend.json |  | 
2026-11-08 23:26:19 | b16401870086b9f8 | backends/code/backend.json | premier-index | 
2026-11-08 23:26:20 | eb563a4d68016767 | backends/code/backend.json | premier-index | 
2026-11-08 23:26:20 | fabe1781d7ca6f80 | backends/code/backend.json | premier-index | 
2026-11-08 23:29:26 | 90558b1e1b7d1f17 | backends/code/backend.json |  | 
2026-11-08 23:29:26 | b6dea48d26b30e4f | backends/code/backend.json |  | 
2026-11-08 23:29:26 | b772ccdacc67419c | backends/code/backend.json |  | 
2026-11-08 23:29:26 | bf32a18ff2a5486f | backends/code/backend.json |  | 
2026-11-08 23:29:28 | 415f7d8d5aea3864 | backends/code/backend.json | premier-index | 
2026-11-08 23:29:28 | 632fbec1854468e7 | backends/code/backend.json | premier-index | 
2026-11-08 23:29:28 | f5848bfac3efcf18 | backends/code/backend.json | premier-index | 
2026-11-08 23:32:40 | 724bdc4bbfbf8b03 | backends/code/backend.json |  | 
2026-11-08 23:32:40 | e459cbb2805b6fd9 | backends/code/backend.json |  | 
2026-11-08 23:32:40 | e9ce0464de06bfac | backends/code/backend.json |  | 
2026-11-08 23:32:40 | ff45a0275023ef07 | backends/code/backend.json |  | 
2026-11-08 23:32:42 | 1c936970aec40d31 | backends/code/backend.json | premier-index | 
2026-11-08 23:32:42 | 6dcbd717738be678 | backends/code/backend.json | premier-index | 
2026-11-08 23:32:42 | efa43439039e1cd1 | backends/code/backend.json | premier-index | 
2026-11-08 23:35:08 | 0ed64b292b69b26a | backends/code/backend.json |  | 
2026-11-08 23:35:08 | 3409b33b64393336 | backends/code/backend.json |  | 
2026-11-08 23:35:08 | 4509e43666d49116 | backends/code/backend.json |  | 
2026-11-08 23:35:08 | d5c17db314ce580d | backends/code/backend.json |  | 
2026-11-08 23:35:10 | 17370aec672fd5d7 | backends/code/backend.json | premier-index | 
2026-11-08 23:35:10 | 942a23cc2091b77b | backends/code/backend.json | premier-index | 
2026-11-08 23:35:10 | f5083502c32d78c4 | backends/code/backend.json | premier-index | 
2026-11-08 23:37:18 | 3a4bd604fa6bef53 | backends/code/backend.json |  | 
2026-11-08 23:37:18 | 5d2a876602ae0642 | backends/code/backend.json |  | 
2026-11-08 23:37:18 | a32a13eef0094a68 | backends/code/backend.json |  | 
2026-11-08 23:37:18 | eb5cd99c62fa20ed | backends/code/backend.json |  | 
2026-11-08 23:37:20 | 0f7f3bf7c298a1fa | backends/code/backend.json | premier-index | 
2026-11-08 23:37:20 | 130494cd669c0f95 | backends/code/backend.json | premier-index | 
2026-11-08 23:37:20 | 8deaf36074fa63d4 | backends/code/backend.json | premier-index | 
2026-11-08 23:47:46 | 279b723fb22075c4 | backends/atelier/backend.json |  | 
2026-11-08 23:47:46 | 90689605e5afb23f | backends/atelier/backend.json |  | 
2026-11-08 23:47:47 | 3354949d2ddee1fe | backends/code/backend.json |  | 
2026-11-08 23:47:47 | b622c73009b72e8b | backends/code/backend.json |  | 
2026-11-08 23:47:47 | d4d500dfd96f8a86 | backends/code/backend.json |  | 
2026-11-08 23:47:49 | 02da11cd0f069e17 | backends/code/backend.json |  | 
2026-11-08 23:47:49 | 98c17e3c10d09a42 | backends/code/backend.json |  | 
2026-11-08 23:47:49 | 9d3f15e1534b2ff7 | backends/code/backend.json |  | 
2026-11-08 23:47:49 | c89a29b8490111be | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 11fbf66b08c1aef7 | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 24a80c048aaf8be1 | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 4bcf8d80bb93356b | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 6b7fb2bbc35f8325 | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 7a2d9cf46e0c25b1 | backends/code/backend.json |  | 
2026-11-08 23:47:50 | 8e2c3fa8051bc314 | backends/code/backend.json |  | 
2026-11-08 23:47:50 | dd86efbb556bae38 | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 0760a46fef5c017e | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 1c2644587030d03b | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 61925a108d7a04ab | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 65f820a6a05d35fb | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 6a50d6a35ce0c1c7 | backends/code/backend.json |  | 
2026-11-08 23:47:51 | 792ca8f224fbff55 | backends/code/backend.json |  | 
2026-11-08 23:47:51 | d60b328e97107063 | backends/code/backend.json |  | 
2026-11-08 23:47:51 | dce6c5787a2e7425 | backends/code/backend.json |  | 
2026-11-08 23:47:51 | fcfa1ba2d6934ff5 | backends/code/backend.json |  | 
2026-11-08 23:47:52 | 4e16d56551e86188 | backends/atelier/backend.json |  | 
2026-11-08 23:47:52 | 688a302aa651550a | backends/code/backend.json | premier-index | 
2026-11-08 23:47:53 | 4791fc3359971cd0 | backends/code/backend.json | premier-index | 
2026-11-08 23:47:53 | 7f34bbd36dd97b21 | backends/code/backend.json | premier-index | 
2026-11-08 23:47:53 | 8a59e779d9737949 | backends/code/backend.json | premier-index | 
2026-11-08 23:47:53 | 970a71f79a0279ac | backends/code/backend.json |  | 
2026-11-08 23:47:56 | 1821db4fe7425074 | — | premier-index | 
2026-11-08 23:47:58 | 4cdd45f0edd56788 | backends/code/backend.json |  | 
2026-11-08 23:50:04 | 6137b4b3522aebc1 | backends/code/backend.json |  | 
2026-11-08 23:50:04 | 623516b5801d1d11 | backends/code/backend.json |  | 
2026-11-08 23:50:04 | 7627862455d18ed7 | backends/code/backend.json |  | 
2026-11-08 23:50:04 | 861f94b05b5d5814 | backends/code/backend.json |  | 
2026-11-08 23:50:06 | b596c7551e63670e | backends/code/backend.json | premier-index | 
2026-11-08 23:50:07 | acdd223f4bdc12ee | backends/code/backend.json | premier-index | 
2026-11-08 23:53:44 | 25cd7e7762247af8 | backends/code/backend.json |  | 
2026-11-08 23:53:44 | 2f9cd5ab2718a5a1 | backends/code/backend.json |  | 
2026-11-08 23:53:44 | c21b2b773f18ae5e | backends/code/backend.json |  | 
2026-11-08 23:53:44 | eb19d9d9368b3882 | backends/code/backend.json |  | 
2026-11-08 23:53:45 | cda4af97589b927a | backends/code/backend.json | premier-index | 
2026-11-08 23:53:45 | d480b73fabdb1bfc | backends/code/backend.json | premier-index | 
2026-11-08 23:53:46 | 752a440800949a2c | backends/code/backend.json | premier-index | 
2026-11-08 23:55:55 | 00c827937afca672 | backends/code/backend.json |  | 
2026-11-08 23:55:55 | 08dbd97662baedde | backends/code/backend.json |  | 
2026-11-08 23:55:55 | 2a4a8bdc323b2ed0 | backends/code/backend.json |  | 
2026-11-08 23:55:55 | 32a4c14199630fa4 | backends/code/backend.json |  | 
2026-11-08 23:55:56 | 5b9cb2a2d5cf9f49 | backends/code/backend.json | premier-index | 
2026-11-08 23:55:56 | e372728a843932b4 | backends/code/backend.json | premier-index | 
2026-11-08 23:55:56 | e8dd344708a56f20 | backends/code/backend.json | premier-index | 
2026-11-08 23:58:06 | a2cb3e424485f51e | backends/code/backend.json |  | 
2026-11-08 23:58:06 | c69ba819846f62a1 | backends/code/backend.json |  | 
2026-11-08 23:58:06 | dab43c7937cc8db6 | backends/code/backend.json |  | 
2026-11-08 23:58:06 | e930ac51895b444a | backends/code/backend.json |  | 
2026-11-08 23:58:07 | 67bfd92131f50ede | backends/code/backend.json | premier-index | 
2026-11-08 23:58:07 | c5ab42d4b69c1ae4 | backends/code/backend.json | premier-index | 
2026-11-08 23:58:07 | dd52042a590c0b7d | backends/code/backend.json | premier-index | 
2026-11-09 00:01:55 | 1b8f65d1ebe5d86d | backends/code/backend.json |  | 
2026-11-09 00:01:55 | 2a5f87e6fbe1731f | backends/code/backend.json |  | 
2026-11-09 00:01:55 | 841f6f9848c75610 | backends/code/backend.json |  | 
2026-11-09 00:01:55 | e6ac03c5217c273b | backends/atelier/backend.json |  | 
2026-11-09 00:01:55 | fed63908d6f1582e | backends/atelier/backend.json |  | 
2026-11-09 00:01:57 | 42eb9c8e4247e005 | backends/code/backend.json |  | 
2026-11-09 00:01:57 | 6207d5dfdf4a29a1 | backends/code/backend.json |  | 
2026-11-09 00:01:57 | 939b74498ae9801f | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 1d8a5ccb5c63389b | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 25d65c18031d2ac1 | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 4bf2dce5e3bc67dd | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 654be2763fef41d2 | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 6d9efc8b00861a7c | backends/code/backend.json |  | 
2026-11-09 00:01:58 | 786a5d6a3f49b9eb | backends/code/backend.json |  | 
2026-11-09 00:01:58 | b666b57762c53db0 | backends/code/backend.json |  | 
2026-11-09 00:01:58 | cac0f29faf8af365 | backends/code/backend.json |  | 
2026-11-09 00:01:58 | d0d5643a1576282e | backends/code/backend.json |  | 
2026-11-09 00:01:58 | e9d849b2e597e7db | backends/code/backend.json |  | 
2026-11-09 00:01:59 | 7b5bab4f77def4c8 | backends/code/backend.json |  | 
2026-11-09 00:01:59 | 88d4e4ad81944f22 | backends/code/backend.json |  | 
2026-11-09 00:01:59 | 94bb17dfb4b15c3b | backends/code/backend.json |  | 
2026-11-09 00:01:59 | 9c0656bb1d6bd8d5 | backends/code/backend.json |  | 
2026-11-09 00:01:59 | a97014de63a0e80a | backends/code/backend.json |  | 
2026-11-09 00:01:59 | daa128d51d8bf768 | backends/code/backend.json |  | 
2026-11-09 00:01:59 | e6051787438fcce6 | backends/code/backend.json |  | 
2026-11-09 00:01:59 | e7ce9e2c91a91830 | backends/atelier/backend.json |  | 
2026-11-09 00:02:00 | dfd450958bb05219 | backends/code/backend.json | premier-index | 
2026-11-09 00:02:01 | 230857150cbd14fd | backends/code/backend.json | premier-index | 
2026-11-09 00:02:01 | 7fcd3f22b3b6bb4f | backends/code/backend.json | premier-index | 
2026-11-09 00:02:01 | 8c21fdf82e4c9e5c | backends/code/backend.json |  | 
2026-11-09 00:02:01 | beba4716c1e645bb | backends/code/backend.json | premier-index | 
2026-11-09 00:02:04 | 5b9ab3324a9f8590 | — | premier-index | 
2026-11-09 00:02:06 | b41abb1ceaf054a5 | backends/code/backend.json |  | 
2026-11-09 00:03:46 | 35e2eb42cd34fcf0 | backends/code/backend.json |  | 
2026-11-09 00:03:46 | 410eced247f8dd4b | backends/code/backend.json |  | 
2026-11-09 00:03:46 | 4e7b568da8804da3 | backends/code/backend.json |  | 
2026-11-09 00:03:46 | c919b6f472ec1034 | backends/code/backend.json |  | 
2026-11-09 00:03:47 | 5a64c237c4f1c4f8 | backends/code/backend.json | premier-index | 
2026-11-09 00:03:47 | 8f028dc7e8e9dc0e | backends/code/backend.json | premier-index | 
2026-11-09 00:03:47 | 96e3dd95528037fa | backends/code/backend.json | premier-index | 
2026-11-09 01:37:27 | 128c9b15dc1f4317 | backends/atelier/backend.json |  | 
2026-11-09 01:37:27 | 3fbfa8efe6a7ec73 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:27 | 9d97cae010005a02 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:27 | de82861739e9c5f4 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:27 | f1790bff09185156 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:28 | 38e9301332a9e675 | — | premier-index | 
2026-11-09 01:37:29 | b2ff63e950877679 | backends/code/backend.json |  | 
2026-11-09 01:37:38 | 08a2d749d2f52b7f | backends/code/backend.json | premier-index | 
2026-11-09 01:37:38 | 2b3f40aba8a6bba4 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:38 | 8de11ad438e7e12b | backends/atelier/backend.json |  | 
2026-11-09 01:37:38 | 9a50fd0a1f2f8265 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:38 | baed53a67c4106f6 | backends/code/backend.json | premier-index | 
2026-11-09 01:37:39 | f8dcc8e9460060a5 | — | premier-index | 
2026-11-09 01:37:40 | 5dc87e41fb36216c | backends/code/backend.json |  | 
2026-11-09 11:01:09 | e2be8fda174dd992 | backends/code/backend.json | premier-index | 
2026-11-09 11:02:25 | afe137d8feeabb37 | backends/code/backend.json | premier-index | 
2026-11-09 11:04:31 | 3a2d03ccd0896502 | backends/code/backend.json | premier-index | 
2026-11-09 11:05:50 | ab6ede9595f21579 | backends/code/backend.json | premier-index | 
2026-11-09 11:05:50 | baf15d3e4a7dc316 | backends/code/backend.json | premier-index | 
2026-11-09 11:07:00 | c930b9730a7df5ec | backends/code/backend.json | premier-index | 
2026-11-09 11:07:00 | d6c69b9e49465310 | backends/code/backend.json | premier-index | 
2026-11-09 11:07:00 | d9c32617d1be2979 | backends/code/backend.json | premier-index | 
2026-11-09 11:07:58 | 020b66b3736e38e2 | backends/code/backend.json | premier-index | 
2026-11-09 11:07:58 | 0928adf08ffca695 | backends/code/backend.json | premier-index | 
2026-11-09 11:07:58 | 682ade8e9e6f028b | backends/code/backend.json | premier-index | 
2026-11-09 11:08:47 | 3a33d974984ff61e | backends/code/backend.json | premier-index | 
2026-11-09 11:08:47 | 8f0ea49f76a29fac | backends/code/backend.json | premier-index | 
2026-11-09 11:08:47 | aff298786629bec1 | backends/code/backend.json | premier-index | 
2026-11-09 11:10:39 | 0b9c0e55c46eb310 | backends/code/backend.json | premier-index | 
2026-11-09 11:10:39 | 260754579a8f5ddd | backends/code/backend.json | premier-index | 
2026-11-09 11:10:39 | db0f769ecb810de3 | backends/code/backend.json | premier-index | 
2026-11-09 11:12:17 | ba5be1ae5c227fdd | backends/code/backend.json | premier-index | 
2026-11-09 11:12:19 | 053acc35a3750326 | backends/code/backend.json | premier-index | 
2026-11-09 11:12:20 | d86fe3187e6c65f9 | backends/code/backend.json | premier-index | 
2026-11-09 11:15:26 | c2f1618c256c9f83 | backends/code/backend.json | premier-index | 
2026-11-09 11:16:45 | 9da9fa0a629e9783 | backends/code/backend.json | premier-index | 
2026-11-09 11:17:55 | 7f1c6ea707aac657 | backends/code/backend.json | premier-index | 
2026-11-09 11:17:58 | 8720c3431f45874f | backends/code/backend.json | premier-index | 
2026-11-09 11:18:04 | af846fb78ddf51e2 | backends/code/backend.json | premier-index | 
2026-11-09 11:19:16 | 3a447398c09a8d83 | backends/code/backend.json | premier-index | 
2026-11-09 11:19:24 | 4c2377b6a6f98dc9 | backends/code/backend.json | premier-index | 
2026-11-09 11:19:24 | d66b6516dc46e4b0 | backends/code/backend.json | premier-index | 
2026-11-09 11:19:24 | fd1a0cfb98e10034 | backends/code/backend.json | premier-index | 
2026-11-09 11:22:20 | 717dbfe34d171de7 | backends/code/backend.json | premier-index | 
2026-11-09 11:22:23 | 7ce43cae8f777fea | backends/code/backend.json | premier-index | 
2026-11-09 11:22:31 | 8a2c2dbeedd50c44 | backends/code/backend.json | premier-index | 
2026-11-09 11:22:31 | 923efabefdfa01ef | backends/code/backend.json | premier-index | 
2026-11-09 11:25:35 | 18e09d8bcab517ba | backends/code/backend.json |  | 
2026-11-09 11:25:35 | 2344246702835e19 | backends/code/backend.json |  | 
2026-11-09 11:25:35 | 2d4e54fec9fbad5b | backends/code/backend.json |  | 
2026-11-09 11:25:35 | 9cb80487863a0cd8 | backends/code/backend.json |  | 
2026-11-09 11:26:24 | 6c24b1d23e2bd194 | backends/code/backend.json | premier-index | 
2026-11-09 11:26:42 | 980029ac310b9c85 | backends/code/backend.json | premier-index | 
2026-11-09 11:26:54 | a0a049089804c4d1 | backends/code/backend.json | premier-index | 
2026-11-09 11:29:37 | 02f1761e15d28087 | backends/code/backend.json |  | 
2026-11-09 11:29:37 | 104af380c8b04c78 | backends/code/backend.json |  | 
2026-11-09 11:29:37 | 3fbf527a96cdcaa2 | backends/code/backend.json |  | 
2026-11-09 11:29:37 | 5d2c3a189ab205b1 | backends/code/backend.json |  | 
2026-11-09 11:30:49 | 47a20037d6086a04 | backends/code/backend.json | premier-index | 
2026-11-09 11:31:00 | dcbbb6114ef53c31 | backends/code/backend.json | premier-index | 
2026-11-09 11:31:11 | cf2aacfffa9e5c67 | backends/code/backend.json | premier-index | 
2026-11-09 11:33:37 | 089468e801783b53 | backends/code/backend.json |  | 
2026-11-09 11:33:37 | 311d9d8eeacf2c36 | backends/code/backend.json |  | 
2026-11-09 11:33:37 | 66a2e40f48d9b9b9 | backends/code/backend.json |  | 
2026-11-09 11:33:37 | f14e39945e65c9c0 | backends/code/backend.json |  | 
2026-11-09 11:34:43 | 254d49964877d18c | backends/code/backend.json | premier-index | 
2026-11-09 11:34:43 | 3d81a05edd9b470c | backends/code/backend.json | premier-index | 
2026-11-09 11:35:07 | fd7f2edb05a654a4 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:14 | fa9dfb2cc1ae7c71 | backends/atelier/backend.json |  | 
2026-11-09 12:31:14 | fbf5e061a684cee6 | backends/atelier/backend.json |  | 
2026-11-09 12:31:17 | 6f6d0bf59dc97ddf | backends/code/backend.json |  | 
2026-11-09 12:31:17 | b9f70df12f20d15a | backends/code/backend.json |  | 
2026-11-09 12:31:17 | c55dcb8dabe0a7a1 | backends/code/backend.json |  | 
2026-11-09 12:31:21 | 756b64e7b9977f3c | backends/code/backend.json |  | 
2026-11-09 12:31:22 | 4d14192887dc5820 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:22 | 506926722c85734b | backends/code/backend.json |  | 
2026-11-09 12:31:22 | a3140e93856bb2e3 | backends/code/backend.json |  | 
2026-11-09 12:31:22 | ab14f6fb5c7482f2 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:22 | b5cfdddd849eb37a | backends/code/backend.json |  | 
2026-11-09 12:31:22 | d5d7e300f936a822 | backends/code/backend.json |  | 
2026-11-09 12:31:22 | e8ea280d53b31d7c | backends/code/backend.json | premier-index | 
2026-11-09 12:31:22 | ff724c430a8739d4 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:23 | 16f905497c87cc95 | backends/code/backend.json |  | 
2026-11-09 12:31:23 | 1a205b616b23953f | backends/code/backend.json |  | 
2026-11-09 12:31:23 | 5242cf04ed6f46b1 | backends/code/backend.json |  | 
2026-11-09 12:31:23 | 648f74cbdd00fcce | backends/code/backend.json |  | 
2026-11-09 12:31:23 | 7918ab89250b49e5 | backends/code/backend.json |  | 
2026-11-09 12:31:23 | f97064f0303f414d | backends/code/backend.json |  | 
2026-11-09 12:31:24 | 101cb1653b81714b | backends/code/backend.json |  | 
2026-11-09 12:31:24 | 23137e1d2586176f | backends/code/backend.json |  | 
2026-11-09 12:31:24 | b5f9bc2441022ae9 | backends/code/backend.json |  | 
2026-11-09 12:31:24 | b696cbe88a9df02f | backends/code/backend.json |  | 
2026-11-09 12:31:24 | ba0633cc85692cb7 | backends/code/backend.json |  | 
2026-11-09 12:31:24 | bd01635e079a2c20 | backends/code/backend.json |  | 
2026-11-09 12:31:24 | dc4a6d05fa594cd8 | backends/code/backend.json |  | 
2026-11-09 12:31:24 | f820988815c19713 | backends/code/backend.json |  | 
2026-11-09 12:31:24 | fd49da8fadbb01be | backends/code/backend.json |  | 
2026-11-09 12:31:26 | adef43ce5d75d865 | backends/atelier/backend.json |  | 
2026-11-09 12:31:28 | 67c5a412de99382f | backends/code/backend.json |  | 
2026-11-09 12:31:28 | 8eb672a6666ede5d | backends/code/backend.json | premier-index | 
2026-11-09 12:31:30 | 0c2a361336f689f5 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:30 | 57b70b9c95e452e5 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:30 | cc4f24248e1ca337 | backends/code/backend.json | premier-index | 
2026-11-09 12:31:34 | ec97b58f02060da4 | — | premier-index | 
2026-11-09 12:31:37 | 9c88b02cf5ee2762 | backends/code/backend.json |  | 
2026-11-09 13:10:31 | 539c1f1d06f3a450 | backends/atelier/backend.json |  | 
2026-11-09 13:10:31 | 9f15899a232a32f3 | backends/atelier/backend.json |  | 
2026-11-09 13:10:34 | 05876ce0d5773898 | backends/code/backend.json |  | 
2026-11-09 13:10:34 | 86bf5450d65d6c7a | backends/code/backend.json |  | 
2026-11-09 13:10:34 | aed746b26ccff265 | backends/code/backend.json |  | 
2026-11-09 13:10:34 | afc38e1d2e6f9864 | backends/code/backend.json |  | 
2026-11-09 13:10:34 | e48426c882ecdb6f | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 022b5f5781c5fde2 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 0b7d6e69b08b76c6 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 1237cf35bfe94dff | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 13c69f63e60000fd | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 15fc12c47ea6e8bd | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 1a36511e6bf1d1a6 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 362dad140d95673c | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 70233c79f67509f7 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 7fc8d620cebf5f13 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 8f5e00839fe65790 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | 9b86f18b731c859b | backends/code/backend.json |  | 
2026-11-09 13:10:35 | a7253ab38a11ed03 | backends/code/backend.json |  | 
2026-11-09 13:10:35 | b4bdda56dfb4620e | backends/code/backend.json |  | 
2026-11-09 13:10:35 | d161c4adf62dbe1e | backends/code/backend.json |  | 
2026-11-09 13:10:35 | e90b556b39e8e701 | backends/code/backend.json |  | 
2026-11-09 13:10:36 | 4caf894a9d17586e | backends/atelier/backend.json |  | 
2026-11-09 13:10:37 | 5ff9b5f30e0a80d6 | backends/code/backend.json | premier-index | 
2026-11-09 13:10:37 | 93457e7520e80f3e | backends/code/backend.json | premier-index | 
2026-11-09 13:10:37 | d2c598884a65914d | backends/code/backend.json | premier-index | 
2026-11-09 13:10:37 | e0889652c3c44752 | backends/code/backend.json | premier-index | 
2026-11-09 13:10:38 | 61443919479f05ff | backends/code/backend.json |  | 
2026-11-09 13:10:41 | 3cd014349c659d2d | — | premier-index | 
2026-11-09 13:10:42 | 865111940e8a49b6 | backends/code/backend.json |  | 
2026-11-09 13:13:07 | 21a499294d3507dc | backends/code/backend.json |  | 
2026-11-09 13:13:07 | c277e799afefb3ad | backends/code/backend.json |  | 
2026-11-09 13:13:07 | de5975b3150a498c | backends/code/backend.json |  | 
2026-11-09 13:13:07 | e93f9cdbaf7618f0 | backends/code/backend.json |  | 
2026-11-09 13:13:09 | 29bc411e5e4110bb | backends/code/backend.json | premier-index | 
2026-11-09 13:13:09 | 5b3aca7ae5504d88 | backends/code/backend.json | premier-index | 
2026-11-09 13:13:09 | f083ce295ac354fe | backends/code/backend.json | premier-index | 
2026-11-09 13:17:27 | e761ebb622feb710 | backends/atelier/backend.json |  | 
2026-11-09 13:17:27 | f8319a90b3bd9931 | backends/atelier/backend.json |  | 
2026-11-09 13:17:31 | 393ed4eff4d2f47f | backends/code/backend.json |  | 
2026-11-09 13:17:31 | 975e56880219d9ac | backends/code/backend.json |  | 
2026-11-09 13:17:31 | 9aadbd52fef3ad6c | backends/code/backend.json |  | 
2026-11-09 13:17:31 | 9d107bb66137ca0c | backends/code/backend.json |  | 
2026-11-09 13:17:31 | c0697711311e549e | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 166789d53c6b04df | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 2e770d94935a5975 | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 3512bb9a14ae0180 | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 60f3d10872967a12 | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 64663978a45bf83b | backends/code/backend.json |  | 
2026-11-09 13:17:32 | 9ed208c604219dfa | backends/code/backend.json |  | 
2026-11-09 13:17:32 | b9fe198acf7c134a | backends/code/backend.json |  | 
2026-11-09 13:17:32 | ecc5cceba2bd7419 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | 2d039fac59eb1dca | backends/code/backend.json |  | 
2026-11-09 13:17:33 | 58eadc2f7f876219 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | 91f36b89cc6ed185 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | ac88f3b3ee576a17 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | c8837cc25859f796 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | fca537d2c00d23a1 | backends/code/backend.json |  | 
2026-11-09 13:17:33 | ff9b3ad76dc50e18 | backends/code/backend.json |  | 
2026-11-09 13:17:34 | 038d0a076d24fc03 | backends/code/backend.json | premier-index | 
2026-11-09 13:17:34 | 0ae558d660de887f | backends/code/backend.json | premier-index | 
2026-11-09 13:17:34 | 6347bac026a4fcad | backends/code/backend.json | premier-index | 
2026-11-09 13:17:34 | 7ed9fecd8df85eff | backends/atelier/backend.json |  | 
2026-11-09 13:17:35 | 0fb0d3f72a57867d | backends/code/backend.json | premier-index | 
2026-11-09 13:17:35 | b74f135d6fc85483 | backends/code/backend.json |  | 
2026-11-09 13:17:39 | eb9be0e5fb4ef1fb | — | premier-index | 
2026-11-09 13:17:40 | 34782dd2677d4da2 | backends/code/backend.json |  | 
2026-11-09 13:19:23 | 55e6d6d4a84b03e4 | backends/code/backend.json |  | 
2026-11-09 13:19:23 | 97910d4ad2c4360f | backends/code/backend.json |  | 
2026-11-09 13:19:23 | 9f87fb5eabae8bc0 | backends/code/backend.json |  | 
2026-11-09 13:19:23 | b48b03362b1783ff | backends/code/backend.json |  | 
2026-11-09 13:19:25 | ad17c8c4e55854a0 | backends/code/backend.json | premier-index | 
2026-11-09 13:19:25 | b87b77a95d8cd030 | backends/code/backend.json | premier-index | 
2026-11-09 13:19:25 | e7b063c08d5659ec | backends/code/backend.json | premier-index | 
2026-11-09 13:24:25 | 29781499bb93adbd | backends/atelier/backend.json |  | 
2026-11-09 13:24:25 | 7a66782d4f6da1c0 | backends/atelier/backend.json |  | 
2026-11-09 13:24:29 | 0f52299a31cf7a20 | backends/code/backend.json | premier-index | 
2026-11-09 13:24:29 | 16290a99fb3b33a8 | backends/code/backend.json | premier-index | 
2026-11-09 13:24:29 | 1b7f73a73168166b | backends/code/backend.json |  | 
2026-11-09 13:24:29 | 38ea88f8281e75c2 | backends/code/backend.json |  | 
2026-11-09 13:24:29 | 3abf40dd340acc34 | backends/code/backend.json |  | 
2026-11-09 13:24:29 | 509d14264aad16db | backends/code/backend.json |  | 
2026-11-09 13:24:29 | 54429bb2044d39c9 | backends/code/backend.json | premier-index | 
2026-11-09 13:24:29 | 7618bf933f1c42f4 | backends/code/backend.json |  | 
2026-11-09 13:24:29 | 7c6256d1913c754b | backends/code/backend.json | premier-index | 
2026-11-09 13:24:29 | b2a77c3a03d2ea26 | backends/code/backend.json |  | 
2026-11-09 13:24:29 | e2fbe2e3f2c821f9 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 16fcdc7f135e1848 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 3f9eafa2a2a0f827 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 4b9ee362b904f50e | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 5eb2bec1ae5dabc9 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 684682bc929a1973 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 6fcfe79715cf741e | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 7a9ee0772aeb4e29 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 84ad6b1009d7cd14 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | 86f9d1832bddd756 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | c4b3a473262fa22f | backends/code/backend.json |  | 
2026-11-09 13:24:30 | e66309d178aff775 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | e9e892d848db44a3 | backends/code/backend.json |  | 
2026-11-09 13:24:30 | fdacd9873182263d | backends/code/backend.json |  | 
2026-11-09 13:24:31 | a5129b0694cf33f5 | backends/atelier/backend.json |  | 
2026-11-09 13:24:32 | 4523d1edcba210d4 | backends/code/backend.json | premier-index | 
2026-11-09 13:24:32 | 45fecf412e0b495d | backends/code/backend.json |  | 
2026-11-09 13:24:32 | 4f446d7093de64d9 | backends/code/backend.json | premier-index | 
2026-11-09 13:24:32 | 7bb95c5e64aeeaea | backends/code/backend.json | premier-index | 
2026-11-09 13:24:33 | a15253a01ef585bb | backends/code/backend.json | premier-index | 
2026-11-09 13:24:37 | 9ff9bd5110326e2b | — | premier-index | 
2026-11-09 13:24:38 | b57d1c7352b089b1 | backends/code/backend.json |  | 
2026-11-09 13:26:28 | 26d4bbeae797f126 | backends/code/backend.json |  | 
2026-11-09 13:26:28 | 7da3089f52072cd0 | backends/code/backend.json |  | 
2026-11-09 13:26:28 | c01c4cb3b7d0b75d | backends/code/backend.json |  | 
2026-11-09 13:26:28 | d7b5382eb8f2a6c9 | backends/code/backend.json |  | 
2026-11-09 13:26:29 | 0335ee96e027d0b8 | backends/code/backend.json | premier-index | 
2026-11-09 13:26:29 | 5e25ba009bf0726d | backends/code/backend.json | premier-index | 
2026-11-09 13:26:29 | c6b6cde40a18b12a | backends/code/backend.json | premier-index | 
```
