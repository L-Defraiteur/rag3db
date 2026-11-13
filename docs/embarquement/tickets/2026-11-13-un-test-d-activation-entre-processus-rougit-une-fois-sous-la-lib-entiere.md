# Un test d'activation entre processus rougit une fois sous la lib entière

- **État** : ouvert (13 novembre 2026), relevé par arbre principal pendant le lot du raccord physique.
- **Pour** : embarquement (décision de c7, 13 novembre 2026).
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; une fois sur sept tours.
- **Atteignable en service** : à départager.
- **Touche rag3weaver** : oui (`src/model_source/activation.rs`).

## Ce que c'est

`model_source::activation::tests::profil_etranger_entre_processus_est_refuse` a rougi une fois,
au tour 3 de trois tours de la lib entière, joués sur master `73c7f15e1` sur lucie-tablette :

```
thread 'model_source::activation::tests::profil_etranger_entre_processus_est_refuse' panicked at src/model_source/activation.rs:196:55:
test result: FAILED. 2307 passed; 1 failed; 67 ignored
```

- **Les conditions** : lib `207701bb2`, sans `MOTEUR_ANCIEN`, jeu large, 8 jobs, fils de test par
  défaut, sans `RAG3WEAVER_EMBED_SERVICE`, sous `poste lourd`.
- **Les six autres tours** : verts, avec ou sans les services.
- **Ce qui manque** : le message de la panique. Le journal ne gardait que les lignes filtrées.

La ligne 196 est `connect_embedder(&f.source).unwrap()` dans le processus parent, avant le
processus enfant. C'est donc l'activation de la fixture elle-même qui a échoué, pas le refus du
profil étranger.

## Ce qu'il faut pour le fermer

Retrouver le message, en rejouant la lib entière avec la sortie complète gardée. Puis départager
une limite de temps sous charge d'une vraie erreur de l'activation.
