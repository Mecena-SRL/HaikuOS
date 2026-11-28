# Città-impero: regole

Gioco a turni, vince chi arriva per primo a 50 cittadini. Scelte di design (valori decisi, da rivedere con i test):

- **Mappa**: 24×12, generata dal seme (splitmix64). Erba, foresta, montagna, acqua.
- **Equità**: giocatore e IA hanno le stesse regole, incluso un solo edificio per turno (`BuildError::TurnLimit`).
- **Crescita**: +1 cittadino a turno, +1 ogni tre fattorie (massimo +3), solo con cibo ≥ 2 e casa libera.
- **Pareggio**: se entrambi arrivano a 50 nello stesso turno, la partita è patta (`Status::Draw`).
- **Tetto di turni**: 80. Allo scadere vince chi ha più cittadini, parità = patta.
- **Traguardi** (una volta per impero): 10 cittadini +10 legno; 25 +5 pietra; 40 +5 oro.
- **Edifici**: casa su erba (10 legno); fattoria su erba (5 legno); legnaia su foresta (5 legno); cava su montagna (8 legno); mercato (15 legno, 5 pietra, +3 oro/turno); biblioteca (10 legno, 10 pietra, 5 oro, +2 conoscenza/turno).
- **Ricerca**: `r` sblocca l'irrigazione con 20 conoscenza (+2 cibo per fattoria).
- **Cantiere**: un edificio completa dopo due fine turno (il secondo turno è tutto del cantiere; compare come `_`).
- **Costi crescenti**: ogni due edifici dello stesso tipo, il successivo costa un legno in più.
- **Demolizione** (`x`): rimuove l'edificio sul cursore e rimborsa metà del legno base. Conta come azione del turno.
- **Eventi**: ogni 10 turni, uguali per entrambi: raccolto abbondante (+12 cibo), siccità (-8 cibo), epidemia (-1 cittadino), festa (+4 oro), tempesta (-6 legno).
- **Personalità IA** (dal seme): agricola (fattorie per prime), mercante (mercato e biblioteca per primi), costruttore (case, legnaie, cave per prime). Cambia solo l'ordine.

## Comandi
`w a s d` muovono il cursore (la tessera evidenziata); `b <edificio>` avvia un cantiere sul cursore;
`x` demolisce; `t` fine turno; `r` ricerca; `p` pausa; `q` esci. Con `b <edificio> <x> <y>` si può costruire altrove.
