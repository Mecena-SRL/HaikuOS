/* Harness headless (libmgba): controlla la scrivania e il file manager in VRAM (mode 3).
 * Guidato dallo stato: ogni tasto parte solo quando la schermata attesa è visibile.
 * Build: gcc -O1 -o desktop_check host/desktop_check.c -lmgba
 * Uso:   ./desktop_check haiku-gba.gba
 * Verifica: ogni pixel è una delle 4 tinte; punti chiave del desktop, della finestra
 * del terminale (testo di LS nel corpo), e del file manager.
 */
#include <stdio.h>
#include <stdint.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>

#define W 240
#define H 160
#define KA 1
#define KB 2
#define KSELECT 4
#define KSTART 8
#define KRIGHT 16
#define KLEFT 32
#define KUP 64
#define KDOWN 128
#define HOLD 4
#define GAP 6
#define MAX_FRAMES 6000

static uint16_t px[W * H];
static const uint16_t T0 = 1 | (2 << 5) | (6 << 10), T1 = 31 | (31 << 5) | (31 << 10),
                      T2 = 31 | (16 << 5), T3 = (20 << 5) | (31 << 10);
static struct mCore *c;
static int frame_no;

static void grab(void) {
    for (int i = 0; i < W * H; i++) px[i] = c->rawRead16(c, 0x06000000 + 2 * i, -1);
}
static int at(int x, int y) { return px[y * W + x]; }
static int is_tint(uint16_t v) { return v == T0 || v == T1 || v == T2 || v == T3; }
static int desk(void) {
    return at(0, 0) == T3 && at(1, 20) == T0 && (at(16, 32) == T1 || at(16, 32) == T2) &&
           (at(64, 32) == T1 || at(64, 32) == T2);
}
static int fm(void) { return at(0, 0) == T0 && at(5, 6) == T3; }
static int term_open(void) { return at(124, 30) == T3 && at(170, 44) == T1; }
static int off_tint(void) {
    int n = 0;
    for (int i = 0; i < W * H; i++) n += !is_tint(px[i]);
    return n;
}
/* Pixel di testo nel corpo della finestra (T0 su T1, esclusi bordo e titolo). */
static int body_text(void) {
    int n = 0;
    for (int y = 43; y < 118 && y < H; y++)
        for (int x = 127; x < 225; x++) n += at(x, y) == T0;
    return n;
}

/* Tiene il tasto premuto HOLD frame e lascia GAP frame di pausa. */
static void press(int key) {
    for (int i = 0; i < HOLD + GAP; i++) {
        c->setKeys(c, i < HOLD ? key : 0);
        c->runFrame(c);
        frame_no++;
    }
    grab();
}
static void wait_frames(int n) {
    for (int i = 0; i < n; i++) {
        c->setKeys(c, 0);
        c->runFrame(c);
        frame_no++;
    }
    grab();
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    c = mCoreCreate(mPLATFORM_GBA);
    if (!c || !c->init(c)) { puts("init fail"); return 1; }
    mCoreInitConfig(c, NULL);
    if (!mCoreLoadFile(c, argv[1])) { puts("load fail"); return 1; }
    c->reset(c);
    int errori = 0;

    /* 1. Avvio: le schermate di test avanzano con Start; si aspetta la scrivania. */
    while (frame_no < MAX_FRAMES) {
        c->setKeys(c, (frame_no % 50 < 4) ? KA : (frame_no % 50 >= 25 && frame_no % 50 < 29) ? KSTART : 0);
        c->runFrame(c);
        frame_no++;
        grab();
        if (desk()) break;
    }
    printf("scrivania al frame %d\n", frame_no);
    if (!desk()) { printf("errore: scrivania non trovata\n"); return 2; }
    printf("pixel fuori tinta sul desktop: %d\n", off_tint());
    errori += off_tint() != 0;

    /* 2. Destra: seleziona l'icona 1 (TERM). */
    press(KRIGHT);
    if (!(at(16, 32) == T2 && at(64, 32) == T1)) { printf("errore: selezione icona 1\n"); errori++; }

    /* 3. A: apre il terminale. */
    press(KA);
    wait_frames(20);
    if (!term_open()) { printf("errore: finestra terminale non aperta\n"); errori++; }
    else printf("finestra terminale aperta\n");

    /* 4. Digitazione di "LS": Select attiva, giù+A sceglie la lettera, sinistra esegue. */
    press(KSELECT);
    for (int i = 0; i < 11; i++) press(KDOWN);
    press(KA); /* L */
    for (int i = 0; i < 5; i++) press(KDOWN);
    press(KA); /* S */
    press(KLEFT);
    wait_frames(60);
    int testo = body_text();
    printf("pixel di testo nel corpo del terminale: %d\n", testo);
    if (testo == 0) { printf("errore: nessun testo di LS\n"); errori++; }
    if (off_tint() != 0) { printf("errore: pixel fuori tinta dopo LS: %d\n", off_tint()); errori++; }

    /* 5. Start: apre il file manager (da verificare: dal terminale può non essere attivo). */
    press(KSTART);
    wait_frames(30);
    if (!fm()) { printf("errore: file manager non aperto\n"); errori++; }
    else printf("file manager aperto\n");
    printf("pixel fuori tinta nel file manager: %d\n", off_tint());
    errori += off_tint() != 0;

    /* 6. B: torna al desktop. */
    press(KB);
    wait_frames(20);
    if (!desk()) { printf("errore: ritorno al desktop\n"); errori++; }

    printf(errori == 0 ? "ESITO: OK\n" : "ESITO: FALLITO\n");
    c->deinit(c);
    return errori == 0 ? 0 : 2;
}
