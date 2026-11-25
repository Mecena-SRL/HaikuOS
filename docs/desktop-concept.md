# Concept del desktop di HaikuOS

> Primo concept visivo. Ogni misura non provata è segnata **"da verificare"**.
> Legato a [graphics.md](graphics.md): il desktop non conosce i pixel dei programmi, solo le loro superfici.

## 1. Idea

Un desktop semplice e leggibile, che parte dal GBA (240×160) e cresce fino a telefono e computer. Pochi elementi, sempre gli stessi: barra, finestre, icone.

## 2. Schermata principale (GBA)

- **Sfondo** pieno, un colore solo (niente texture: costa memoria).
- **Barra in alto**, alta 12 px (da verificare): orologio del timer, memoria libera, nome della finestra attiva.
- **Icone** in colonna a sinistra, 16×16 px (da verificare), con etichetta di 12 caratteri al massimo.
- Nessun cursore libero: il focus si muove con il keypad.

## 3. Finestre

- Una finestra = una superficie del compositore (vedi [graphics.md](graphics.md) §4).
- Barra del titolo alta 12 px, corpo 120×80 px come misura iniziale (da verificare).
- Una sola finestra piena alla volta sul GBA; sovrapposizione solo sugli schermi più grandi.
- Quota video per finestra: 32 KB, da verificare in modo 4.

## 4. Programmi di base

- **File manager.** Elenco di file e cartelle (max 16, come nel file system attuale). Vista a lista, non a griglia.
- **Terminale.** Una riga di testo con comandi a stile Linux (`ls`, `cd`, `cat`, `mkdir`). Stessa struttura di directory del file system GBA. Il parser resta minimo.
- **Impostazioni.** Solo tre voci: luminosità, suono, avvio. Il resto più avanti.

## 5. Navigazione con il keypad GBA

| Tasto | Azione |
|---|---|
| D-pad | Sposta il focus tra icone e voci |
| A | Apri icona o voce |
| B | Chiudi finestra / torna indietro |
| Start | Apri il menu (barra) |
| Select | Passa alla finestra successiva |

## 6. Schermi più grandi (computer piccolo)

- La barra diventa un dock in basso, con più finestre aperte.
- Il mouse sostituisce il focus: clic per aprire, trascinamento per spostare (da verificare).
- Risoluzione e colori da decidere con il driver schermo; il layout a strati resta uguale.

## 7. Telefono (touch)

- Tocco per aprire; scorrimento dal bordo superiore per la barra.
- Finestre a schermo intero; una alla volta, con passaggio tra app con gesto laterale.
- Pulsanti di almeno 44 px (da verificare: misura comune su touch, non provata qui).
- Il terminale usa la tastiera a schermo.

## 8. Cosa è già provato

- Compositore con superfici, ritaglio e trasparenza a chiave: test host in `gba/src/compositor.rs`.
- Schermata di prova `desktop_sketch`: sfondo, barra, finestra, icona. Verificata sui byte del framebuffer (test host), non sullo schermo GBA.

## 9. Domande aperte

1. Il desktop deve sembrare una scrivania classica (barra e finestre) o una console con icone a schermo intero? Decide l'estetica del primo passo.
