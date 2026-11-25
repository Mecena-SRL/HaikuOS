# HaikuOS su GBA: prima ROM

Prima ROM: monolite, agente di allocazione e slot. Mostra lo stato a schermo. Nessun bootloader separato.

## Build

Serve Rust nightly con `rust-src` (già in `rust-toolchain.toml`). Target `thumbv4t-none-eabi`, con `build-std`.

```sh
cd gba
cargo build --release
/usr/bin/llvm-objcopy -O binary target/thumbv4t-none-eabi/release/haiku-gba haiku-gba.gba
```

## Cosa mostra la ROM

- Barra in alto: memoria del pool (verde monolite, blu agente, arancio slot assegnati, grigio scuro libero).
- Quattro quadrati: arancio = slot assegnato, grigio = libero. Sequenza di boot: 3 concessi, il 4° negato, revoca, riassegnazione.
- Quattro quadretti in basso: monolite (verde), agente (blu), persistenza (verde = ok), allocatore (verde = ok).
- Riga blu in basso: un quadretto per ogni avvio salvato (il contatore sopravvive al riavvio).

## File system (`src/fs.rs`)

- Store a 128 blocchi da 64 byte (8 KB). Nomi e contenuti separati. Max 16 file, nomi da 12 byte.
- Checksum FNV-1a su ogni blocco, verificato in lettura.
- Scrittura atomica: blocchi nuovi prima, cambio di testa poi, liberazione del vecchio. Se manca spazio, il file resta uguale.
- Permessi: solo l'owner. Quota: la scrittura non supera i byte concessi dall'agente.
- Test host: `cd gba && rustc --edition 2021 --test src/fs.rs -o /tmp/fs_test && /tmp/fs_test` (5 test, tutti ok).
- Directory: percorsi "a/b" (max 24 byte), `mkdir`, `list`. Un file richiede la directory padre. Una directory non vuota non si cancella.

## Verificato

