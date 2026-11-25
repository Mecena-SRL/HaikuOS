/* Harness headless (libmgba): esegue la ROM e confronta la VRAM (mode 3) con la composizione attesa.
 * Build: gcc -O1 -o vram_check host/vram_check.c -lmgba   (pacchetto libmgba-dev)
 * Uso:   ./vram_check haiku-gba.gba
 * Schermata demo: premi A a frame 300 e 500; la VRAM si controlla al frame 400.
 */
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>
#include <string.h>
#include "font.h"

#define W 240
#define H 160
#define SW 64
#define SH 48

static uint16_t vram[W * H], expect[W * H];

static uint16_t rgb(int r, int g, int b) { return r | (g << 5) | (b << 10); }

/* Glifo 3x5: righe da 3 bit, bit alto = colonna sinistra. Minuscole come maiuscole. */
static const uint8_t *glyph(char ch) {
    if (ch >= 'a' && ch <= 'z') ch -= 32;
    const char *p = strchr(FONT_CHARS, ch);
    return FONT[p ? (int)(p - FONT_CHARS) : 0];
}

/* Composizione attesa: sfondo nero; superficie 0 = testo "hello haikuos" in verde; 2 blu; 3 arancio (3 sopra). */
static void compose_expected(void) {
    struct { int x, y; uint16_t c; } s[3] = {
        {10, 20, rgb(4, 24, 6)}, {50, 50, rgb(6, 14, 28)}, {100, 70, rgb(30, 16, 2)}};
    for (int i = 0; i < W * H; i++) expect[i] = 0;
    /* Superficie 0: testo a (2,2) della superficie, avanzamento 4 px, ritaglio a 64x48. */
    const char *txt = "hello haikuos";
    for (size_t i = 0; i < strlen(txt); i++)
        for (int ry = 0; ry < 5; ry++)
            for (int col = 0; col < 3; col++) {
                if (!(glyph(txt[i])[ry] & (0x4 >> col))) continue;
                int gx = 2 + (int)i * 4 + col, gy = 2 + ry;
                if (gx < SW && gy < SH) expect[(s[0].y + gy) * W + s[0].x + gx] = s[0].c;
            }
    for (int k = 1; k < 3; k++)
        for (int y = 0; y < SH; y++)
            for (int x = 0; x < SW; x++) expect[(s[k].y + y) * W + s[k].x + x] = s[k].c;
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    for (int f = 0; f <= 400; f++) {
        c->setKeys(c, (f >= 300 && f < 302) ? 1 : 0);
        c->runFrame(c);
    }
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
    compose_expected();
    int diff = 0, first = -1, green = 0, blue = 0, orange = 0;
    for (int i = 0; i < W * H; i++) {
        if (vram[i] != expect[i]) { diff++; if (first < 0) first = i; }
        green += vram[i] == rgb(4, 24, 6);
        blue += vram[i] == rgb(6, 14, 28);
        orange += vram[i] == rgb(30, 16, 2);
    }
    printf("pixel verdi=%d blu=%d arancio=%d\n", green, blue, orange);
    printf("pixel diversi dall'atteso: %d", diff);
    if (diff) printf(" (primo a x=%d y=%d)", first % W, first / W);
    printf("\n");
    int ok = diff == 0;
    printf(ok ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return ok ? 0 : 2;
}
