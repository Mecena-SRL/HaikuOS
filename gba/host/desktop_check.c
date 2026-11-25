/* Harness headless (libmgba): controlla la scrivania di prova in VRAM (mode 3).
 * Build: gcc -O1 -o desktop_check host/desktop_check.c -lmgba
 * Uso:   ./desktop_check haiku-gba.gba
 * Premi A a frame 300 (fine misure), Start a frame 500 (fine editor); la scrivania si controlla al frame 650.
 */
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>

#define W 240
#define H 160
#define KEY_A 1
#define KEY_START 8

static uint16_t vram[W * H];
static uint16_t expect[W * H];

static uint16_t rgb(int r, int g, int b) { return r | (g << 5) | (b << 10); }

static void fillr(int x, int y, int w, int h, uint16_t c) {
    for (int yy = y; yy < y + h; yy++)
        for (int xx = x; xx < x + w; xx++)
            if (xx >= 0 && xx < W && yy >= 0 && yy < H) expect[yy * W + xx] = c;
}

/* Composizione attesa: stessa scena di compositor::desktop_sketch. */
static void compose_expected(void) {
    fillr(0, 0, W, H, 0x2D6B);
    fillr(0, 0, W, 12, 0x0C63);
    fillr(40, 30, 120, 80, 0x7FFF);
    fillr(40, 30, 120, 12, 0x3A10);
    fillr(8, 20, 16, 16, 0x03E0);
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    for (int f = 0; f <= 650; f++) {
        c->setKeys(c, (f >= 300 && f < 302) ? KEY_A : (f >= 500 && f < 502) ? KEY_START : 0);
        c->runFrame(c);
    }
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
    compose_expected();
    int diff = 0, first = -1;
    for (int i = 0; i < W * H; i++)
        if (vram[i] != expect[i]) { diff++; if (first < 0) first = i; }
    printf("pixel diversi dall'atteso: %d", diff);
    if (diff) printf(" (primo a x=%d y=%d)", first % W, first / W);
    printf("\n");
    printf(diff == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return diff == 0 ? 0 : 2;
}
