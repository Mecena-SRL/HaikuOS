# Opzioni desktop di HaikuOS

> Studio dello stato dell'arte (fine 2026) per: filosofia, window manager, tiling, dock, barra in alto, app a schermo intero.
> Fonti: ricerca web su panoramiche 2026 (es. [Arch Wiki – confronto tiling WM](https://wiki.archlinux.org/title/Comparison_of_tiling_window_managers), [Hyprland vs Sway vs COSMIC 2026](https://botmonster.com/self-hosting/hyprland-vs-sway-vs-cosmic-wayland-compositors/), [COSMIC 1.0](https://botmonster.com/self-hosting/cosmic-desktop-1-0-daily-driving-system76-rust-de/), [A Mac-like experience on Linux](https://pointieststick.com/2025/10/04/a-mac-like-experience-on-linux/)).
> Le licenze sono indicative: **da verificare** prima di usare codice altrui.

## 0. Il punto chiave

Tutte le opzioni qui sotto girano sopra Linux (kernel, driver, Wayland/X11). HaikuOS ha un suo kernel in Rust no_std: nessuna di queste è usabile "così com'è".

Quindi ogni scelta si divide in due:
- **A. Riusare lo stack Linux/Wayland** come livello di compatibilità: veloce da avere, ma porta dentro molta complessità.
- **B. Protocollo proprio**: un compositor piccolo che parla con il nostro kernel, con bar/dock/app come processi separati.

Le scelte sotto sono pensate per B, prendendo idee (non codice) dalle opzioni esistenti.

## 0b. Stato nel repo (allineato a `docs/graphic-bible.md`, `docs/desktop-concept.md`, `gui/`)

- **Già esiste:** barra superiore 12 px in T3 con orologio a destra; finestre fluttuanti con titolo, chiudi, riduci, massimizza (`gui/src/wm.rs`); menu HAIKUOS con ricerca (`gui/src/shell.rs`); pannello con pulsanti per finestra; compositor che decide cosa si vede (i programmi disegnano solo nella loro superficie).
- **Regole vincolanti da rispettare:** 4 colori (T0..T3), niente ombre o animazioni decorative, "il Mac non aggiunge elementi: se un elemento c'è su GBA, c'è anche su Mac".
- **Non esiste:** tiling, dock separato, menu globale macOS.
- **Conseguenza:** le raccomandazioni sotto valgono solo se accettate come nuove funzioni. Ogni nuovo elemento (dock, menu globale, tiling) va deciso con Ivan e poi aggiunto alla bibbia grafica.

---

## 1. Filosofia del desktop

| Opzione | Idea | Pro | Contro | Uso reale |
|---|---|---|---|---|
| **macOS-like** | Un menu globale, dock, finestre fluttuanti | Familiare, coerente | Più regole da mantenere | GNOME/KDE con estensioni, Plank |
| **Tiling a mano** | Le finestre si dispongono da sole, da tastiera | Molto veloce, leggero | Curva di apprendimento | i3, Sway, Hyprland |
| **Tiling scorrevole** | Colonne su una striscia infinita, niente griglia fissa | Semplice da capire, ottimo con schermi piccoli | Meno diffuso | niri (in forte crescita nel 2026) |
| **Minimale / suckless** | Poche funzioni, tutto configurato a codice | Piccolissimo, prevedibile | Va costruito quasi tutto | dwm, awesome |

**Raccomandazione:** tiling scorrevole (stile niri) come base, con uno strato "macOS" sopra (menu globale, dock, barra). È la combinazione più leggera e più familiare.

## 2. Window manager / compositor

| Opzione | Linguaggio | Licenza (indicativa) | Pro | Contro |
|---|---|---|---|---|
| Hyprland | C++ | BSD-3 | Bello, molte funzioni, animazioni | Complesso, molte dipendenze |
| Sway | C | MIT | Stabile, compatibile i3 | Solo tiling classico |
| niri | Rust | GPL-3 | Tiling scorrevole, semplice, Rust | Giovane, meno funzioni |
| COSMIC | Rust | GPL-3 | Desktop completo, 1.0 nel 2025 | Grande: troppo per noi |
| i3 / dwm | C | BSD / MIT | Minimo, maturo | Solo X11 |

**Raccomandazione:** nessun compositor esterno da riusare (sono tutti Wayland). Prendere solo idee: **niri** come modello di layout (Rust, scorrevole, piccolo). Il nostro `gui/src/wm.rs` resta la base: finestre fluttuanti già presenti.

## 3. Tiling

| Modo | Pro | Contro |
|---|---|---|
| Griglia automatica (i3/Sway) | Prevedibile | Rigida su schermi piccoli |
| Colonne scorrevoli (niri) | Adatta a qualsiasi schermo | Si perde la vista d'insieme |
| Tiling manuale + fluttuanti | Flessibile | Più stati da gestire |

**Raccomandazione:** nessun tiling per ora. Il WM attuale è a finestre fluttuanti: è già coerente con la bibbia grafica. Il tiling scorrevole è una funzione nuova da proporre dopo la base. Sul GBA non serve: una sola app a tutto schermo (vedi `docs/desktop-concept.md`, sez. 5).

## 4. Dock (stile macOS)

| Opzione | Pro | Contro |
|---|---|---|
| Plank | Semplice, leggero | Poco mantenuto |
| nwg-dock (Hyprland) | Leggero, Wayland | Legato a Hyprland |
| Dash to Dock (GNOME) | Maturo | Legato a GNOME |
| Dock nostro (processo separato) | Su misura, piccolo | Da scrivere |

**Raccomandazione:** nessun dock nuovo. La barra dei programmi del pannello (`gui/src/shell.rs`) fa già il suo lavoro. Un dock separato costerebbe un elemento in più, contro la regola "il Mac non aggiunge elementi". Se Ivan lo vuole, va deciso come eccezione.

## 5. Barra in alto (stile macOS)

| Opzione | Pro | Contro |
|---|---|---|
| Waybar (MIT) | Configurabile, diffusa | Solo con compositor Wayland compatibili |
| Quickshell / Astal (LGPL/GPL) | Barre fatte in QML o TypeScript, belle | Dipendenze da gestire |
| Barra nostra | Controllo totale | Da scrivere |

**Raccomandazione:** tenere la barra attuale (12 px, T3, orologio a destra, come da `docs/graphic-bible.md` sez. 7). Il menu globale della app attiva è un elemento nuovo: da decidere con Ivan.

## 6. App a schermo intero

- Un'app può chiedere lo schermo intero; la barra e il dock si nascondono.
- Con i "spazi" (workspace) scorrevoli, ogni app a tutto schermo occupa uno spazio suo (come macOS Spaces).
- Lo schermo intero deve essere un **permesso** che il monolite concede e può revocare (coerente con la sez. 4 di `docs/architecture.md`).

**Raccomandazione:** usare la massimizzazione già presente (`toggle_max` in `gui/src/wm.rs`). Lo schermo intero vero (la barra si nasconde) è una funzione nuova: come stato del compositor, con permesso del monolite. Sul GBA è già l'unico modo.

---

## 7. Domande aperte per Ivan

1. **Base dello stack grafico:** A (riusare Wayland/Linux come compatibilità) oppure B (protocollo nostro, già in uso: i programmi disegnano nella loro superficie)?
   - **B, protocollo nostro** – coerente con il repo e con la filosofia. *(consigliato)*
   - **A, compatibilità Wayland** – si ha subito un desktop, ma si eredita la complessità di Linux.
2. **Nuovi elementi (menu globale, dock, tiling, schermo intero):** aggiungerli, o restare con la barra e il WM attuali?
   - **Restare con quelli attuali, aggiungere solo lo schermo intero** – rispetta la regola "pochi elementi". *(consigliato)*
   - **Aggiungere menu globale e tiling** – più innovativo, ma va cambiata la bibbia grafica.
3. **Primo schermo da supportare:** solo grande (PC/ARM64) o anche GBA fin da subito?
   - **Prima grande, poi GBA** – si valida la logica dove c'è spazio. *(consigliato)*

## 7b. Decisioni prese (da Ivan)

- Stack grafico: **protocollo nostro**.
- Tiling e menu globale stile Mac: **sì**, solo schermi grandi. Dettagli in `docs/graphic-bible.md` sez. 12.

## 8. Prossimi passi (proposta)

1. Scegliere le domande sopra.
2. Scrivere `gui/` con una prima bozza: compositor scorrevole, dock, barra.
3. Test su schermo 640×480 (render già disponibili nel progetto) prima del GBA.
