# Suite creativa: brainstorm 1

Metodo: 15 playtester hanno testato l'editor attuale. Poi 45 agenti Haiku (15 ingegneri, 15 artisti, 15 playtester) hanno proposto 3 idee ciascuno (150 idee). Votazione a triadi: 90 triadi, ogni agente ordina 2 triadi, 3 punti al primo, 2 al secondo, 1 al terzo. Pesi uguali per ruolo (1 ciascuno).

## Problemi emersi dal playtest (sintesi)

- Ogni tasto ridisegna tutto il frame: troppo lento su ARM7.
- Il canvas ARGB u32 a 240x160 pesa circa 150 KB: metà della RAM GBA. Serve u16 in VRAM.
- L'undo non copre sprite, livello solido, `fill_rect` e redo. `Key::Solid` e `fill_rect` bypassano la history.
- Il caricamento non svuota la history, quindi l'undo agisce su una scena diversa.
- Il formato scena non ha versione né dimensioni W/H; `Scene<8,6,N>` e `Scene<6,8,N>` hanno la stessa lunghezza.
- L'edit da terminale non disegna nulla e salva solo con `q`. `new` sovrascrive senza conferma.
- Gli scacchi mancano di arrocco, en passant e patta per 50 mosse; `Moves::default()` azzera circa 1 KB a ogni `in_check`.

## Classifica (prime idee)

Punteggio massimo 6 (ogni idea compare in 2 triadi). Molti pareggi: la classifica completa è nel risultato del workflow.

| # | Idea | Area | Punti |
|---|---|---|---|
| 1 | Renderer a regioni sporche RGB565 condiviso da tutte le app | engine/gui | 6 |
| 2 | Aggiornamento a regioni sporche (bitmask fissa di celle) | OS engine | 6 |
| 3 | Sequencer a tracker su 4 canali (onde quadre e rumore, stile PSG) | musica | 6 |
| 4 | Montaggio a clip magnetiche, tagli come coppie di indici | video | 6 |
| 5 | Sequencer a griglia 16 passi, riuso della griglia tile | musica | 6 |
| 6 | Album con modifiche salvate come ricetta | foto | 6 |

## Prossimo passo proposto

Prima ondata: il renderer a regioni sporche RGB565 nel motore condiviso. È l'idea con più voti e serve a tutte e tre le app. Poi il fix dei problemi di undo e formato. Le app foto, video e musica vengono dopo.
