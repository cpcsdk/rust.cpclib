#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Minimal declarations needed by the FFI wrapper.
int PackLz77(int lzlen, int flags, int *startEscape, int endAddr, int memEnd, int type);
extern unsigned char *indata;
extern int inlen;
extern unsigned char outBuffer[65536];
extern int outPointer;

// Minimal FFI wrapper: compresses input to output buffer, returns output size or -1 on error
int pucrunch_compress(const uint8_t* input, size_t input_len, uint8_t* output, size_t* output_len) {
    // Setup global input
    indata = (unsigned char*)malloc(input_len);
    if (!indata) return -1;
    memcpy(indata, input, input_len);
    inlen = (int)input_len;
    outPointer = 0;

    // Use default pucrunch settings (raw, no load address, C64 mode)
    int lzlen = -1;
    int flags = 0x01; // F_2MHZ
    int startEscape = 0;
    int endAddr = 0x10000; // max
    int memEnd = 0x10000;
    int type = 64; // C64

    int res = PackLz77(lzlen, flags, &startEscape, endAddr, memEnd, type);
    if (res != 0) {
        free(indata);
        return -1;
    }
    // Copy output from static outBuffer
    memcpy(output, outBuffer, outPointer);
    *output_len = outPointer;
    free(indata);
    return 0;
}
