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

/* Composizione attesa: sfondo nero; superficie 0 = `text` in verde e `pending` in arancio;
 * superficie 2 blu, superficie 3 arancio (sopra). */
static void compose_expected(const char *text, char pending, const char *status) {
    struct { int x, y; uint16_t c; } s[3] = {
        {10, 20, rgb(4, 24, 6)}, {50, 50, rgb(6, 14, 28)}, {100, 70, rgb(30, 16, 2)}};
    for (int i = 0; i < W * H; i++) expect[i] = 0;
    size_t n = strlen(text);
    for (size_t i = 0; i <= n; i++) {
        char ch = i < n ? text[i] : pending;
        uint16_t col = i < n ? s[0].c : rgb(30, 16, 2);
        for (int ry = 0; ry < 5; ry++)
            for (int col3 = 0; col3 < 3; col3++) {
                if (!(glyph(ch)[ry] & (0x4 >> col3))) continue;
                int gx = 2 + (int)i * 4 + col3, gy = 2 + ry;
                if (gx < SW && gy < SH) expect[(s[0].y + gy) * W + s[0].x + gx] = col;
            }
    }
    if (status) {
        size_t m = strlen(status);
        for (size_t i = 0; i < m; i++)
            for (int ry = 0; ry < 5; ry++)
                for (int col3 = 0; col3 < 3; col3++) {
                    if (!(glyph(status[i])[ry] & (0x4 >> col3))) continue;
                    int gx = 2 + (int)i * 4 + col3, gy = 10 + ry;
                    if (gx < SW && gy < SH) expect[(s[0].y + gy) * W + s[0].x + gx] = rgb(6, 14, 28);
                }
    }
    for (int k = 1; k < 3; k++)
        for (int y = 0; y < SH; y++)
            for (int x = 0; x < SW; x++) expect[(s[k].y + y) * W + s[k].x + x] = s[k].c;
}

/* Confronta la VRAM con l'atteso. Ritorna il numero di pixel diversi. */
static int check(struct mCore *c, const char *label, const char *text, char pending, const char *status) {
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
    compose_expected(text, pending, status);
    int diff = 0, first = -1;
    for (int i = 0; i < W * H; i++)
        if (vram[i] != expect[i]) { diff++; if (first < 0) first = i; }
    printf("%s: pixel diversi dall'atteso: %d", label, diff);
    if (diff) printf(" (primo a x=%d y=%d)", first % W, first / W);
    printf("\n");
    return diff;
}

int main(int argc, char **argv) {
    if (argc < 3) { puts("uso: vram_check ROM save|load"); return 1; }
    int save = strcmp(argv[2], "save") == 0;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    /* Maschera GBA: A=1, Start=8, Select=4, Su=0x40.
     * save: Su a 450, A a 470 (aggiunge 'A'), Select a 490 (salva). Controllo a 520.
     * load: nessun tasto. Controllo a 400: il testo salvato deve essere ricaricato. */
    int diff = 0;
    for (int f = 0; f <= 520; f++) {
        int k = 0;
        if (f >= 300 && f < 302) k = 1;
        if (save && f >= 450 && f < 452) k = 0x40;
        if (save && f >= 470 && f < 472) k = 1;
        if (save && f >= 490 && f < 492) k = 4;
        c->setKeys(c, k);
        c->runFrame(c);
        if (!save && f == 400) diff += check(c, "caricato (400)", "hello haikuosA", ' ', NULL);
        if (save && f == 520) diff += check(c, "salvato (520)", "hello haikuosA", 'A', "SALVATO");
    }
    if (save) {
        /* Riavvio nella stessa SRAM: il testo salvato deve tornare nella demo. */
        c->reset(c);
        for (int f = 0; f <= 400; f++) {
            c->setKeys(c, (f >= 300 && f < 302) ? 1 : 0);
            c->runFrame(c);
        }
        diff += check(c, "dopo riavvio (400)", "hello haikuosA", ' ', NULL);
    }
    printf(diff == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return diff == 0 ? 0 : 2;
}