- Compila con nightly (`thumbv4t-none-eabi`, `build-std`). Binario ~1,1 KB.
- Header ROM corretto (branch d'ingresso, titolo, byte fisso 0x96). Entry a 0x08000000.
- Avvio in mGBA 0.10.2 (software renderer, Xvfb): la schermata si vede come descritta.

## Non fatto (vecchio elenco)

- **Hardware reale:** il logo Nintendo (0x04-0x9F) e il checksum (0xBD) non sono impostati. mGBA li ignora; il BIOS reale no.
- Logica del monolite: porting manuale da `sim/src/lib.rs`. Nessun test sul target: i test di `sim/` coprono la versione host.
- Dimensione reale del codice e uso reale di RAM: non misurati oltre al binario.
- Messaggi (filtro allowlist) non portati: fuori dal primo passo.
- Allocatore dentro gli slot: non ancora.
- Quanta EWRAM resta davvero libera: da verificare.

## Persistenza (`src/persist.rs`)

- Supporto: il livello sottile `Medium`. Sul GBA è la SRAM della cartuccia da 32 KB (`src/sram.rs`, tag `SRAM_V113` per mGBA). Sulla macchina potente sarà un file.
- Journal con checksum FNV-1a su ogni record. Due regioni di log alternate e due slot di intestazione. Quando il log è pieno, lo stato si riscrive nella regione libera (compattazione), poi cambia la testa.
- Un byte 0 dopo la coda del log: il replay si ferma lì e non legge record vecchi.
- Salvataggio incrementale: solo i file cambiati dall'ultimo salvataggio, con più modifiche dello stesso file coalescenti in una scrittura.
- Regole di salvataggio (profilo del supporto): salva a soglia di modifiche (`units`) o dopo `max_frames`, solo se c'è almeno una modifica. Limite di `max_writes_per_min` salvataggi per minuto.
- Profilo GBA: soglia 64, tempo 1800 frame (30 s), 2 salvataggi al minuto. Profilo macchina potente: da definire.
- Test host: 17 test, inclusi limiti di salvataggi e assenza di salvataggi senza modifiche. `cd gba && rustc --edition 2021 --test host/test_root.rs -o /tmp/gba_t && /tmp/gba_t`.
- Verificato in mGBA: il contatore di avvii sale da 1 a 2 tra due esecuzioni (il `.sav` contiene i dati).

## Da verificare

- Cicli di scrittura della SRAM e della flash GBA: non misurati. Il profilo sceglie frequenza bassa per questo motivo.
- Su hardware reale: non testato. SRAM e tempi di scrittura sono da misurare.
- Stack: il file system e il persistente stanno nell'IWRAM da 32 KB. Non misurato oltre a quello che mGBA mostra.

## Directory e allocatore

- Directory nel file system: `mkdir`, percorsi con `/`, `list`. Persistite con il record `MKDIR`, prima delle loro voci.
- Allocatore per slot (`src/alloc.rs`): bitmap di 512 blocchi da 64 byte (32 KB). `Monolith::alloc` e `free` per programma, entro la quota concessa. Oltre la quota: `OverQuota`.
- Profilo macchina potente (`persist::POWERFUL`): soglia 4 modifiche o 6 frame (0,1 s), nessun limite di salvataggi. Scelto senza misure: da verificare.
- Test host: 23 test. Comando invariato.

## Da verificare (aggiunte)

- Profilo macchina potente: valori scelti a tavolino, non misurati.
- Allocatore: costo in cicli su ARM7TDMI non misurato.

## Spazio di lavoro e misure

- Keypad: su/giù scelgono, A apre (directory o file), B torna alla directory superiore, Start crea un file `f<n>` nella directory corrente. Logica in `src/workspace.rs`, con 4 test host.
- Schermo: barra in alto = directory corrente (lunghezza del percorso). Quadretti blu = directory, arancio = file, bordo bianco = selezione. Barra verde in basso = lunghezza del file aperto.
- Schermata misure (premi A per continuare): due righe di 32 bit, alto a sinistra = bit più significativo. Sono cicli del timer (16,78 MHz) misurati in mGBA.

### Misure (mGBA, media)

| Operazione | Cicli | Tempo a 16,78 MHz |
|---|---|---|
| alloc 4 KB + free (64 blocchi) | 35 414 | circa 2,1 ms |
| sync di un file da 100 byte | 165 122 | circa 9,8 ms |

Note sulle misure:
- Sono cicli di emulazione. Le attese della SRAM e della ROM sono modellate da mGBA, non dal hardware reale.
- Il sync include, a ogni chiamata, l'azzeramento di un buffer da 8 KB: è probabilmente la parte più costosa. Da ottimizzare prima di aumentare la frequenza di salvataggio.
- La media sul sync include le compattazioni occasionali.

## Da verificare (aggiunte)

- Costo reale su hardware: non misurato.
- Il buffer da 8 KB nel sync: da spostare fuori dal percorso caldo.

## Due spazi (`src/space.rs`)

- Sistema e utente sono due spazi separati nello stesso supporto, ognuno con la sua finestra (`Window`), la sua regione di log, le sue intestazioni e i suoi checksum. Sul GBA: sistema a 0 (16 KB), utente da 16 KB (32 KB di SRAM in totale).
- Formattare lo spazio sistema (`Spaces::format_system`) azzera solo le sue intestazioni e lo riapre vuoto. Lo spazio utente non viene toccato: test host.
- Permessi per spazio: solo la capacità di sistema (`SYS_CAP`, id 0) può formattare il sistema. Nello spazio utente vale il proprietario di ogni file. Niente permessi in stile Unix.
- Il contatore di avvii sta nel sistema; i file e il workspace stanno nell'utente.
- Limite: un file deve stare nella regione del suo spazio (circa 8 KB per regione). Un file di 8 KB pieno non rientra in compattazione: da correggere.
- Test host: 31 test. ROM con demo del compositore (PR #24) ancora presente: passa con A.

## Da fare sul GBA

- Verificare su hardware reale SRAM, tempi di scrittura e stack.
- Ridurre il costo di sync (buffer da 8 KB azzerato a ogni chiamata).
- Formattazione del sistema: non collegata a un tasto né a un comando nel workspace.
- Limite del file pieno (vedi sopra).
- Profilo macchina potente: valori da verificare.
