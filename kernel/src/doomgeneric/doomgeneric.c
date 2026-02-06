// Minimal doomgeneric.c stub - demo rendering loop
#include "doomgeneric.h"
#include "doomgeneric_syscalls.h"
#include <stdint.h>

// Simple static framebuffer for demo (320x200)
static uint32_t fb[320 * 200];

void doomgeneric_main(void) {
    // Draw gradient
    for (int y = 0; y < 200; ++y) {
        for (int x = 0; x < 320; ++x) {
            uint8_t r = (x * 255) / 320;
            uint8_t g = (y * 255) / 200;
            uint8_t b = 128;
            fb[y * 320 + x] = (r << 16) | (g << 8) | b;
        }
    }

    // Send framebuffer to kernel for blit
    DG_Sys_Framebuffer(fb, 320, 200);

    // Wait for keypress (exit on 'q' or Ctrl+C)
    while (1) {
        int k = DG_Sys_ReadKey();
        if (k == 'q' || k == 'Q' || k == 3) {
            break;
        }
        // Idle loop
    }
}
