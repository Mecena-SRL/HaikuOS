# Architettura di HaikuOS

> Documento di partenza. Tutto ciò che è segnato **"da verificare"** è un'ipotesi: non è ancora stata misurata né provata.

## 1. Visione

HaikuOS è un sistema operativo modulare, leggero e sicuro.

- **Obiettivo:** girare su hardware molto diverso tra loro, dalla Game Boy Advance (circa 288 KB di RAM) fino a ARM64 e AMD64.
- **Primo uso previsto:** un desktop generico.
- **Idea guida:** il sistema è fatto di parti piccole con confini chiari. Ogni parte fa poche cose e le fa bene.
- **Principio:** ridurre al minimo il codice che ha il potere di decidere su tutto (il "monolite", vedi sotto).

## 2. Gli strati

Dal basso verso l'alto:

```
Programma                      (si comporta come un unikernel)
Microkernel per programma      (uno per ogni programma)
Agente di allocazione          (uno solo, all'inizio)
Monolite orizzontale           (controllore piccolo)
Microkernel hardware           (verticali: CPU, memoria, GPU, rete)
Hardware
```

### 2.1 Hardware
Il chip reale: CPU, memoria, schermo, rete. HaikuOS non lo cambia.

### 2.2 Microkernel hardware (verticali)
Un microkernel per settore: uno per la CPU, uno per la memoria, uno per la GPU, uno per la rete.
Ogni verticale parla solo con l'hardware del proprio settore.

### 2.3 Monolite orizzontale (controllore piccolo)
È il punto unico che attraversa tutti i settori. Fa solo quattro cose:

1. conta la memoria e i limiti di ogni agente;
2. verifica i permessi;
3. smista i messaggi tra microkernel;
4. concede o revoca le quote.

Il monolite **non** contiene politiche di priorità, non decide cosa è più importante e non conosce il contenuto dei programmi.

### 2.4 Agente di allocazione
Gestisce le richieste dei programmi (quanta memoria, quale risorsa) e la comunicazione sicura, sempre entro limiti fissi.
All'inizio ce n'è uno solo. Non può superare i limiti che il monolite impone.

### 2.5 Microkernel per programma
Ogni programma ha il suo microkernel minimo, che isola il programma dagli altri.

### 2.6 Programma
Un programma che si comporta come un unikernel: contiene solo ciò che gli serve per girare.

## 3. Compiti e confini

| Strato | Fa | Non fa |
|---|---|---|
| Microkernel hardware | Accesso sicuro all'hardware del proprio settore | Decisioni su altri settori |
| Monolite | Conteggio, permessi, smistamento, quote | Politiche di priorità, logica dei programmi |
| Agente | Gestisce richieste entro limiti fissi | Superare le quote, bypassare il monolite |
| Microkernel per programma | Isola il programma | Vedere la memoria degli altri programmi |
| Programma | Svolge il suo lavoro | Chiedere più di quanto il monolite concede |

## 4. Regole di sicurezza

1. **Non fidarsi ciecamente degli agenti.** Il monolite fa rispettare i limiti sempre, anche se un agente sbaglia o è compromesso.
2. **Filtro dei messaggi a regole fisse (allowlist).** Un messaggio passa solo se una regola esplicita lo permette. Niente euristiche o "intuizioni" (da verificare: la lista delle regole iniziali).
3. **Isolamento tra programmi.** Ogni programma gira nel proprio microkernel.
4. **Revoca delle quote.** Il monolite può togliere una quota in qualsiasi momento.

## 5. Priorità e risorse

- **Reattività garantita:** le risorse necessarie per rispondere in tempo devono essere sempre disponibili (da verificare: quali sono i tempi di risposta).
- **Riserva di sistema:** una parte della memoria resta sempre libera per il sistema.
- **Admission control:** il sistema non accetta un nuovo programma se non c'è abbastanza memoria per tutti. Non promette più di quanto ha.

## 6. Limiti di partenza (bozza)

Su Game Boy Advance, con 288 KB di RAM utili (EWRAM 256 KB + IWRAM 32 KB, vedi `docs/feasibility-gba.md`):

| Voce | Valore di partenza |
|---|---|
| Monolite | 64 KB, in EWRAM (IWRAM è 32 KB, troppo piccola) |
| Agente di allocazione | 16 KB |
| Slot per programma | 32 KB ciascuno |
| Programmi registrati | massimo 4 |
| Quote concedibili | massimo 3: uno slot è sempre libero |

Somma indicativa: 64 + 16 + 3 × 32 = 176 KB, quindi restano circa 112 KB per il resto. Il valore 208 KB (4 slot pieni) non è ammesso dalla regola dello slot libero.

