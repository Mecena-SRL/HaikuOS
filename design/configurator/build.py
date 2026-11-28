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
VOCAB = {
 "bar": ["top", "bottom", "none"], "title": ["left", "center", "hidden"],
 "ctl": ["left", "right"], "shape": ["square", "circle"],
 "shadow": ["none", "soft", "strong"], "theme": ["light", "dark"],
 "tb": ["neutral", "accent", "translucent", "inactive"],
 "border": ["hair", "thin", "heavy", "none"], "btn": ["solid", "outline", "pill", "ghost"],
 "field": ["outline", "filled", "underline"], "switch": ["pill", "square"],
 "icons": ["filled", "line", "rounded"], "hover": ["lighten", "underline", "none"],
 "tip": ["on", "off"], "font": ["sans", "mono", "serif", "bold"],
 "scroll": ["thin", "wide", "hidden"], "dock": ["bottom", "left", "right", "top", "none"],
 "barh": [2, 3, 4, 5], "tbh": [2, 3, 4], "dockicon": [3, 4.4, 6, 8],
 "radm": [0, 1, 2, 3, 4], "pad": [1, 2, 3], "fs": [1.5, 2, 2.5],
 "w_checkbox": ["on"],  "w_radio": ["on"],  "w_slider": ["on"],  "w_progress": ["on"],  "w_menu": ["on"],  "w_list": ["on"],
 "w_tabs": ["on"],  "w_dialog": ["on"],  "w_toast": ["on"],  "w_focus": ["on"],  "w_error": ["on"],  "w_disabled": ["on"],
 "w_sidebar": ["on"],  "w_badge": ["on"],
 "clk": ["time", "sec", "dow", "full", "none"], "icn": [5, 7, 9], "lh": [1.2, 1.45, 1.8],
 "grid": ["free", "rows", "cols"], "dot": ["on"], "start": ["corner", "center", "left"],
 "cur": ["arrow", "large", "cross", "hand"], "grip": ["on"], "ghost": ["on"], "split": ["on"],
 "ovw": ["on"], "notif": ["on"], "lock": ["on"], "widget": ["on"], "ctlhover": ["red"], "acc": ["navy", "gold"], 
}
pmap = {}
for f in sorted(glob.glob(os.path.join(HERE, "map", "[mrgh]*.json"))):
    for oid, mm in json.load(open(f, encoding="utf-8")).items():
        has = any(not k.startswith("_") for k in mm) or bool(mm.get("_nv"))
        if has or oid not in pmap:
            pmap[oid] = mm
bad = []
for oid, m in pmap.items():
    for k, v in m.items():
        if k.startswith("_"):
            continue
        if k not in VOCAB or v not in VOCAB[k]:
            bad.append(f"{oid}: {k}={v}")
if bad:
    sys.exit("valori non ammessi:\n" + "\n".join(bad))
pmap = {k: v for k, v in pmap.items() if k in ids}

html = open(os.path.join(HERE, "template.html"), encoding="utf-8").read()
html = html.replace("/*DATA*/null", json.dumps(qs, ensure_ascii=False))
html = html.replace("/*MAP*/null", json.dumps(pmap, ensure_ascii=False))
html = html.replace("/*VOCAB*/null", json.dumps(VOCAB, ensure_ascii=False))
open(os.path.join(HERE, "configurator.html"), "w", encoding="utf-8").write(html)

opts = sum(len(q["options"]) for q in qs)
cust = sum(1 for q in qs for o in q["options"] if o["customizable"])
cats = sorted({q["category"] for q in qs})
print(f"domande {len(qs)} · opzioni {opts} · min opzioni/domanda "
      f"{min(len(q['options']) for q in qs)} · suggerite personalizzabili {cust} · "
      f"categorie {len(cats)} · mappate preview {len(pmap)}")
