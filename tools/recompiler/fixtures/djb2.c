/* Hash djb2 su n byte. */
unsigned long djb2(const unsigned char *s, unsigned long n) {
    unsigned long h = 5381;
    unsigned long i;
    for (i = 0; i < n; i++) h = h * 33 + s[i];
    return h;
}
