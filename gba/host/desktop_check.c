/* Harness headless (libmgba): controlla i colori di desktop::paint in VRAM (mode 3).
 * Build: gcc -O1 -o desktop_check host/desktop_check.c -lmgba
 * Uso:   ./desktop_check haiku-gba.gba
 * Sequenza: vedi keys_at(). Controllo a 1000.
 * Verifica: ogni pixel è una delle 4 tinte, e alcuni punti chiave hanno la tinta attesa.
 */
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>

#define W 240
#define H 160
#define KEY_A 1
#define KEY_START 8
#define KEY_RIGHT 16
#define KEY_LEFT 32
#define KEY_DOWN 128
#define KEY_SELECT 4
#define KEY_B 2
#define HOLD 4
#define PERIOD 10

static uint16_t vram[W * H];

/* Tinte (15 bit, r | g<<5 | b<<10), come in src/desktop.rs. */
static const uint16_t T[4] = {
    1 | (2 << 5) | (6 << 10),        /* T0 sfondo */
    31 | (31 << 5) | (31 << 10),     /* T1 bianco */
    31 | (16 << 5) | (0 << 10),      /* T2 arancio */
    0 | (20 << 5) | (31 << 10),      /* T3 blu */
};

typedef struct { int x, y, tint; const char *nome; } punto;


/* Tasti per frame. A a 300 (fine misure), Start a 500 (fine editor), Destra a 600, A a 610 (apre il terminale).
 * Poi SELECT (digitazione), 11 giù + A = 'L', 5 giù + A = 'S', sinistra esegue LS. Controllo a 800. */
static int keys_at(int f) {
    if (f >= 300 && f < 304) return KEY_A;
    if (f >= 500 && f < 504) return KEY_START;
    if (f >= 600 && f < 604) return KEY_RIGHT;
    if (f >= 610 && f < 614) return KEY_A;
    if (f >= 620 && f < 624) return KEY_SELECT;
    if (f >= 630 && f < 630 + 11 * PERIOD && (f - 630) % PERIOD < HOLD) return KEY_DOWN;
    if (f >= 630 + 11 * PERIOD && f < 630 + 11 * PERIOD + HOLD) return KEY_A;
    int g = 630 + 11 * PERIOD + PERIOD;
    if (f >= g && f < g + 5 * PERIOD && (f - g) % PERIOD < HOLD) return KEY_DOWN;
    if (f >= g + 5 * PERIOD && f < g + 5 * PERIOD + HOLD) return KEY_A;
    int h = g + 5 * PERIOD + PERIOD;
    if (f >= h && f < h + HOLD) return KEY_LEFT;
    /* Start apre il file manager (FILES), B nella radice torna al desktop. */
    if (f >= 850 && f < 854) return KEY_START;
    if (f >= 900 && f < 904) return KEY_B;
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    static uint16_t fm[W * H]; /* istantanea del file manager (frame 880) */
    for (int f = 0; f <= 1000; f++) {
        c->setKeys(c, keys_at(f));
        c->runFrame(c);
        if (f == 880)
            for (int i = 0; i < W * H; i++) fm[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
    }
    int fm_fuori = 0;
    for (int i = 0; i < W * H; i++) {
        int ok = 0;
        for (int k = 0; k < 4; k++) ok |= fm[i] == T[k];
        fm_fuori += !ok;
    }
    printf("pixel fuori tinta nel file manager (frame 880): %d\n", fm_fuori);
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);

    int fuori = 0;
    for (int i = 0; i < W * H; i++) {
        int ok = 0;
        for (int k = 0; k < 4; k++) ok |= vram[i] == T[k];
        fuori += !ok;
    }
    printf("pixel fuori dalle 4 tinte: %d\n", fuori);
    /* Output di LS: almeno un pixel di testo (T0) nella riga di output, dentro il corpo della finestra. */
    int testo = 0;
    for (int y = 51; y < 58; y++)
        for (int x = 128; x < 224; x++) testo += vram[y * W + x] == T[0];
    printf("pixel di testo nell'output di LS: %d\n", testo);
    fuori += testo == 0;
    fuori += fm_fuori == 0; /* il file manager deve essere apparso */

    /* Punti chiave: barra, orologio, icona 0 (evidenziata), icona 1, titolo, chiudi, corpo, testo, bordo. */
    punto p[] = {
        {0, 0, 3, "barra"}, {200, 1, 3, "barra"}, {1, 20, 0, "sfondo"},
        {16, 32, 2, "icona 0"}, {64, 32, 1, "icona 1 (selezionata)"},
        {127, 32, 2, "titolo"}, {227, 33, 1, "chiudi"},
        {127, 43, 1, "corpo"}, {124, 30, 3, "bordo"},
    };
    int errori = fuori;
    for (size_t k = 0; k < sizeof p / sizeof p[0]; k++) {
        uint16_t v = vram[p[k].y * W + p[k].x];
        int ok = v == T[p[k].tint];
        errori += !ok;
        if (!ok) printf("errore: %s a (%d,%d) = %04x, atteso tinta T%d\n", p[k].nome, p[k].x, p[k].y, v, p[k].tint);
    }
    printf(errori == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return errori == 0 ? 0 : 2;
}
