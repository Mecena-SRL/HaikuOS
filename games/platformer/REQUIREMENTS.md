# Platformer: requisiti per la riscrittura

Lista dai test dei revisori (versione 17f87bf). Ogni voce è un requisito della nuova versione.

## Fisica e livelli
- Salto reale circa 3,75 tile (60 px), corsa circa 5 tile. Ogni salto necessario deve stare entro questo limite con margine.
- Buche larghe 3 o più richiedono la corsa; buche larghe 4 hanno margine di almeno 4 px.
- Ogni moneta deve essere raggiungibile (L1 riga 6 col 15-17, L2 riga 5 col 20-22 e 60-62, L3 riga 5 col 56-58 sono irraggiungibili oggi).
- Nessuna piattaforma decorativa irraggiungibile, o dichiarata tale.
- Il nemico non entra nei muri. Il pilota automatico deve evitare i funghi con una finestra adeguata (almeno 32 px).
- Caduta più reattiva della salita (gravità in discesa maggiore).
- Coyote time di circa 6 frame.

## Collisioni e stati
- Crescere (power-up) non deve incastrarsi sotto un soffitto basso.
- Uno stomp non deve far subire danno da un secondo nemico nello stesso frame.
- Game Over e vittoria devono restare visibili (nel terminale: fermare il blocco di 60 frame).
- Game Over oggi azzera i livelli sbloccati: decidere se è voluto.

## Rendering
- Il costo per fotogramma è il principale problema: circa 1,1 milioni di pixel scritti per fotogramma per l'overdraw delle ellissi dello sfondo. Obiettivo: sotto 1 ms a 800x480.
- Clipping dei poligoni alla finestra di vista (il bordo viene poi tagliato da `bars`).
- Il font dell'HUD viene caricato a ogni chiamata di `vfont`: va messo in cache.
- Parallasse con profondità coerente: più lontano = più lento.
- Nessun elemento sotto la barra HUD (14 px).
- Cespugli visibili sopra il terreno.
- Gambe del personaggio in salto divaricate e in corsa più evidenti.
- Testo dei banner centrato; testo dei menu leggibile (8 px è troppo piccolo).
- Limite di MAXW 1920: non andare in panico su schermi più larghi; clamp o errore esplicito.
- Poligoni con più di 40 vertici o 32 intersezioni: troncamento silenzioso, da evitare o da segnalare.

## Test da aggiungere
- Rasterizzatore su triangolo noto (pixel interno pieno, esterno vuoto, bordo parziale).
- Raccolta moneta e moneta non raccolta due volte.
- Sblocco mappa e cursore che non supera i livelli sbloccati.
