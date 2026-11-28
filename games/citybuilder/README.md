# Città-impero: regole

Gioco a turni, vince chi arriva per primo a 50 cittadini. Scelte di design (valori decisi, da rivedere con i test):

- **Mappa**: 24×12, generata dal seme (splitmix64). Erba, foresta, montagna, acqua.
- **Equità**: giocatore e IA hanno le stesse regole, incluso un solo edificio per turno (`BuildError::TurnLimit`).
- **Crescita**: +1 cittadino a turno, +1 ogni tre fattorie (massimo +3), solo con cibo ≥ 2 e casa libera.
- **Pareggio**: se entrambi arrivano a 50 nello stesso turno, la partita è patta (`Status::Draw`).
- **Edifici**: casa su erba (10 legno); fattoria su erba (5 legno); legnaia su foresta (5 legno); cava su montagna (8 legno); mercato (15 legno, 5 pietra, +3 oro/turno); biblioteca (10 legno, 10 pietra, 5 oro, +2 conoscenza/turno).
- **Ricerca**: `r` sblocca l'irrigazione con 20 conoscenza (+2 cibo per fattoria).

## Comandi
`w a s d` muovono il cursore (la tessera evidenziata); `b <edificio>` costruisce sul cursore;
`t` fine turno; `r` ricerca; `p` pausa; `q` esci. Con `b <edificio> <x> <y>` si può costruire altrove.
