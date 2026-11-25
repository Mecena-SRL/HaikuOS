/* Harness headless (libmgba): esegue la ROM e confronta la VRAM (mode 3) con la composizione attesa.
 * Build: gcc -O1 -o vram_check host/vram_check.c -lmgba   (pacchetto libmgba-dev, host/font.h)
 * Uso:   ./vram_check ROM save
 * Sequenza: vedi main(). Ogni controllo confronta la VRAM (o solo la superficie 0) con l'atteso.
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>
#include "font.h"

#define W 240
#define H 160
#define SW 64
#define SH 48

static uint16_t vram[W * H], expect[W * H];
static uint16_t GREEN_C, BLUE_C, ORANGE_C;

static uint16_t rgb(int r, int g, int b) { return r | (g << 5) | (b << 10); }

/* Glifo 3x5: righe da 3 bit, bit alto = colonna sinistra. Minuscole come maiuscole. */
static const uint8_t *glyph(char ch) {
    if (ch >= 'a' && ch <= 'z') ch -= 32;
    const char *p = strchr(FONT_CHARS, ch);
    return FONT[p ? (int)(p - FONT_CHARS) : 0];
}

/* Disegna `disp` nella superficie 0 come glyph::draw_wrapped: 15 colonne, 6 righe visibili,
 * origine (2,2), riga di 6 px. Il carattere a `ppos` è arancio, gli altri verdi. */
static void draw_disp(const char *disp, int ppos, const char *status) {
    size_t n = strlen(disp);
    size_t total_lines = (n - 1) / 15 + 1;
    size_t start = total_lines > 6 ? total_lines - 6 : 0;
    for (size_t i = 0; i < n; i++) {
        size_t line = i / 15;
        if (line < start) continue;
        int row = (int)(line - start);
        uint16_t col = (int)i == ppos ? ORANGE_C : GREEN_C;
        for (int ry = 0; ry < 5; ry++)
            for (int col3 = 0; col3 < 3; col3++) {
                if (!(glyph(disp[i])[ry] & (0x4 >> col3))) continue;
                int gx = 2 + (int)(i % 15) * 4 + col3, gy = 2 + row * 6 + ry;
                if (gx < SW && gy < SH) expect[(20 + gy) * W + 10 + gx] = col;
            }
    }
    if (status) {
        size_t m = strlen(status);
        for (size_t i = 0; i < m; i++)
            for (int ry = 0; ry < 5; ry++)
                for (int col3 = 0; col3 < 3; col3++) {
                    if (!(glyph(status[i])[ry] & (0x4 >> col3))) continue;
                    int gx = 2 + (int)i * 4 + col3, gy = 40 + ry;
                    if (gx < SW && gy < SH) expect[(20 + gy) * W + 10 + gx] = BLUE_C;
                }
    }
}

/* Atteso: sfondo nero; superficie 0 = testo con pending a `ppos`; con `others`: superfici 2 blu e 3 arancio. */
static void compose_expected(const char *disp, int ppos, const char *status, int others) {
    for (int i = 0; i < W * H; i++) expect[i] = 0;
    draw_disp(disp, ppos, status);
    if (others) {
        int xs[2] = {50, 100}, ys[2] = {50, 70};
        uint16_t cs[2] = {BLUE_C, ORANGE_C};
        for (int k = 0; k < 2; k++)
            for (int y = 0; y < SH; y++)
                for (int x = 0; x < SW; x++) expect[(ys[k] + y) * W + xs[k] + x] = cs[k];
    }
}

/* Confronta la VRAM con l'atteso. region=1: solo la superficie 0 (10,20) 64x48. */
static int check_disp(struct mCore *c, const char *label, const char *disp, int ppos, const char *status, int region) {
    for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
    compose_expected(disp, ppos, status, !region);
    int diff = 0, first = -1;
    for (int i = 0; i < W * H; i++) {
        int x = i % W, y = i / W;
        if (region && (x < 10 || x >= 10 + SW || y < 20 || y >= 20 + SH)) continue;
        if (vram[i] != expect[i]) { diff++; if (first < 0) first = i; }
    }
    printf("%s: pixel diversi dall'atteso: %d", label, diff);
    if (diff) printf(" (primo a x=%d y=%d)", first % W, first / W);
    printf("\n");
    return diff;
}

