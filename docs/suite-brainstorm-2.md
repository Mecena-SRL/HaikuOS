# Suite creativa: ciclo 2 (target PSP 480x272)

Target minimo: PSP (480x272). Il GBA è fuori scope per la suite.

Metodo: 15 playtester sul renderer a righe sporche (commit d5e400a), poi 45 agenti Haiku (15 ingegneri, 15 artisti, 15 playtester) con 150 idee e 90 triadi votate. Pesi uguali per ruolo.

## Problemi dal playtest, e stato
- Maschera a 64 righe: con tile da 4 px su 272 px ci sono 68 righe, quindi ridisegno completo. **Scelta: tile da 8 px (34 righe).** Da fare nel renderer della suite.
- Undo ridisegnava tutto. **Fatto:** ora marca solo la riga della modifica annullata (`History::peek`).
- Mancano redo, undo di Solid e sprite, e contratto del backend (input a tre stati, present solo se sporco). **Da fare.**
- Mancano test di confine (riga 63/64, `mark_all`). **Fatto.**

## Prime idee in classifica (punteggio massimo 6)
1. Tracker a step per la musica con campioni PCM statici (musica)
2. Contratto backend PSP: input a tre stati, stride, ridisegno completo dopo il flip (piattaforma)
3. Undo/redo con cronologia a indice fisso (editor core)
4. Maschera dirty a 34 righe con tile da 8 px (motore di rendering)
5. Ciclo di eventi a tre stati con attesa di vblank (piattaforma / input)
6. Ritocco a raggio con lo stick (foto)

## Prossimo passo
Redo a indice fisso, backend PSP con contratto di input a tre stati, poi un nuovo ciclo test + brainstorm.
