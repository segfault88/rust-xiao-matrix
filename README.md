# rust-xiao-matrix

Perlin noise plasma on a Seeed Studio 6x10 RGB MATRIX for XIAO, driven by a XIAO ESP32-C6
in bare-metal Rust.

No RTOS, no `std`, no framework — just `esp-hal`, the RMT peripheral, and a loop that
computes 60 pixels of noise and shoves them down a wire about 40 times a second.

## What it does

Every frame, each pixel samples a 3D Perlin noise field where the third axis is time.
One sample picks the hue, a second sample (offset well away from the first) picks the
brightness. That gives you drifting colour blobs that don't all pulse in lockstep.

That's the whole program. It lives in `src/main.rs` and it's about a hundred lines.

## Hardware

- **XIAO ESP32-C6**
- **[Seeed Studio 6x10 RGB MATRIX for XIAO](https://wiki.seeedstudio.com/rgb_matrix_for_xiao/)**
  — 60 WS2812B LEDs, GRB colour order, thumb-sized at 21 x 17.5mm

The matrix is a XIAO-form-factor board that stacks straight onto the microcontroller via
its 7-pin headers, so there's no wiring to get wrong. Its DIN line lands on **D0**, which
is **GPIO0** on the ESP32-C6 — that's what the code drives:

```rust
peripherals.GPIO0,   // D0 on the XIAO header
```

**On power:** 60 WS2812Bs at full white would pull around 3.6A, which your USB port will
not enjoy. The code runs at brightness 64/255 with gamma correction, so real draw is a
small fraction of that and USB is fine. If you crank the brightness up, think about an
external 5V supply before you find out the hard way.

## Building and flashing

You'll need the RISC-V target and `espflash`:

```sh
rustup target add riscv32imac-unknown-none-elf
cargo install espflash
```

Then plug the board in and:

```sh
cargo run
```

`cargo run` is wired up to `espflash flash --monitor` in `.cargo/config.toml`, so that one
command builds, flashes, and drops you into the serial monitor. Ctrl-C to get out.

## Knobs worth turning

All in the main loop in `src/main.rs`:

| What | Where | Notes |
|---|---|---|
| Brightness | `brightness(gamma(...), 64)` | 0–255. Mind the power note above. |
| Animation speed | `time_offset += 0.025` | Bigger = faster drift. |
| Frame rate | `delay.delay_millis(25)` | 25ms is roughly 40fps. |
| Noise zoom | `x as f32 * 0.3` | Smaller = bigger, smoother blobs. |
| Colour spread | `map_noise(hue_noise, 0, 255)` | Narrow the range for a limited palette. |

## Things that bit me

Leaving these here.

**Gamma before brightness, always.** `smart-leds` says so in its docs and it means it:

```rust
brightness(gamma(pixels), 64)   // correct
gamma(brightness(pixels, 64))   // everything goes black
```

Gamma-correcting an already-dimmed value pushes every channel into the flat 0-and-1 region
at the bottom of the gamma table. The matrix looks dead. It isn't — it's being sent zeros.

**Don't drive hue and brightness from the same noise sample.** If they're the same number,
the only pixels that are ever bright are the ones at the top of the hue wheel, which is red.
You get one lonely red pixel blinking every ten seconds and a lot of confusion. Two samples,
offset in z.

**A single flickering pixel is good news.** It means RMT, the pin, and the wiring are all
fine and your problem is in the colour maths. Worth remembering before you start rewiring.

## Pixel layout

The panel is treated as 6 wide by 10 tall, with **progressive** wiring — every row running
the same direction:

```rust
const MATRIX_WIDTH: usize = 6;
const MATRIX_HEIGHT: usize = 10;

let idx = y * MATRIX_WIDTH + x;
```

Two ways that can be wrong, both easy to spot:

- **Pattern looks rotated or sheared diagonally** — width and height are swapped. Flip the
  two constants to 10 and 6.
- **Pattern looks torn or mirrored on alternate rows** — the panel is **serpentine**, not
  progressive. Reverse `x` when `y` is odd.

Neither hurts anything, they just look wrong. Get the noise field on screen first, then
sort the orientation out by eye.

## License

Do whatever you like with it.
