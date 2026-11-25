/* Harness headless (libmgba): controlla i colori di desktop::paint in VRAM (mode 3).
 * Build: gcc -O1 -o desktop_check host/desktop_check.c -lmgba
 * Uso:   ./desktop_check haiku-gba.gba
 * Sequenza: A a 300 (fine misure), Start a 500 (fine editor), Destra a 600, A a 610 (apre il terminale); controllo a 650.
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

static uint16_t vram[W * H];

/* Tinte (15 bit, r | g<<5 | b<<10), come in src/desktop.rs. */
static const uint16_t T[4] = {
    1 | (2 << 5) | (6 << 10),        /* T0 sfondo */
    31 | (31 << 5) | (31 << 10),     /* T1 bianco */
    31 | (16 << 5) | (0 << 10),      /* T2 arancio */
    0 | (20 << 5) | (31 << 10),      /* T3 blu */
};

typedef struct { int x, y, tint; const char *nome; } punto;

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    for (int f = 0; f <= 650; f++) {
        c->setKeys(c, (f >= 300 && f < 302) ? KEY_A : (f >= 500 && f < 502) ? KEY_START
                      : (f >= 600 && f < 602) ? KEY_RIGHT : (f >= 610 && f < 612) ? KEY_A : 0);
        c->runFrame(c);
    }
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);

    int fuori = 0;
    for (int i = 0; i < W * H; i++) {
        int ok = 0;
        for (int k = 0; k < 4; k++) ok |= vram[i] == T[k];
        fuori += !ok;
    }
    printf("pixel fuori dalle 4 tinte: %d\n", fuori);

    /* Punti chiave: barra, orologio, icona 0 (evidenziata), icona 1, titolo, chiudi, corpo, testo, bordo. */
    punto p[] = {
        {0, 0, 3, "barra"}, {10, 5, 1, "orologio"}, {1, 20, 0, "sfondo"},
        {16, 32, 2, "icona 0"}, {64, 32, 1, "icona 1 (selezionata)"},
        {127, 32, 2, "titolo"}, {227, 33, 1, "chiudi"},
        {127, 43, 1, "corpo"}, {129, 45, 0, "testo prompt"}, {124, 30, 3, "bordo"},
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
