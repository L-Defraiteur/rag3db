# Fixtures OCR

`hello.png` — 400×120, fond blanc, deux lignes de texte noir en DejaVu Sans :
« Hello rag3weaver » (34 px, à (20, 14)) et « OCR 2026 » (28 px, à (20, 68)).
Produite avec Pillow (`Image.new("RGB", (400, 120), "white")`, `ImageDraw.text`,
`ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSans.ttf", …)`, `save(optimize=True)`), 6,5 Ko.
Sert à `tests/e2e_burn_ocr.rs` et à `examples/burn_ppocr_vs_onnxruntime.rs`.
