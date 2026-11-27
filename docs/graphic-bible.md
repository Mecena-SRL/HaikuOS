# Bibbia grafica di HaikuOS

> Regole vincolanti per ogni schermata. Vale per GBA (240×160, 15 bit) e per Mac (sRGB).
> Base: [desktop-concept.md](desktop-concept.md) (4 tinte) e [graphics.md](graphics.md) (compositor).
> Ogni valore non misurato su hardware è segnato **da verificare**.

## 1. Principi

1. **Pochi colori.** Quattro tinte, nessun'altra. Su Mac non si aggiungono colori di accento.
2. **Un ruolo, un token.** Ogni elemento usa solo il token del suo ruolo (sezione 3). Niente valori a mano.
3. **Essenziale.** Niente gradienti, ombre, trasparenze, animazioni decorative.
4. **Stesso disegno, scala diversa.** GBA e Mac mostrano la stessa interfaccia; cambia solo la scala.

## 2. Ispirazione

| Da | Prendiamo | Scartiamo |
|---|---|---|
| **GNOME** (Adwaita) | Una sola barra in alto; titoli sobri; spazi generosi; poche icone | Barra del titolo doppia; ombre |
| **KDE Plasma** (Breeze) | Pannello con orologio e programmi aperti; finestre con titolo e pulsante chiudi | Temi multipli attivi di default; decorazioni |

## 3. Palette (4 token)

Formato GBA: `r | g<<5 | b<<10` (5 bit per canale). Mac: sRGB esadecimale, conversione `(v<<3)|(v>>2)`.

| Token | Ruolo | GBA (15 bit) | Mac |
|---|---|---|---|
| **T0** | Sfondo; testo su barra e su titolo attivo | (2, 4, 7) | `#102139` |
| **T1** | Superfici: corpo finestra, testo normale, icona selezionata | (30, 29, 28) | `#F7EFE7` |
| **T2** | Accento: titolo finestra attiva, icone | (27, 15, 12) | `#DE7B63` |
| **T3** | Barra superiore, bordi finestra | (29, 25, 17) | `#EFCE8C` |

Neutri ammessi solo su Mac: nessuno per ora. Se servono (es. testo disabilitato), si aggiungono qui e nel codice insieme.

### 3.1 x86: colori a 32 bit (solo x86)

Il framebuffer x86 è a 32 bit: i quattro token restano il tema (T0..T3), ma si aggiungono accenti fissi `A(k)` (`gui/src/palette.rs`, `ACCENTS`):

| Indice | Uso | Colore |
|---|---|---|
| A(1)..A(8) | Titolo di ogni app (FILE, TERM, CALC, WEB, EDIT, SET, ORA, READ) | giallo, verde acqua, arancio, blu, lilla, grigio azzurro, rosa, verde |
| A(9), A(10), A(11) | Pulsanti riduci, massimizza, chiudi | giallo, verde, rosso |

Su GBA `A(_)` vale T2: il GBA non usa mai questi token e resta a 4 colori. Testo sugli accenti: T0.

Temi alternativi (`apps/settings.rs`, `PALETTES`): restano varianti, il tema 0 è il riferimento. **Da verificare** la leggibilità di ciascuno.

## 4. Testo

- **GBA**: font di sistema 3×5, solo maiuscole (il font non ha le minuscole). Passo di riga 6 px per il testo, 10 px per le liste.
- **Mac**: lo stesso bitmap, ingrandito di un fattore intero (2× o 3×), mai interpolato. Il font vettoriale è rinviato finché non serve. **Da verificare**.
- **x86**: font Spleen 5×8 (BSD-2-Clause, `x86/kernel/LICENSE-spleen.txt`), attivato dalla feature `wide-font` di `gui`. Minuscole incluse. **Da verificare** a schermo.
- Colori: testo su sfondo T1 = T0; testo su barra = T0; testo su titolo attivo = T0; testo su icona = T1.

## 5. Spaziature

Unità logica = 1 px GBA. Tutte le misure sono in unità logiche.

| Elemento | Misura |
|---|---|
| Barra superiore | 12 px |
| Margine interno finestra | 1 px (bordo) + 4 px (contenuto) |
| Titolo finestra | 10 px |
| Pulsante chiudi | 8×8 px, a 1 px dal bordo destro |
| Icona | 16×16 px |
| Griglia icone | passo 48 × 40 px, 4 colonne, inizio a (8, 24) |
| Etichetta icona | 2 px sotto l'icona |

**Mac**: la stessa griglia moltiplicata per un fattore intero (es. 4× o 5×, secondo lo schermo). Il layout non cambia. Icone sul telefono: minimo 48 px fisici. **Da verificare**.

## 6. Finestre

- Struttura: barra del titolo, corpo, bordo di 1 px in T3.
- Titolo **attivo**: fondo T2, testo T0.
- Titolo **inattivo**: fondo T1, testo T0. Pulsanti riduci, massimizza, chiudi: quadrati 8×8 T0 (x86).
- Corpo: fondo T1. Il programma disegna solo qui dentro.
- Angoli: retti su GBA. Su x86, smussati con raggio 4 px (fondo T0). Niente ombre (decisione di design). **Da verificare** a schermo.
- **x86**: titolo attivo nel colore dell'app (A(app)), titolo inattivo T1; icona 8×8 logica a sinistra del titolo; pulsanti colorati (A(9..11)). Barra delle applicazioni: il pulsante attivo prende lo stesso colore.

