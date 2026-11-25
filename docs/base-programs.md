# Programmi base: cosa c'è già (visualizzatore di testo)

> Stato su main. Tutto è verificato in libmgba e nei test host, non su hardware reale.
> Il file manager è del thread dei programmi base: qui c'è solo il visualizzatore, come programma richiamabile.

## Interfaccia del programma

- **Superficie:** programma 0, 64×48 pixel, posizione (10, 20). Aperta e chiusa dal monolite con quota (`gba/src/surfaces.rs`).
- **Entrata:** `run_editor(surf, m, testo, salva)` in `gba/src/main.rs`.
  - `testo`: byte da mostrare, fino a 128 caratteri.
  - `salva`: callback chiamata da Select con la riga corrente. Ritorna `true` se il salvataggio riesce.
- **Uscita:** Start. La superficie viene chiusa e la quota torna al monolite.

## Comandi (keypad)

| Tasto | Azione |
|---|---|
| Su / Giù | Cambia il carattere in attesa (arancio) |
| A | Aggiunge il carattere in attesa |
| B | Cancella l'ultimo carattere |
| Select | Salva (riga di stato "SALVATO" o "ERRORE") |
| Start | Esce |

## Dove viene usato

- **Demo all'avvio:** apre `docs/hello` e lo salva in `docs/hello`.
- **Spazio di lavoro:** A su un file lo apre nel visualizzatore, con salvataggio nello stesso file. Start torna allo spazio di lavoro.

## Moduli

- `gba/src/glyph.rs`: font 3×5 e disegno di testo (test host).
- `gba/src/editor.rs`: riga editabile con limite (test host).
- `gba/src/surfaces.rs`: superfici con quota (test host, 10 test).
- `gba/src/compositor.rs`: composizione con trasparenza (test host, 6 test).

## Limiti noti

- 128 caratteri per file: il resto viene tagliato.
- A capo ogni 15 caratteri. Si vedono solo le ultime 6 righe: il cursore è sempre in fondo, non c'è scorrimento.
- Il salvataggio è verificato solo nella SRAM di libmgba (`gba/host/vram_check.c`, modalità `save`).

## Da verificare

- Tempi di scrittura della SRAM su hardware reale.
- Modifica nel mezzo del testo: non supportata, si lavora solo in fondo.
