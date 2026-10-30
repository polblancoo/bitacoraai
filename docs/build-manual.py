#!/usr/bin/env python3
"""Arma el manual embebido en la app (crates/server/web/manual/{es,en,pt}.md) a partir de las guías de docs/.
Uso: python3 docs/build-manual.py"""
import pathlib, re
root = pathlib.Path(__file__).resolve().parent
out = root.parent / "crates/server/web/manual"
out.mkdir(parents=True, exist_ok=True)
parts = {"es": ("GUIA.md", "AGENTES.md"), "en": ("GUIDE.md", "AGENTS.md"), "pt": ("GUIA.pt.md", "AGENTES.pt.md")}
for lang, files in parts.items():
    texts = []
    for f in files:
        t = (root / f).read_text()
        t = re.sub(r'\A<p align="center">.*?</p>\n+', "", t, flags=re.S)  # quita el selector de idioma de GitHub
        texts.append(t.strip())
    (out / f"{lang}.md").write_text("\n\n---\n\n".join(texts) + "\n")
    print(lang, sum(len(t) for t in texts), "bytes")

# capturas usadas por el manual: se copian junto al servidor (la política de seguridad de la app no deja cargar imágenes externas)
shots = root / "screenshots"
img = out / "img"
img.mkdir(exist_ok=True)
names = sorted({n for lang in parts for n in re.findall(r"screenshots/([\w.-]+\.png)", (out / f"{lang}.md").read_text())})
import shutil
for n in names:
    shutil.copy(shots / n, img / n)
rs = ["// Generado por docs/build-manual.py: capturas del manual embebidas en el binario.", "pub const IMGS: &[(&str, &[u8])] = &["]
rs += [f'    ("{n}", include_bytes!("../web/manual/img/{n}")),' for n in names]
rs += ["];", ""]
(root.parent / "crates/server/src/manual_img.rs").write_text("\n".join(rs))
print(len(names), "capturas")
