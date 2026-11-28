# Brief 3: ultime opzioni senza effetto visivo

Input: `/home/user/HaikuOS/design/configurator/questions.json`.
Mappe già decise: `/home/user/HaikuOS/design/configurator/map/m*.json` e `r*.json`. Un'opzione è "scoperta" se nella mappa unita non ha nessuna chiave che non inizi con `_`.

Passo 1: calcola le opzioni scoperte del TUO intervallo (domande indicate nel prompt), con questo script:
```
python3 -c "import json,glob;m={}\nfor f in sorted(glob.glob('map/[mr]*.json')):m.update(json.load(open(f)))\nq=json.load(open('questions.json'))\nfor x in q:\n for o in x['options']:\n  if not any(not k.startswith('_') for k in m.get(o['id'],{})):print(o['id'],'|',o['label'])"
```
(eseguilo dalla cartella `/home/user/HaikuOS/design/configurator`, filtra per il tuo intervallo di domande.)

Passo 2: per OGNI opzione scoperta decidi:
A) ha un effetto visivo: mappa con chiavi e valori AMMESSI:
- bar: top|bottom|none · title: left|center|hidden · ctl: left|right · shape: square|circle
- shadow: none|soft|strong · theme: light|dark · tb: neutral|accent|translucent|inactive
- border: hair|thin|heavy|none · btn: solid|outline|pill|ghost · field: outline|filled|underline
- switch: pill|square · icons: filled|line|rounded · hover: lighten|underline|none
- tip: on|off · font: sans|mono|serif|bold · scroll: thin|wide|hidden · dock: bottom|left|right|none
- barh: 2|3|4|5 · tbh: 2|3|4 · dockicon: 3|4.4|6|8 · radm: 0|1|2|3|4 · pad: 1|2|3 · fs: 1.5|2|2.5 (numeri)
- Widget (additivi, valore sempre "on"): w_checkbox (caselle), w_radio (pulsanti radio), w_slider (cursore), w_progress (barra di avanzamento), w_menu (menu a tendina), w_list (lista), w_tabs (schede), w_dialog (finestra di dialogo), w_toast (notifica), w_focus (focus da tastiera sui campi), w_error (stato di errore sui campi), w_disabled (pulsanti disabilitati), w_sidebar (barra laterale), w_badge (badge numerico sulle icone)
B) non ha effetto visivo (suoni, aptica, tempi, comportamento puro): `{"_nv": true}`.

Regole:
- Usa A se l'opzione cambia davvero ciò che si vede; usa un widget se descrive un controllo o uno stato dell'interfaccia.
- Ogni opzione scoperta del tuo intervallo DEVE avere una decisione: A oppure B. Non lasciare nessuna opzione fuori.
- Non inventare chiavi o valori. Aggiungi `"_f"` alle opzioni A (formula breve in u, solo documentazione).

Output: scrivi UN oggetto JSON nel file indicato, chiave = id opzione, valore = mappa. Verifica con `python3 -c "import json;json.load(open('FILE'))"`. Risposta finale: UNA riga con numero di A, numero di B, numero totale. Non incollare il JSON.
