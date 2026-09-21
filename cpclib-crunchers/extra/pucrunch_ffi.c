#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Minimal declarations needed by the FFI wrapper.
int PackLz77(int lzlen, int flags, int *startEscape, int endAddr, int memEnd, int type);
int pucrunch_ffi_init(void);
extern unsigned char *pucrunch_indata;
extern int pucrunch_inlen;
extern unsigned char outBuffer[65536];
extern int outPointer;

// Minimal FFI wrapper: compresses input to output buffer, returns 0 on success
// and -1 on error (including output larger than `output_cap`, which is
// reported rather than written past the buffer). `*output_len` is the number
// of bytes produced. pucrunch keeps its state in globals, so callers must
// serialise calls (the Rust wrapper does).
int pucrunch_compress(const uint8_t* input, size_t input_len, size_t output_cap, uint8_t* output, size_t* output_len) {
    // Every static main() would have set up - see pucrunch_ffi_init.
    int default_lz_range = pucrunch_ffi_init();

    // Setup global input
    pucrunch_indata = (unsigned char*)malloc(input_len);
    if (!pucrunch_indata) return -1;
    memcpy(pucrunch_indata, input, input_len);
    pucrunch_inlen = (int)input_len;
    outPointer = 0;

    // Use default pucrunch settings (raw, no load address, C64 mode)
    int lzlen = default_lz_range;
    int flags = 0x01; // F_2MHZ
    int startEscape = 0;
    int endAddr = 0x10000; // max
    int memEnd = 0x10000;
    int type = 64; // C64

    int res = PackLz77(lzlen, flags, &startEscape, endAddr, memEnd, type);
    if (res != 0) {
        free(pucrunch_indata);
        return -1;
    }
    if (outPointer < 0 || (size_t)outPointer > output_cap) {
        free(pucrunch_indata);
        return -1;
    }
    // Copy output from static outBuffer
    memcpy(output, outBuffer, outPointer);
    *output_len = outPointer;
    free(pucrunch_indata);
    return 0;
}
