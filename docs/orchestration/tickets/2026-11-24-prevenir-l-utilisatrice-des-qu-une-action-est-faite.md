# Prévenir l'utilisatrice dès qu'une action demandée est faite

Ouvert le 24 novembre 2026, 13 h. Constat de Lucie le même jour, dans `tisserande` : à « tu peux lancer le projet et me
donner le lien pour aller dans mon navigateur ? », l'agent lance le serveur Vite en fond, puis continue (tests, lecture
de `vite.config.ts`, plan) sans donner le lien tout de suite. Elle attend.

## Ce qui est proposé

1. **Un outil `notify_user`** (Lucie : « quand tu as fait une des actions que demandait l'utilisatrice, notifie-la
   immédiatement via cet outil ») : un graphe-outil déclaré qui pose tout de suite, en plein tour, une ligne pour
   l'utilisatrice (`kind: notice`), avec un texte court et, au besoin, un lien ou un chemin. Le terminal et l'IDE
   l'affichent en évidence et à part du texte de la réponse ; elle reste dans la boîte. Une phrase de l'invite (un
   gabarit) : dès qu'une action que la demande attend est faite (un serveur lancé, un fichier écrit, une commande
   finie), `notify_user` avec ce qui sert : le lien, le chemin, le résultat.
2. **Le filet déterministe, sans compter sur le modèle** (c7) : la sortie d'une commande lancée en fond est déjà
   suivie (son journal). Une règle déclarée y repère une adresse servie (`http://localhost:5173/`, `127.0.0.1:<port>`,
   la ligne « Local: » de Vite, de Next, de webpack, de Jupyter…) et la pose aussitôt pour l'utilisatrice, une fois par
   adresse : « le serveur répond à http://localhost:5173/ ». Les motifs vivent dans un fichier livré, extensibles par
   l'utilisatrice.

## Le témoin

Par un vrai hôte et un faux modèle : une commande en fond qui écrit `Local: http://localhost:5173/` fait poser la ligne
pour l'utilisatrice avant la fin du tour ; un appel à `notify_user` la pose avant le prochain appel au modèle.
