/* Harness headless (libmgba): percorso desktop -> lanciatore -> app.
 * Eseguito con libmgba 0.10 (apt libmgba-dev): esito OK.
 * Build: gcc -O1 -o launch_check host/launch_check.c -lmgba
 * Uso:   ./launch_check haiku-gba.gba
 * Sequenza (frame): A 300 (fine misure), Start 500 (fine editor demo), Destra x3 600/610/620 (NOTES, icona 3),
 *   A 630 (editor su docs/hello), Start 700 (torna al desktop, sel 3), Destra x3 800/810/820 (da NOTES a FILES, 6 icone),
 *   A 830 (spazio di lavoro), L 900 (torna al desktop).
 * Verifica: (0,0) nero nell'editor; (5,5) = blu del workspace; (0,0) = tinta barra sul desktop dopo ogni ritorno.
 */
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>

#define KEY_A 1
#define KEY_START 8
#define KEY_RIGHT 16
#define KEY_LEFT 32
#define KEY_L 512

#define BAR_T3 (0 | (20 << 5) | (31 << 10))   /* barra del desktop (T3) */
#define BLACK_PX 0
#define WS_BLUE (6 | (14 << 5) | (28 << 10))  /* barra del workspace */

static uint16_t px(struct mCore *c, int x, int y) {
    return c->rawRead16(c, 0x06000000 + 2 * (y * 240 + x), -1);
}

static int errori = 0;
static void atteso(const char *nome, uint16_t v, uint16_t atteso_v) {
    printf("%s: %04x (atteso %04x) %s\n", nome, v, atteso_v, v == atteso_v ? "ok" : "ERRORE");
    errori += v != atteso_v;
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    struct mCore *c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    for (int f = 0; f <= 1000; f++) {
        int k = 0;
        if (f >= 300 && f < 302) k = KEY_A;
        if (f >= 500 && f < 502) k = KEY_START;
        if (f >= 600 && f < 602) k = KEY_RIGHT; /* 0 -> 1 */
        if (f >= 610 && f < 612) k = KEY_RIGHT; /* 1 -> 2 */
        if (f >= 620 && f < 622) k = KEY_RIGHT; /* 2 -> 3 (NOTES) */
        if (f >= 630 && f < 632) k = KEY_A;
        if (f >= 700 && f < 702) k = KEY_START;
        if (f >= 800 && f < 802) k = KEY_RIGHT; /* 3 -> 4 */
        if (f >= 810 && f < 812) k = KEY_RIGHT; /* 4 -> 5 */
        if (f >= 820 && f < 822) k = KEY_RIGHT; /* 5 -> 0 (FILES) */
        if (f >= 830 && f < 832) k = KEY_A;
        if (f >= 900 && f < 902) k = KEY_L;
        c->setKeys(c, k);
        c->runFrame(c);
        if (f == 645) atteso("editor NOTES (0,0) nero", px(c, 0, 0), BLACK_PX);
        if (f == 715) atteso("desktop dopo editor (0,0) barra", px(c, 0, 0), BAR_T3);
        if (f == 895) atteso("workspace FILES (5,5) blu", px(c, 5, 5), WS_BLUE);
        if (f == 995) atteso("desktop dopo L (0,0) barra", px(c, 0, 0), BAR_T3);
    }
    printf(errori == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return errori == 0 ? 0 : 2;
}
