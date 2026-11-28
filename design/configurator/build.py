#!/usr/bin/env python3
"""Unisce parts/*.json, valida, scrive questions.json e configurator.html (dati inline)."""
import json, glob, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = {"macOS", "Windows", "GNOME", "KDE"}

qs = []
for f in sorted(glob.glob(os.path.join(HERE, "parts", "p*.json"))):
    qs += json.load(open(f, encoding="utf-8"))

errs, ids = [], set()
for i, q in enumerate(qs, 1):
    q["id"] = "q%03d" % i
    if len(q.get("options", [])) < 5:
        errs.append(f"{q['id']}: meno di 5 opzioni")
    for o in q.get("options", []):
        o["id"] = f"{q['id']}-{o['id']}"  # id globale unico
        if o["id"] in ids:
            errs.append(f"opzione duplicata {o['id']}")
        ids.add(o["id"])
        if not set(o.get("sources", [])) <= SRC:
            errs.append(f"{q['id']}/{o['id']}: fonte sconosciuta")
        if not isinstance(o.get("customizable"), bool):
            errs.append(f"{q['id']}/{o['id']}: customizable non booleano")
if errs:
    sys.exit("\n".join(errs))

json.dump(qs, open(os.path.join(HERE, "questions.json"), "w", encoding="utf-8"),
          ensure_ascii=False, indent=1)

mp = os.path.join(HERE, "preview-map.json")
pmap = json.load(open(mp, encoding="utf-8")) if os.path.exists(mp) else {}
pmap = {k: v for k, v in pmap.items() if k in ids}

html = open(os.path.join(HERE, "template.html"), encoding="utf-8").read()
html = html.replace("/*DATA*/null", json.dumps(qs, ensure_ascii=False))
html = html.replace("/*MAP*/null", json.dumps(pmap, ensure_ascii=False))
open(os.path.join(HERE, "configurator.html"), "w", encoding="utf-8").write(html)

opts = sum(len(q["options"]) for q in qs)
cust = sum(1 for q in qs for o in q["options"] if o["customizable"])
cats = sorted({q["category"] for q in qs})
print(f"domande {len(qs)} · opzioni {opts} · min opzioni/domanda "
      f"{min(len(q['options']) for q in qs)} · suggerite personalizzabili {cust} · "
      f"categorie {len(cats)} · mappate preview {len(pmap)}")