## 7. Barra superiore

- Fondo T3, altezza 12 px. Testo T0 (nome a sinistra, programmi al centro).
- Orologio a destra, testo T0.
- Sempre in alto, sopra ogni finestra.

## 8. Icone e stati

Icona 16×16 = blocco 14×14 (T2) con segni a rettangoli in T0 (glifo, `gui/src/icons.rs`). Etichetta sconosciuta: riquadro vuoto.

**x86**: icona vera 32×32 px fisici (cella 16×16 logica), in `icons::art`: piastra dell'accento dell'app con angoli smussati (raggio 4 px) e segni in T0/T1. Nei titoli la stessa icona a 16×16 px. Selezione: anello T1 di 1 px logico.

| Stato | Icona | Etichetta |
|---|---|---|
| Normale | Blocco T2, segni T0 | T1 |
| Selezionata | Cella piena T1, segni T0 | T1 |
| Aperta | Come normale, con sottolineatura T2 (1 px) | T1 |
| Premuta (A) | Inversione: T0 pieno, bordo T1 | T1 |

Glifi: FILE, TERM, CALC, WEB, EDIT, SET, ORA, READ. **Da verificare** su schermo GBA reale e su x86 a 32×32 (non fatto).

## 9. Scala GBA → Mac

- Stesse 4 tinte, stessi ruoli, stessa struttura.
- Scala intera: nessuna interpolazione, nessun pixel mezzo.
- Il Mac non aggiunge elementi rispetto a GBA, salvo quelli elencati nella sezione 12 (solo schermi grandi, decisi con Ivan).

## 10. Da non fare

- Più di 4 colori su GBA.
- Testo minuscolo su GBA.
- Icone o finestre fuori dalla griglia.
- Valori di colore scritti a mano nel codice: usare i token.

## 12. Tiling e menu globale (solo schermi grandi)

Decisi con Ivan. Non si applicano al GBA, che resta a finestra piena.

- **Menu globale (stile Mac):** la barra superiore mostra a sinistra il menu del programma attivo (nome e voci). Il nome del sistema resta a sinistra della barra; il menu globale segue. Solo voci che il programma espone; nessuna voce di sistema aggiunta. Testo T0, fondo T3.
- **Tiling:** finestre disposte in colonne scorrevoli su una striscia; i programmi restano nella loro superficie. Finestre di dialogo restano fluttuanti. Nessun riquadro con bordo nuovo: solo il bordo T3 già previsto (sez. 6).
- **Tocco e tastiera:** il tiling si comanda da tastiera; il clic sulla finestra la rende attiva.
- **Regola:** dove il tiling è attivo, la massimizzazione (`toggle_max`) resta come stato a tutto schermo.
- **Da verificare:** larghezza minima di colonna, comportamento del menu globale con più finestre, uso reale su x86 e Mac.

## 11. Stato di applicazione

- Fase 1: questo documento.
- Fase 3 (`gba/src/scale.rs`): scala intera per schermi grandi, letterbox centrato con banda `T0`. Il disegno resta 240×160; test host in `gba/host/test_scale.rs`. Da verificare: schermo reale o VM grafica, barra e finestre ridimensionabili (non fatti).
- Fase 2 (`gba/src/desktop.rs`): orologio della barra in T0, come il resto del testo sulla barra. Il resto del desktop già rispetta palette, spaziature e griglia.
- Fase 4 (x86, `gui/src/shell.rs`, `render.rs`): menu HAIKUOS con ricerca (tasto Win), pulsanti finestra nella barra (riduci = icona, click sul pulsante attivo), orologio RTC in T0, trascinamento dal titolo, massimizza sotto la barra. Screenshot: `/mnt/project-files/haikuos/render-x86/`. Da verificare: ora in UTC (nessun fuso), contenuto delle app nelle finestre (ancora vuoto), temi alternativi, ombre e animazioni non fatte (voluto).
- Fase 5 (x86, `gui/src/render.rs`, `x86/kernel/src/fb.rs`): accenti a 32 bit, icone vere 32×32, titoli colorati con icona, pulsanti colorati. Mouse: PS/2 in polling, verificato con `x86/scripts/mouse-qemu.py` (clic su FILE apre la finestra; posizione calibrata a mano, **da verificare** su hardware). Font: Spleen 5×8 (BSD-2, `x86/kernel/LICENSE-spleen.txt`), **da verificare** a schermo con altri font. Da verificare: contrasto del testo T0 sugli accenti A(1..8) su schermo reale.
- Fase 1 (icone, `gui/src/icons.rs`): glifi 16×16 per GBA e x86. Da fare: versione 32×32 x86; `gba/host` screenshot. **Da verificare**.
- Da verificare su hardware GBA: leggibilità del 3×5 su schermo reale, contrasto T2 su T1, tempi di ridisegno.

## 12. Logo

- File: `docs/brand/logo.svg` (vettoriale, sfondo trasparente).
- Significato: un monolite piccolo al centro, quattro settori per i programmi, uno sempre libero (tratteggiato). Il bordo è l'hardware.
- Colori: solo i quattro token della palette (sezione 3), più il bordo SKY. Nessun marchio di terzi.
