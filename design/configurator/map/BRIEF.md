# Brief: regole visive per le opzioni del configuratore

Input: `/home/user/HaikuOS/design/configurator/questions.json` (array di domande: `id` tipo "q012", `question`, `formula`, `options[]` con `id` tipo "q012-raggio-3u" e `label`).
Lavora SOLO sulle domande del tuo intervallo (indicato nel prompt).

Per OGNI opzione del tuo intervallo decidi cosa cambia visivamente nell'anteprima, usando SOLO queste chiavi e valori:

- bar: top|bottom|none            (barra dei menu; none = nascosta)
- title: left|center|hidden       (allineamento titolo finestra / nascosto)
- ctl: left|right                 (lato dei pulsanti di controllo)
- shape: square|circle            (forma dei pulsanti di controllo)
- shadow: none|soft|strong        (ombra della finestra)
- theme: light|dark
- tb: neutral|accent|translucent|inactive   (aspetto della barra del titolo)
- border: hair|thin|heavy|none    (bordo finestra; hair = 1 px fisso, thin/heavy scalano con u)
- btn: solid|outline|pill|ghost   (aspetto dei pulsanti)
- field: outline|filled|underline (aspetto dei campi di testo)
- switch: pill|square
- icons: filled|line|rounded      (aspetto delle icone desktop)
- hover: lighten|underline|none   (feedback al passaggio del mouse)
- tip: on|off                     (suggerimenti)
- font: sans|mono|serif|bold
- scroll: thin|wide|hidden        (barra di scorrimento nella finestra)
- dock: bottom|left|right|none    (posizione del dock)
- barh: 2|3|4|5                   (altezza barra dei menu, in u; NUMERO)
- tbh: 2|3|4                      (altezza barra del titolo, in u; NUMERO)
- dockicon: 3|4.4|6|8             (dimensione icone dock, in u; NUMERO)
- radm: 0|1|2|3|4                 (raggio angoli, in u; NUMERO)
- pad: 1|2|3                      (padding interno finestra, in u; NUMERO)
- fs: 1.5|2|2.5                   (dimensione testo, in u; NUMERO)

Regole:
- Mappa solo le opzioni con un effetto visivo chiaro su queste chiavi. Le opzioni senza effetto visivo (suoni, aptica, tempi di animazione, comportamento) ricevono `{}`.
- Usa la chiave che corrisponde meglio. Non inventare chiavi o valori.
- Una opzione può impostare più chiavi, ad esempio `{"barh": 4, "bar": "top"}`.
- Aggiungi `"_f"` a ogni opzione mappata: una formula breve in u, ad esempio `"barra h = 4u"`. È solo documentazione.

Output: scrivi UN oggetto JSON nel file indicato nel prompt con lo strumento Write. Chiavi = id di TUTTE le opzioni del tuo intervallo (anche quelle `{}`), valori = oggetti. Verifica con `python3 -c "import json;json.load(open('FILE'))"`. Risposta finale: UNA riga con numero di opzioni mappate e numero vuote. Non incollare il JSON nella risposta.
