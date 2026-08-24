Searched the web
Searched the web
Voici le paysage actuel — et je parie que ton post LinkedIn portait sur l'un de ces deux-là, selon ce que « super petit » voulait dire.

Les candidats « vraiment minuscules »

PP-OCRv6 (PaddleOCR), sorti en juin 2026 : une série qui va de 1,5M à 34,5M de paramètres, avec +4,9% en détection et +5,1% en reconnaissance par rapport à PP-OCRv5, jusqu'à 5,2× plus rapide en inférence CPU via OpenVINO, 50 langues dans un modèle unifié, Apache 2.0. Le pitch officiel — « de 1,5M à 34,5M de paramètres, dépassant des VLM à l'échelle du milliard sur les tâches OCR » — est exactement le genre de formule qui tourne sur LinkedIn. C'est un pipeline classique détection+reconnaissance, pas un VLM : texte brut, pas de layout/formules, mais ça tourne sur CPU voire dans un navigateur. 
X
GitHub

Les candidats « petit VLM »

GLM-OCR (Z.ai/Zhipu), février 2026 : 0,9B paramètres, premier sur OmniDocBench V1.5 avec 94,62, poids sous licence MIT. Il a battu Qwen3-VL-235B — 260× plus gros — et dépassé les 3 millions de téléchargements sur Hugging Face son premier mois. C'est probablement le plus gros buzz OCR de l'année. 
NYU Shanghai RITS
Towards AI

Les autres de la même famille :

Modèle	Params	Licence	Point fort
Surya	650M	Apache 2.0	90+ langues, rapide même CPU, 83,3% olmOCR-bench
GOT-OCR 2.0	580M	Apache 2.0	Formules, partitions musicales
Dolphin (ByteDance)	~400M	Apache 2.0	Analyze-then-parse, layouts complexes
LightOnOCR-2	1B	Apache 2.0	Labo français, langues européennes
dots.ocr	1.7B	Apache 2.0	N'importe quel script
DeepSeek-OCR 2	~1.3B	MIT	Compression optique de contexte
Deux réserves : les chiffres divergent selon les sources (une comparaison d'août donne GLM-OCR à ~500M alors que Z.ai annonce 0,9B, et confond PaddleOCR-VL avec PP-OCRv6 qui sont deux choses distinctes). Et le retour terrain sur GLM-OCR en local est mitigé — un test sur MacBook Pro M3 Max a dû retomber sur CPU faute de mémoire côté Metal, environ une minute par page. 
Fractional

Vu ton contexte (souveraineté, Rust, self-host), LightOnOCR-2 et PP-OCRv6 sont les deux à regarder en priorité — le premier pour le français, le second parce qu'à 34M de paramètres avec export ONNX tu peux l'embarquer sans stack Python. Si tu me dis ce que le post mettait en avant (le chiffre de paramètres ? un benchmark ? un labo ?), je peux resserrer.