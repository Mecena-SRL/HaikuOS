# Compressione e cifratura della memoria

Stato: bozza di ricerca. Ogni ipotesi non verificata su hardware è marcata **da verificare**.

## 1. Sintesi

- **Cifratura:** Ascon-AEAD128 ovunque, per semplicità (decisione del 25/11/2026). È lo standard NIST per i dispositivi piccoli (NIST SP 800-232, agosto 2025).
- **Compressione:** su GBA solo offline (LZSS a finestra 256 byte, stile heatshrink). Nessuna compressione a runtime. Su macchine potenti si valuta LZ4/Zstd in una fase successiva.
- **Chiavi:** su GBA non esiste memoria sicura: la chiave è un segreto che il dispositivo può leggere. Il modello di minaccia va scritto prima di implementare.

## 2. Cifratura

| Algoritmo | Dove | Note |
|---|---|---|
| Ascon-AEAD128 | GBA, MCU, tutti i profili | Standard NIST SP 800-232. Stato 320 bit, codice piccolo, nessuna tabella grande. Chiave 128 bit, nonce 128 bit. **da verificare**: velocità reale su ARM7TDMI |
| ChaCha20-Poly1305 (RFC 8439) | ARM senza AES, telefoni vecchi | Solo ALU, niente tabelle. Poly1305 usa moltiplicazioni 32×32→64: lente su ARM7TDMI. **da verificare** |
| AES-256-GCM | ARM64/AMD64 con AES hardware | Il più veloce dove esiste AESE/AESD (ARMv8) o AES-NI. Nonce 96 bit casuali: limite pratico ~2^32 messaggi per chiave |
| XChaCha20-Poly1305 | Macchine potenti, nonce casuali | Nonce 192 bit: rischio di riuso quasi nullo |

Scelta per profilo:
- **GBA (ARM7TDMI, 16 MHz, 256 KB RAM):** Ascon-AEAD128. AES con tabelle (circa 4 KB) non entra bene in IWRAM/EWRAM senza togliere spazio agli slot da 32 KB. **da verificare** con misure.
- **MCU senza crypto extension:** Ascon-AEAD128.
- **ARM64/AMD64 telefono e PC:** AES-256-GCM se c'è accelerazione, altrimenti ChaCha20-Poly1305.

Regole comuni:
1. Ogni slot ha un tag AEAD: un blocco alterato in flash si rileva prima dell'uso.
2. Il nonce non si riusa mai con la stessa chiave. Contatore per slot + ID dispositivo, non solo casuale, dove possibile.
3. Dati associati (AAD): numero di slot, versione, ID programma. Impedisce di scambiare slot tra programmi.

## 3. Compressione

| Algoritmo | RAM decodifica | Velocità | Dove |
|---|---|---|---|
| RLE | pochi byte | altissima | dati molto ripetitivi (tile, mappe) |
| heatshrink (LZSS) | finestra scelta (es. 2^8–2^11 byte) | bassa ma decodifica semplice | GBA, MCU |
| LZ4 | nessuna finestra extra in decodifica, tabella di hash in compressione (**da verificare**: 16 KB di default) | molto veloce | ARM medio, PC |
| Zstd | finestra e tabelle più grandi | buona ratio | solo macchine potenti |

Scelta per profilo:
- **GBA:** heatshrink con finestra piccola, o RLE per asset. Compressione offline (sul PC di sviluppo), decompressione sul dispositivo. Nessuna compressione in tempo reale nella RAM dello slot da 32 KB.
- **MCU:** heatshrink o LZ4 decoder.
- **Macchine potenti:** LZ4 per la RAM, Zstd per il disco.

Ordine: **comprimere prima, cifrare dopo.** Il testo cifrato non si comprime. La lunghezza compressa può rivelare informazioni: su dati sensibili si aggiunge padding a multipli di blocco. **da verificare** se il rischio è reale per il nostro uso.

## 4. Gestione delle chiavi

Il GBA non ha un elemento sicuro. Quindi:
- Una chiave dentro la ROM è leggibile da chiunque abbia la cartuccia: serve solo a proteggere dalla copia accidentale dei dati su disco, non da un attaccante fisico.
- Una chiave derivata da una password dell'utente è più sicura, ma su GBA l'input è scomodo e la KDF costosa.

Per macchine potenti:
- Derivazione da password: Argon2id (RFC 9106) con parametri alti.
- Su GBA: KDF semplice basata su Ascon (es. Ascon-XOF) con poche iterazioni. **da verificare** sul tempo di avvio accettabile.
- Chiavi separate per livello: una chiave di slot derivata da una chiave master, mai la master usata direttamente.

## 5. Cosa è fattibile dove

| Funzione | GBA | MCU | ARM64/AMD64 |
|---|---|---|---|
| Cifratura slot 32 KB con tag | sì, Ascon (**da verificare** tempo) | sì | sì |
| Compressione asset offline | sì, heatshrink/RLE | sì | sì |
| Compressione in RAM a runtime | limitata | sì | sì |
| Chiave da password | no, KDF troppo costosa | limitata | sì, Argon2id |
| Zstd | no | no | sì |

## 6. Decisioni per Ivan (filosofia)

1. **Accettiamo che su GBA la chiave sia leggibile dal dispositivo?** Se sì, la cifratura protegge solo da copie accidentali.
2. **Priorità:** sicurezza forte su macchine potenti, o uniformità con lo stesso algoritmo su tutti i profili (Ascon ovunque)?
3. ~~Compressione: accettiamo di comprimere solo offline su GBA?~~ Deciso: sì, solo offline su GBA.

## 7. Prototipo (sim/)

- `sim/src/lzss.rs`: LZSS a finestra 256 byte. Il decodificatore usa solo finestra e output.
- `sim/src/slot.rs`: LZSS poi Ascon-AEAD128 (`AsconAead128` di `ascon-aead` 0.6), nonce 16 byte, AAD con numero di slot.
- `sim/tests/memoria.rs`: 9 test, tutti passano. Coprono round-trip, limiti di dimensione, manomissione, slot e chiave sbagliati.
- Misure host (non GBA): dati ripetitivi arrivano a circa 15% della dimensione (massimo 10 byte per 12 bit). Il rumore casuale si espande (9 bit per byte): 32 KB casuali non entrano nello slot. Da verificare su dati reali.
- Limite del formato: il nonce va gestito dal monolite (contatore per slot). Il prototipo lo riceve come parametro.

## 8. Da verificare (lista)

- Tempi reali di Ascon-AEAD128 e ChaCha20-Poly1305 su ARM7TDMI (misurare nel simulatore o su hardware).
- Dimensione codice e RAM di heatshrink e LZ4 su ARM7TDMI.
- Parametri Argon2id per macchine potenti.
- Rischio della lunghezza compressa (side channel) per i dati del nostro uso.

## 9. Fonti

- NIST SP 800-232, Ascon per la cifratura leggera: https://csrc.nist.gov/pubs/sp/800/232/final
- Annuncio NIST, agosto 2025: https://www.nist.gov/news-events/news/2025/08/nist-finalizes-lightweight-cryptography-standard-protect-small-devices
- RFC 8439, ChaCha20-Poly1305
- RFC 9106, Argon2
- heatshrink (compressione LZSS per embedded): https://spin.atomicobject.com/heatshrink-embedded-data-compression/
- Documentazione GBA (ARM7TDMI, memoria): https://rust-console.github.io/gbatek-gbaonly

Nota: le cifre di velocità e memoria per ARM7TDMI non sono ancora misurate. Sono **da verificare**.
