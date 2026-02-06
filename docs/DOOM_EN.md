# DOOM for ospabOS (English Version)

## What is this?

The legendary DOOM game ported to ospabOS! Runs directly from the command line.

## Current Status

**Version**: 0.44 (Demo)  
**Status**: Demo mode with fire effects

Currently implemented:
- Graphics buffer 320x200 with auto-scaling
- Keyboard handling (WASD, Space, Ctrl)
- Demo with animated visual effects
- "DOOM" text in center

## How to Run

```bash
[ospab]~> doom
```

## Controls

| Key | Action |
|-----|--------|
| W | Move forward |
| S | Move backward |
| A | Turn left |
| D | Turn right |
| Space | Use/Open door |
| Ctrl+C | Exit |

## Running in QEMU

```bash
# Linux
qemu-system-x86_64 -cdrom ospab-os-44.iso -m 256M

# Windows (PowerShell)
& "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom ospab-os-44.iso -m 256M
```

After boot:
1. Wait for prompt `[ospab]~>`
2. Type command: `doom`
3. Enjoy the fire effects!
4. Press Ctrl+C to exit

## Technical Details

### Architecture

```
doom
├── mod.rs              - Main DOOM module
├── Framebuffer         - 320x200x4 (RGBA) buffer
├── Scaling             - Automatic to screen
└── Keyboard            - Non-blocking input
```

### Sizes

- **DOOM Resolution**: 320x200 pixels (original)
- **Framebuffer**: 256 KB (320 * 200 * 4 bytes)
- **Scaling**: Automatic (x2, x3, x4 depending on screen)
- **FPS**: ~30-60 (depends on CPU)

### Kernel Integration

```rust
// kernel/src/doom/mod.rs
pub fn run_demo() {
    // Initialize
    init();
    
    // Game loop
    loop {
        process_input();
        if should_quit() { break; }
        
        draw_fire_effect(frame);
        draw_frame();
        
        frame += 1;
    }
}
```

## Roadmap

### v0.45 - Basic Engine
- [ ] Load DOOM1.WAD (shareware version)
- [ ] Render BSP tree
- [ ] Wall and floor textures
- [ ] Enemy sprites

### v0.46 - Gameplay
- [ ] Player movement
- [ ] Shooting
- [ ] Monsters (AI)
- [ ] Physics and collisions

### v0.47 - Sound (optional)
- [ ] Sound Blaster / AC'97 driver
- [ ] Music (MIDI)
- [ ] Sound effects

### v1.0 - Full Version
- [ ] All 9 Shareware episodes
- [ ] Menu system
- [ ] Save/Load
- [ ] Settings

## DOOM Ports History

DOOM is famous for running **everywhere**:
- ✅ MS-DOS (1993, original)
- ✅ Windows 95/XP/10/11
- ✅ Linux/Unix
- ✅ macOS
- ✅ PlayStation, Xbox, Nintendo
- ✅ Printers (Canon)
- ✅ ATMs
- ✅ Calculators (TI-83)
- ✅ Refrigerators (Samsung)
- ✅ Pregnancy tests (2020)
- ✅ **ospabOS (2026)**

## Credits

- **id Software** - For the legendary game (1993)
- **doomgeneric** - Portable DOOM engine
- **Fabien Sanglard** - "Game Engine Black Book: DOOM"
- **OSDev Community** - For knowledge and support

## Links

- [DOOM on Wikipedia](https://en.wikipedia.org/wiki/Doom_(1993_video_game))
- [doomgeneric on GitHub](https://github.com/ozkl/doomgeneric)
- [Game Engine Black Book: DOOM](https://fabiensanglard.net/gebbdoom/)
- [DOOM Shareware WAD](https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad)

---

**"If it can run code, it can run DOOM"** - Ancient programmer wisdom