**Da verificare** quanta memoria serve davvero al microkernel hardware e al sistema, dopo i 176 KB.

## 7. Obiettivi tecnici

- Ridurre il più possibile il monolite: ogni funzione che può stare altrove, va altrove.
- Mantenere ogni strato piccolo e testabile da solo.
- Non scrivere codice o bootloader in questa fase: si parte dalla documentazione.

## 8. Domande aperte

Da decidere con Ivan:

1. **Linguaggi di programmazione:** quali usare per ogni strato (kernel, agenti, programmi)?
2. **Chi decide le quote:** il monolite le assegna in modo fisso, oppure l'agente le propone?
3. **Filosofia:** quali sono i valori che hanno la precedenza in caso di conflitto (sicurezza, velocità, semplicità)?
4. **Priorità:** quale uso è il primo da supportare dopo il desktop generico?
5. **Limiti:** i valori della sezione 6 vanno bene come punto di partenza, o cambiano?
6. **Filtro messaggi:** quali regole fisse sono necessarie al primo rilascio?

## 9. Decisione: nucleo e piattaforme

Valida per GBA e per una macchina potente.

- **Nucleo:** monolite logico, agente, file system e messaggi sono Rust `no_std` puro, senza dipendenze dall'hardware.
- **Piattaforma:** ogni piattaforma fornisce solo un livello sottile: avvio, timer, memoria fisica, schermo o console. Si scrive in assembly per GBA, in assembly o Rust per x86-64 e ARM64.
- **Parametri, non costanti:** blocchi, slot e limiti sono parametri di configurazione. GBA usa i valori piccoli, la macchina potente quelli grandi.
- **File system senza allocazione dinamica:** solo blocchi fissi nel nucleo, così il codice è identico su entrambe le piattaforme.
- **Linguaggi:** assembly solo per l'hardware, Rust ovunque il resto. C resta piano B.
- **Test:** solo in emulatore per ora. Il logo Nintendo e il checksum della ROM non sono inclusi.

## 10. Misure sull'emulatore (GBA, mGBA)

Cicli del timer a 16,78 MHz, media su più esecuzioni. Non sono misure su hardware.

| Operazione | Cicli | Tempo circa |
|---|---|---|
| alloc 4 KB + free (64 blocchi) | 35 414 | 2,1 ms |
| sync di un file da 100 byte | 165 122 | 9,8 ms |

Il sync azzera un buffer da 8 KB a ogni chiamata: è probabilmente la parte più costosa. Da correggere prima di aumentare la frequenza di salvataggio.

## 11. Obiettivi non ancora realizzati

Questi sono obiettivi, non funzionalità esistenti.

- **Multicore:** esecuzione su più core, con scheduling e sincronizzazione.
- **Multithreading:** più thread dentro lo stesso programma.
- **GPU:** accelerazione grafica sulle piattaforme che la hanno.
- **Compressione e cifratura in RAM:** dati compressi e cifrati in memoria, per i sistemi con poca RAM.
- **Swap:** spostamento di pagine su un supporto più lento quando la RAM non basta.
- **Portabilità a 64 bit e x86:** il nucleo è pensato per essere portabile, ma oggi gira solo sul GBA.

## 12. Compatibilità CPU e GPU

Stato su x86-64 (ISO, verificato in QEMU):

- **CPU:** il loader controlla CPUID (long mode e SSE2). Se mancano, stampa `CPU NON SUPPORTATA` su seriale COM1 e VGA, poi si ferma. Nessun avvio parziale.
- **Schermo:** il framebuffer GRUB (Multiboot2, richiesta opzionale) supporta 15, 16, 24 e 32 bit. I canali (posizione e dimensione) arrivano dal firmware e la conversione è generica. Il percorso veloce è solo per 32 bit standard.
- **Fallback:** framebuffer non valido o modo non RGB → resta il testo VGA. Nessun modo grafico non verificato.
- **Verificato:** 800x600x24 e 1280x800x32 (QEMU). 16 bit e GPU reali: **da verificare**.

Stato su ARM64 (`arm64/`, QEMU virt, verificato):

- **Avvio:** kernel a 0x40080000 in EL1, stack e BSS, seriale PL011 (0x09000000). Fuori da EL1 stampa un errore e si ferma.
- **Non ancora:** GIC, timer, MMU, framebuffer (virt non ha GOP; da fare con UEFI), boot UEFI su hardware reale. **Da verificare.**

Regola: ogni architettura dichiara ciò che supporta. Ciò che manca si rifiuta in modo sicuro, senza avvio parziale.
