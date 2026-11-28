# Brief 2: revisione delle opzioni senza effetto visivo

Input: `/home/user/HaikuOS/design/configurator/questions.json`. Lavora SOLO sul tuo intervallo di domande.
Le mappe esistenti sono in `/home/user/HaikuOS/design/configurator/map/m*.json`: leggile per il tuo intervallo.
Un'opzione con mappa `{}` (o senza chiavi non "_") oggi non cambia l'anteprima.

Obiettivo: ogni opzione deve diventare VISIBILE nell'anteprima, oppure essere dichiarata esplicitamente non visiva.

Per ogni opzione con mappa vuota decidi:
A) ha un effetto visivo reale sull'anteprima: scrivi la mappa con SOLO queste chiavi e valori:
- bar: top|bottom|none · title: left|center|hidden · ctl: left|right · shape: square|circle
- shadow: none|soft|strong · theme: light|dark · tb: neutral|accent|translucent|inactive
- border: hair|thin|heavy|none · btn: solid|outline|pill|ghost · field: outline|filled|underline
- switch: pill|square · icons: filled|line|rounded · hover: lighten|underline|none
- tip: on|off · font: sans|mono|serif|bold · scroll: thin|wide|hidden · dock: bottom|left|right|none
- barh: 2|3|4|5 · tbh: 2|3|4 · dockicon: 3|4.4|6|8 · radm: 0|1|2|3|4 · pad: 1|2|3 · fs: 1.5|2|2.5   (numeri)
B) non ha effetto visivo (suoni, aptica, tempi di animazione, comportamento puro): scrivi `{"_nv": true}`.

Regole:
- Preferisci A quando un'opzione cambia davvero aspetto (colore, forma, posizione, dimensione, bordo, ombra, font, icone, pulsanti, campi, scorrimento, tooltip).
- Non cambiare le opzioni che hanno già una mappa visiva.
- Non inventare chiavi o valori.
- Aggiungi `"_f"` a ogni opzione in A: formula breve in u, solo documentazione.

Output: scrivi UN oggetto JSON nel file indicato, con chiave = id opzione e valore = mappa. Includi SOLO le opzioni con mappa vuota del tuo intervallo (sia A sia B). Verifica con `python3 -c "import json;json.load(open('FILE'))"`. Risposta finale: UNA riga con numero di opzioni in A e in B. Non incollare il JSON nella risposta.
