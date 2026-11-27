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

**Raccomandazione:** **niri** come modello (Rust, scorrevole, piccolo). Non riusarlo tal quale: è un compositor Wayland. Ne prendiamo la logica di layout.

## 3. Tiling

| Modo | Pro | Contro |
|---|---|---|
| Griglia automatica (i3/Sway) | Prevedibile | Rigida su schermi piccoli |
| Colonne scorrevoli (niri) | Adatta a qualsiasi schermo | Si perde la vista d'insieme |
| Tiling manuale + fluttuanti | Flessibile | Più stati da gestire |

**Raccomandazione:** colonne scorrevoli, con app fluttuanti per finestre di dialogo. Sul Game Boy Advance il tiling non serve: una sola app a tutto schermo.

## 4. Dock (stile macOS)

| Opzione | Pro | Contro |
|---|---|---|
| Plank | Semplice, leggero | Poco mantenuto |
| nwg-dock (Hyprland) | Leggero, Wayland | Legato a Hyprland |
| Dash to Dock (GNOME) | Maturo | Legato a GNOME |
| Dock nostro (processo separato) | Su misura, piccolo | Da scrivere |

**Raccomandazione:** dock nostro, piccolo, come processo separato che legge la lista app dal monolite. Il monolite non sa nulla dell'interfaccia (vedi `docs/architecture.md`, sez. 3).

## 5. Barra in alto (stile macOS)

| Opzione | Pro | Contro |
|---|---|---|
| Waybar (MIT) | Configurabile, diffusa | Solo con compositor Wayland compatibili |
| Quickshell / Astal (LGPL/GPL) | Barre fatte in QML o TypeScript, belle | Dipendenze da gestire |
| Barra nostra | Controllo totale | Da scrivere |

**Raccomandazione:** barra nostra con menu globale e orologio, come processo separato. Il menu mostra solo ciò che l'app attiva espone.

## 6. App a schermo intero

- Un'app può chiedere lo schermo intero; la barra e il dock si nascondono.
- Con i "spazi" (workspace) scorrevoli, ogni app a tutto schermo occupa uno spazio suo (come macOS Spaces).
- Lo schermo intero deve essere un **permesso** che il monolite concede e può revocare (coerente con la sez. 4 di `docs/architecture.md`).

**Raccomandazione:** schermo intero come stato del compositor; il monolite verifica il permesso. Sul GBA è l'unico modo.

---

## 7. Domande aperte per Ivan

1. **Base dello stack grafico:** A (riusare Wayland/Linux come compatibilità) oppure B (protocollo nostro)?
   - **B, protocollo nostro** – più lavoro, più coerente con la filosofia del progetto. *(consigliato)*
   - **A, compatibilità Wayland** – si ha subito un desktop, ma si eredita la complessità di Linux.
2. **Stile di default:** macOS-like con tiling nascosto, oppure tiling scorrevole con barra macOS?
   - **Tiling scorrevole + barra macOS** – più leggero e unico. *(consigliato)*
   - **Macchina macOS pura** – più familiare, meno innovativo.
3. **Primo schermo da supportare:** solo grande (PC/ARM64) o anche GBA fin da subito?
   - **Prima grande, poi GBA** – si valida la logica dove c'è spazio. *(consigliato)*

## 8. Prossimi passi (proposta)

1. Scegliere le domande sopra.
2. Scrivere `gui/` con una prima bozza: compositor scorrevole, dock, barra.
3. Test su schermo 640×480 (render già disponibili nel progetto) prima del GBA.
