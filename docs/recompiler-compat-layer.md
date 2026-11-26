# Layer di compatibilità Linux/Unix (documento, nessun codice)

Stato: proposta. Ipotesi **da verificare** con prototipi. Fase successiva al ricompilatore (vedi `docs/recompiler.md`).

## 1. Obiettivo

Far girare software Linux/Unix su HaikuOS senza emulare la CPU. Niente Rosetta né binary translation.

## 2. Tre strade

| Strada | Come | Pro | Contro |
|---|---|---|---|
| A. Ricompilazione (preferita) | Il ricompilatore traduce il sorgente C in Rust `no_std` e i test ne verificano il comportamento | Nativo, veloce, sicuro | Serve il sorgente. Non copre i binari chiusi |
| B. Traduzione delle syscall | Un programma utente riceve le chiamate Linux (ABI x86_64 o ARM64) e le traduce in servizi HaikuOS | Esegue binari esistenti | Serve un sottoinsieme preciso di syscall. Il monolite non deve conoscerlo |
| C. Emulazione o binary translation | Esegue codice macchina di un'altra architettura | Copre tutto | Escluso dal progetto |

## 3. Posizione nel modello a strati

- Il layer è un **programma utente**: vive sopra il microkernel per programma, come gli altri. Il monolite non conosce Linux.
- Ogni chiamata Linux passa per il filtro a regole fisse (allowlist) del monolite, come ogni altro messaggio.
- Il layer non ottiene quote oltre quelle del suo programma.

## 4. Primo passo (quando si inizia)

1. Elenco delle syscall minime per un programma semplice (es. `ls` o `cat` ricompilati). Da misurare con `strace`.
2. Prototipo per una sola architettura (ARM64 o x86_64, da decidere con Ivan).
3. Criterio di successo: un programma con output identico al Linux di riferimento, misurato con test.

## 5. Domande aperte

1. Quale architettura di partenza per le syscall: x86_64 o ARM64?
2. Serve compatibilità binaria, o basta la ricompilazione dai sorgenti?
