# Programmi base

> Piano, non specifica. Ogni misura non provata è segnata **da verificare**.
> Base: [desktop-concept.md](desktop-concept.md) (finestre, tasti), [graphics.md](graphics.md) (superfici), [filesystem-roadmap.md](filesystem-roadmap.md) (file system), [resource-microkernels.md](resource-microkernels.md) (quote).

## Regola

Ogni programma è un processo con una sola superficie. Usa solo:
- **File system**: lettura via `Fs` (directory, file), scrittura solo con la quota concessa dal monolite.
- **Superfici**: disegna nel proprio buffer, il compositore decide cosa si vede.
- **Risorse**: memoria e slot dal monolite; nessun accesso diretto all'hardware.

## Elenco

| # | Programma | Scopo | Usa | Stato | Da verificare sul GBA |
|---|---|---|---|---|---|
| 1 | File manager | Elenca e apre directory e file | File system (lettura), superficie | Sulla ROM: spazio di lavoro (`gba/src/workspace.rs`), su/giù, A apre, B su, Start crea file. Nessuno scorrimento: al massimo 16 voci entrano nella griglia | Leggibilità del testo a 8 px |
| 2 | Calcolatrice | Somma, sottrazione, moltiplicazione, divisione intere su tastierino | Superficie, tastiera Sulla ROM sul desktop, con superficie propria (`gba/src/calc_ui.rs`, logica in `gba/src/calc.rs`, test host) | Tastierino dentro la superficie; overflow a 16 bit (`i32` in centesimi) |
| 3 | Editor di testo | Modifica un file di testo | File system (lettura/scrittura), superficie | Parziale: visualizzatore/editor di riga (vedi sotto) | Dimensione massima file (8 KB totali nello store) |
| 4 | Task manager | Elenca programmi attivi, memoria e slot, chiude un programma | Monolite (stato), superficie | Nuovo | Dati letti dal monolite senza costi extra |
| 5 | Visualizzatore di immagini | Mostra una immagine a 15 bit già pronta | File system (lettura), superficie | Nuovo | Formato immagine; RAM per una superficie piena |
| 6 | Impostazioni / orologio | Ora, nome del sistema, tema (4 tinte) | Risorse (timer), file system (salvataggio) | Nuovo | Timer per l'orologio (desktop-concept, §2) |
| 7 | Terminale | Comandi base (`ls`, `cd`, `cat`, `echo`) | File system, superficie a caratteri | Nuovo, dopo il file manager | Griglia 30×16 caratteri (desktop-concept, §4) |

## Priorità

1. **File manager**: serve per vedere lo store e per testare le quote con dati reali.
2. **Editor di testo**: il primo uso reale (Ivan: documenti). Riusa l'editor di riga.
3. **Calcolatrice**: piccola, ottimo banco di prova per il tastierino e le superfici.
4. **Task manager**: mostra la memoria che già il monolite conosce.
5. **Impostazioni / orologio**, poi **visualizzatore di immagini**, poi **terminale**.

## Note

- Ogni programma ha un limite di superficie e di memoria fissato dal monolite. Il programma deve reggere il rifiuto della quota (`Denied`) senza bloccarsi.
- I programmi non parlano direttamente con il file system: passano dalla lettura di `Fs` già concessa. Da verificare: come si concede la lettura a un programma, e se oggi il monolite è l'unico a scrivere.
- Il file manager è prima di tutto logica pura: cursore, scorrimento, navigazione. Il disegno arriva dopo, sulla superficie.
- Prima versione per ogni programma: solo le funzioni indispensabili. Estetica e scorciatoie dopo.

---

## Stato: visualizzatore di testo (su main)

> Stato su main. Tutto è verificato in libmgba e nei test host, non su hardware reale.
> Il file manager è del thread dei programmi base: qui c'è solo il visualizzatore, come programma richiamabile.

### Interfaccia del programma

- **Superficie:** programma 0, 64×48 pixel, posizione (10, 20). Aperta e chiusa dal monolite con quota (`gba/src/surfaces.rs`).
- **Entrata:** `run_editor(surf, m, testo, salva)` in `gba/src/main.rs`.
  - `testo`: byte da mostrare, fino a 128 caratteri.
  - `salva`: callback chiamata da Select con la riga corrente. Ritorna `true` se il salvataggio riesce.
- **Uscita:** Start. La superficie viene chiusa e la quota torna al monolite.

### Comandi (keypad)

| Tasto | Azione |
|---|---|
| Su / Giù | Cambia il carattere in attesa (arancio) |
| A | Aggiunge il carattere in attesa |
| B | Cancella l'ultimo carattere |
| Select | Salva (riga di stato "SALVATO" o "ERRORE") |
| Start | Esce |

### Dove viene usato

- **Demo all'avvio:** apre `docs/hello` e lo salva in `docs/hello`.
- **Spazio di lavoro:** A su un file lo apre nel visualizzatore, con salvataggio nello stesso file. Start torna allo spazio di lavoro.

### Moduli

- `gba/src/glyph.rs`: font 3×5 e disegno di testo (test host).
- `gba/src/editor.rs`: riga editabile con limite (test host).
- `gba/src/surfaces.rs`: superfici con quota (test host, 10 test).
- `gba/src/compositor.rs`: composizione con trasparenza (test host, 6 test).

### Limiti noti

- 128 caratteri per file: il resto viene tagliato.
- A capo ogni 15 caratteri. Si vedono solo le ultime 6 righe: il cursore è sempre in fondo, non c'è scorrimento.
- Il salvataggio è verificato solo nella SRAM di libmgba (`gba/host/vram_check.c`, modalità `save`).

### Da verificare

- Tempi di scrittura della SRAM su hardware reale.
- Modifica nel mezzo del testo: non supportata, si lavora solo in fondo.
