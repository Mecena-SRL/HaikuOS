/* Adattato da zlib adler32.c (Copyright Mark Adler, licenza zlib). */
#define BASE 65521UL /* primo più grande sotto 65536 */

unsigned long adler32(unsigned long adler, const unsigned char *buf, unsigned long len) {
    unsigned long a = adler & 0xffff;
    unsigned long b = (adler >> 16) & 0xffff;
    unsigned long i;
    for (i = 0; i < len; i++) {
        a = (a + buf[i]) % BASE;
        b = (b + a) % BASE;
    }
    return (b << 16) | a;
}