/* Testo con pending in fondo (cursore in coda). */
static int check(struct mCore *c, const char *label, const char *text, char pending, const char *status, int region) {
    char disp[160];
    size_t n = strlen(text);
    memcpy(disp, text, n);
    disp[n] = pending;
    disp[n + 1] = 0;
    return check_disp(c, label, disp, (int)n, status, region);
}

int main(int argc, char **argv) {
    if (argc < 3) { puts("uso: vram_check ROM save|load"); return 1; }
    int save = strcmp(argv[2], "save") == 0;
    GREEN_C = rgb(4, 24, 6);
    BLUE_C = rgb(6, 14, 28);
    ORANGE_C = rgb(30, 16, 2);
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    /* Maschera GBA: A=1, Start=8, Select=4, Su=0x40, Giù=0x80, R=0x100, L=0x200.
     * save: Su 450, A 470 (inserisce 'A'), Select 490 (salva), Start 540 (esce dall'editor),
     *       Start 560 (esce dal desktop), A 580 (entra in docs), A 600 (apre docs/hello).
     *       Su 654 (pending 'A'), A x12 da 660 (testo a capo). Su 780 (pending 'B'),
     *       L 790 (cursore a 25), A 800 (inserisce 'B' a 25). Controlli: 400, 520, 640, 760, 840.
     * Riavvio nella stessa SRAM: controllo a 400 del testo salvato. */
    int diff = 0;
    for (int f = 0; f <= 840; f++) {
        int k = 0;
        if (f >= 300 && f < 302) k = 1;
        if (save && f >= 450 && f < 452) k = 0x40;
        if (save && f >= 470 && f < 472) k = 1;
        if (save && f >= 490 && f < 492) k = 4;
        if (save && f >= 540 && f < 542) k = 8;
        if (save && f >= 560 && f < 562) k = 8;
        if (save && ((f >= 580 && f < 582) || (f >= 600 && f < 602))) k = 1;
        if (save && f >= 654 && f < 656) k = 0x40;
        if (save && f >= 660 && f < 732 && ((f - 660) % 6) < 2) k = 1;
        if (save && f >= 780 && f < 782) k = 0x40;
        if (save && f >= 790 && f < 792) k = 0x200;
        if (save && f >= 800 && f < 802) k = 1;
        c->setKeys(c, k);
        c->runFrame(c);
        if (!save && f == 400) diff += check(c, "caricato (400)", "hello haikuosA", ' ', NULL, 0);
        if (save && f == 520) diff += check(c, "salvato (520)", "hello haikuosA", 'A', "SALVATO", 0);
        if (save && f == 640) diff += check(c, "file aperto dallo spazio di lavoro (640)", "hello haikuosA", ' ', NULL, 1);
        if (save && f == 760) diff += check(c, "a capo (760)", "hello haikuos" "AAAAAAAAAAAAA", 'A', NULL, 1);
        /* Dopo Su (B), L (cursore a 25) e A: "hello haikuos" + 12 A + B + A, pending B a 26. */
        if (save && f == 840) diff += check_disp(c, "cursore in mezzo (840)", "hello haikuos" "AAAAAAAAAAAA" "BBA", 26, NULL, 1);
    }
    if (save) {
        /* Riavvio nella stessa SRAM: il testo salvato (fino a Select) torna nella demo. */
        c->reset(c);
        for (int f = 0; f <= 400; f++) {
            c->setKeys(c, (f >= 300 && f < 302) ? 1 : 0);
            c->runFrame(c);
        }
        diff += check(c, "dopo riavvio (400)", "hello haikuosA", ' ', NULL, 0);
    }
    printf(diff == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return diff == 0 ? 0 : 2;
}
