// Harness headless (libmgba): esegue la ROM e controlla i pixel di VRAM in mode 3.
// Build: gcc -O1 -o vram_check gba/host/vram_check.c -lmgba  (libmgba-dev)
// Uso: ./vram_check gba/haiku-gba.gba
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/internal/gba/gba.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>

#define W 240
#define H 160
static uint16_t vram[W * H];

static void grab(struct mCore *c) {
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
}
static int count(uint16_t col) {
    int n = 0;
    for (int i = 0; i < W * H; i++) n += vram[i] == col;
    return n;
}
int main(int argc, char **argv) {
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    // A (bit 0) premuto a frame 300 e 500 per superare le schermate di misura e demo.
    int ok = 1;
    for (int f = 0; f < 700; f++) {
        c->setKeys(c, (f >= 300 && f < 302) || (f >= 500 && f < 502) ? 1 : 0);
        c->runFrame(c);
        if (f == 400) {
            grab(c);
            uint16_t green = 0x0000 | (4) | (24 << 5) | (6 << 10);
            uint16_t blue = 6 | (14 << 5) | (28 << 10);
            int g = count(green), b = count(blue);
            printf("frame %d: green=%d (atteso 5250) blue=%d (atteso 3000)\n", f, g, b);
            // Pixel di controllo: A, B, sovrapposizione (blu dove B è opaco, verde dove è trasparente), fondo nero.
            printf("A (25,35)=%04x atteso %04x\n", vram[35 * W + 25], green);
            printf("B (150,100)=%04x atteso %04x\n", vram[100 * W + 150], blue);
            printf("sovrapposizione blu (80,65)=%04x atteso %04x\n", vram[65 * W + 80], blue);
            printf("sovrapposizione trasparente (81,65)=%04x atteso %04x\n", vram[65 * W + 81], green);
            printf("fondo (10,10)=%04x atteso 0000\n", vram[10 * W + 10]);
            ok = g == 5250 && b == 3000 && vram[35 * W + 25] == green && vram[100 * W + 150] == blue
                 && vram[65 * W + 80] == blue && vram[65 * W + 81] == green && vram[10 * W + 10] == 0;
        }
    }
    printf(ok ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return ok ? 0 : 2;
}
