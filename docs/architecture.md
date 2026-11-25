# Architettura di HaikuOS

> Documento di partenza. Tutto ciò che è segnato **"da verificare"** è un'ipotesi: non è ancora stata misurata né provata.

## 1. Visione

HaikuOS è un sistema operativo modulare, leggero e sicuro.

- **Obiettivo:** girare su hardware molto diverso tra loro, dalla Game Boy Advance (circa 288 KB di RAM) fino a ARM64 e AMD64.
- **Hardware di partenza:** la Game Boy Advance, come hardware primordiale da cui partire.
- **Primo uso previsto:** documenti e giochi.
- **Idea guida:** il sistema è fatto di parti piccole con confini chiari. Ogni parte fa poche cose e le fa bene.
- **Priorità di progetto:** sicurezza, semplicità, velocità, in quest'ordine in caso di conflitto.
- **Principio:** ridurre al minimo il codice che ha il potere di decidere su tutto (il "monolite", vedi sotto).

## 1bis. Linguaggi

- **Rust `no_std`** per kernel, agenti e servizi.
- **Assembly** solo per l'avvio.
- **C** come alternativa, se Rust risulta troppo grande per la Game Boy Advance. **Da verificare** con una misura reale.

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
L'agente **propone** le quote; il monolite **approva** e le fa rispettare entro i limiti fissi.

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
2. **Filtro dei messaggi a regole fisse (allowlist).** Un messaggio passa solo se una regola esplicita lo permette. Niente euristiche o "intuizioni". Regole iniziali:
   - allowlist di coppie mittente-destinatario;
   - dimensione massima fissa per ogni messaggio;
   - id del mittente verificato dal monolite, non dichiarato dal messaggio;
   - nessun messaggio può concedere diritti da solo.
   (**Da verificare:** che la lista basti per il primo rilascio.)
3. **Isolamento tra programmi.** Ogni programma gira nel proprio microkernel.
4. **Revoca delle quote.** Il monolite può togliere una quota in qualsiasi momento.

## 5. Priorità e risorse

- **Reattività garantita:** le risorse necessarie per rispondere in tempo devono essere sempre disponibili (da verificare: quali sono i tempi di risposta).
- **Riserva di sistema:** una parte della memoria resta sempre libera per il sistema.
- **Admission control:** il sistema non accetta un nuovo programma se non c'è abbastanza memoria per tutti. Non promette più di quanto ha.

## 6. Limiti di partenza (bozza)

Su Game Boy Advance, con circa 288 KB di RAM (da verificare: la quantità esatta disponibile dopo il sistema):

| Voce | Valore di partenza |
|---|---|
| Monolite | 64 KB |
| Agente di allocazione | 16 KB |
| Slot per programma | 32 KB ciascuno |
| Programmi contemporanei | massimo 4 |
| Slot libero | sempre uno disponibile |

Somma indicativa: 64 + 16 + 4 × 32 = 208 KB, quindi resta circa 80 KB per il resto. **Da verificare** se tutto questo entra davvero, e quanta memoria serve al microkernel hardware.

## 7. Obiettivi tecnici

- Ridurre il più possibile il monolite: ogni funzione che può stare altrove, va altrove.
- Mantenere ogni strato piccolo e testabile da solo.
- Non scrivere codice o bootloader in questa fase: si parte dalla documentazione.

## 8. Decisioni prese (da Ivan)

1. **Linguaggi:** Rust `no_std` per kernel, agenti e servizi; assembly solo per l'avvio; C come alternativa (vedi 1bis).
2. **Quote:** l'agente propone, il monolite approva e fa rispettare i limiti fissi.
3. **Priorità:** sicurezza, semplicità, velocità.
4. **Primo uso:** documenti e giochi.
5. **Limiti:** la sezione 6 è il punto di partenza; il Game Boy Advance è l'hardware primordiale.
6. **Primo filtro messaggi:** allowlist di coppie mittente-destinatario, dimensione massima fissa, id mittente verificato dal monolite, nessun messaggio concede diritti da solo.

## 9. Domande ancora aperte

- Rust `no_std` entra nel Game Boy Advance? **Da verificare** con una misura reale; se no, si passa a C.
- Il limite di 4 programmi e lo slot libero reggono con la RAM reale? **Da verificare.**
