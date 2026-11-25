# Concept di ambiente grafico (primo schizzo)

> Concept, non specifica. Ogni misura non provata è segnata **da verificare**.
> Base tecnica: [graphics.md](graphics.md) (compositor, superfici, chiave di trasparenza).

## 1. Idea

Un desktop a finestre, pensato prima per GBA (240×160, 15 bit) e poi esteso a schermi più grandi e al touch. Regola unica: **i programmi disegnano nella loro superficie, il compositor decide cosa si vede**.

## 2. Schermata principale (GBA)

```
+------------------------------------------------------------+ <- barra 240×12
| HaikuOS            [term] [files]               12:34      |
+------------------------------------------------------------+
|  [ico]    [ico]    [ico]        +-----------------------+  |
|  files    term     giochi       | Terminale             |  |
|                                 |                       |  |
|  [ico]                          |  $ ls                 |  |
|  notes                          |  docs/  gba/          |  |
|                                 +-----------------------+  |
+------------------------------------------------------------+
```

- **Barra superiore**: 12 px di altezza. Nome del sistema a sinistra, programmi aperti al centro, orologio a destra (da verificare: l'orologio richiede il timer).
- **Icone**: 16×16 px, griglia 4 colonne con passo di 48 px. Testo di 8 px sotto l'icona (da verificare: leggibilità con il font a tile).
- **Sfondo**: un colore pieno o un tile ripetuto. Nessuna immagine intera (RAM).
- **Cursore**: uno sprite hardware, non parte del buffer (graphics.md, sezione 4).

## 3. Finestre

- Barra del titolo 10 px con il nome del programma. Il pulsante chiudi è un quadrato 8×8 a destra.
- Corpo: la superficie del programma, con bordo di 1 px. Quota per finestra: da verificare contro i 32 KB di architecture.md (una finestra 120×80 a 15 bit occupa circa 19 KB: entra).
- Ordine a strati: la finestra attiva sta sopra le altre. Nessuna finestra può disegnare fuori dalla propria superficie (taglio del compositor).
- Trascinamento e ridimensionamento: non nella prima versione.

## 4. File manager e terminale

- **File manager**: elenco a righe di testo, una riga per file, con icona piccola a sinistra. Scorrimento con su/giù.
- **Terminale**: una finestra con griglia di caratteri (da verificare: 30 colonne × 16 righe con font 8 px). Comandi Linux di base (`ls`, `cd`, `cat`, `echo`) sulla stessa struttura del filesystem di `docs/filesystem-roadmap.md`. Quale parte dei comandi entra nella prima versione: da decidere.

## 5. Navigazione con il keypad del GBA

| Tasto | Azione |
|---|---|
| Croce | Muove il cursore, o la selezione dentro una finestra |
| A | Apre l'icona o conferma |
| B | Chiude il menu o torna indietro |
| Start | Apre il menu del sistema |
| Select | Cambia finestra attiva |
| L / R | Scorre tra le finestre aperte |

Il cursore è uno sprite 8×8 e si muove di 1 px per frame, con passo rapido tenendo premuto (da verificare: sensazione di controllo).

## 6. Schermi più grandi

- Le misure sono in **unità logiche** (1 unità = 1 px GBA). Su uno schermo più grande l'unità diventa 2×, 3× o più, senza cambiare il layout.
- Più finestre visibili insieme. La barra diventa più alta e mostra più testo.
- Stesso protocollo: il programma consegna una superficie, il compositor la compone (graphics.md, sezione 5).

## 7. Telefono (touch)

- Una finestra a tutto schermo per volta. La barra va in basso, con i programmi come pulsanti.
- Tocco = clic; trascinamento da bordo = cambio finestra. Niente cursore hardware.
- Icone da almeno 48 px fisici (da verificare: misura reale del display del telefono).

## 8. Cosa mostra il primo schizzo

Nel compositore (`gba/src/compositor.rs`) un test host compone tre elementi: sfondo, barra, una finestra con icona trasparente. Il test legge i pixel del framebuffer, come già fatto per la demo. La ROM mostra la stessa scena dopo la demo delle superfici (premi A). Verificata in VRAM con libmgba (`gba/host/desktop_check.c`, 0 pixel diversi). Non ancora su hardware reale.

## 9. Domande aperte

1. Estetica: colori e tono visivo (colori caldi, freddi, a 4 o 16 tinte?) — da decidere con Ivan.
2. Font: un font di 8 px disegnato a mano, o uno standard già libero?
3. Il terminale è la prima app o viene dopo il file manager? Cambia l'ordine dei lavori.
