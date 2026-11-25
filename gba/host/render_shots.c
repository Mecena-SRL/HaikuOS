/* Render da VRAM (libmgba): esegue la ROM con una sequenza di tasti e salva la VRAM
 * (mode 3, 240x160, RGB555) in file .raw ai frame indicati. Poi host/render_shots.py fa i PNG.
 * Build: gcc -O1 -o render_shots host/render_shots.c -lmgba
 * Uso:   ./render_shots ROM OUTDIR   (cancellare prima il .sav accanto alla ROM: stato pulito)
 */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>

#define W 240
#define H 160

/* Tasti GBA: A=1, B=2, Select=4, Start=8, Destra=0x10, Sinistra=0x20, Su=0x40, Giù=0x80, R=0x100, L=0x200. */
enum { A = 1, B = 2, SEL = 4, START = 8, RT = 0x10, LF = 0x20, UP = 0x40, DN = 0x80, RB = 0x100, LB = 0x200 };

/* Sequenza: a ogni frame `f` il tasto premuto è quello con f in [from, from+2). */
struct press { int from; int keys; };
static const struct press PRESS[] = {
    {300, A},                       /* salta la schermata delle misure */
    {450, UP}, {470, A},            /* inserisce 'A' */
    {490, SEL},                     /* salva */
    {540, START},                   /* esce dal visualizzatore: desktop */
    {580, RT}, {600, RT},           /* icona CALC */
    {620, A},                       /* apre la calcolatrice */
    {640, A}, {660, A}, {680, A},   /* premi tasti della calcolatrice */
    {720, B},                       /* chiude la finestra */
    {740, START},                   /* spazio di lavoro */
    {780, A},                       /* entra in docs */
    {800, A},                       /* apre docs/hello */
    {840, UP},                      /* carattere in attesa 'A' */
    {860, A}, {872, A}, {884, A}, {896, A}, {908, A}, {920, A}, {932, A},
    {960, LB}, {970, A},            /* cursore a sinistra, inserisce */
};
static const int NPRESS = sizeof(PRESS) / sizeof(PRESS[0]);

/* Frame a cui salvare la VRAM, con il nome del file. */
struct shot { int frame; const char *name; };
static const struct shot SHOTS[] = {
    {560, "01_desktop_lanciatore"},
    {700, "02_calcolatrice"},
    {770, "03_spazio_di_lavoro"},
    {815, "04_visualizzatore_testo"},
    {945, "05_editor"},
};
static const int NSHOTS = sizeof(SHOTS) / sizeof(SHOTS[0]);

static uint16_t vram[W * H];

int main(int argc, char **argv) {
    if (argc < 3) { puts("uso: render_shots ROM OUTDIR"); return 1; }
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    int last = SHOTS[NSHOTS - 1].frame;
    for (int f = 0; f <= last; f++) {
        int k = 0;
        for (int i = 0; i < NPRESS; i++)
            if (f >= PRESS[i].from && f < PRESS[i].from + 2) k |= PRESS[i].keys;
        c->setKeys(c, k);
        c->runFrame(c);
        for (int s = 0; s < NSHOTS; s++) {
            if (SHOTS[s].frame != f) continue;
            for (int i = 0; i < W * H; i++) vram[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
            char path[600];
            snprintf(path, sizeof path, "%s/%s.raw", argv[2], SHOTS[s].name);
            FILE *out = fopen(path, "wb");
            if (!out) { puts("open fail"); return 1; }
            fwrite(vram, 2, W * H, out);
            fclose(out);
            printf("frame %d -> %s\n", f, path);
        }
    }
    return 0;
}
