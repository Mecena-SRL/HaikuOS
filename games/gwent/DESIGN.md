# Gwent × Balatro: design (v2)

Stato: bozza per la riscrittura. Nessun merge senza il sì di Ivan.

## Tre pilastri

1. **Gwent console**: due file per giocatore (vicina, a distanza), meteo globale (Gelo, Nebbia, Sereno), carte con abilità (Spia, Guaritore, Stendardo, Corno, Schieramento, Scorch).
2. **Balatro**: il punteggio non è solo somma di potenza. Ogni carta dà *chip* (valore base), ogni fila dà *moltiplicatore*; il punteggio di un round è chip × mult. Le combinazioni di unità dello stesso gruppo (es. tre lupi) danno bonus.
3. **Jolly passivi**: a inizio partita si scelgono fino a 2 jolly (modificatori permanenti, es. "Corno vale +1 mult", "il Gelo non colpisce la tua fila", "ripesca una carta quando un'unità muore"). Vengono mostrati come icone vettoriali nel pannello destro.

## Regole di base (bozza)

- Partita al meglio di 3 round. Chi perde un round apre il successivo.
- Ogni turno: gioca una carta o passa. Il round finisce quando passano entrambi.
- Punteggio round = somma dei chip × mult di ciascuna fila del giocatore, meno quello dell'avversario; vince chi ha più punti. Pareggio: vince chi non apre.
- Meteo: Gelo e Nebbia riducono solo la **potenza base delle unità** della fila (correzione del bug segnalato dal gruppo). Sereno le annulla.

## Come si lavora

- Ogni prova (test, correzione, schermata) passa dal ciclo a 45 agenti: 15 tester, 15 proposte, 15 votanti.
- Nessun merge: si apre/aggiorna la PR in draft e si aspetta il sì di Ivan.
- Schermate di riferimento a 1920x1080 e 800x480, vettoriali, generate da `gwent --shot`.

## Priorità della prima tranche

1. Correggere: meteo su sole unità, bonus IA che spreca carte, Spia (testo coerente con il motore).
2. Mostrare il tabellone di fine round (riepilogo chip × mult).
3. Introdurre il moltiplicatore di fila e i tre jolly base.
4. Tasti H/N e riga comandi visibile in finestra.
