#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{delay::Delay, rmt::Rmt, time::Rate};
use esp_hal_smartled::{RmtSmartLeds, color_order, buffer_size, WS2812_TIMING};
use log::info;
use noise_perlin::perlin_3d;
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
esp_bootloader_esp_idf::esp_app_desc!();

// Seeed Studio 6x10 RGB MATRIX for XIAO: 60 WS2812B LEDs, DIN on D0 (= GPIO0).
const LED_COUNT: usize = 60;
const MATRIX_WIDTH: usize = 6;
const MATRIX_HEIGHT: usize = 10;

/// Perlin noise realistically spans about -0.7..0.7 rather than the full
/// -1.0..1.0, so normalise with gain and clamp into `lo..=hi`.
fn map_noise(noise: f32, lo: u8, hi: u8) -> u8 {
    const GAIN: f32 = 1.4;
    let t = ((noise * GAIN) + 1.0) * 0.5; // -> roughly 0.0..1.0
    let t = if t < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        t
    };
    lo + (t * (hi - lo) as f32) as u8
}

#[esp_hal::main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    info!("Starting ESP32-C6 LED Matrix with Perlin Noise!");

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Configure RMT for WS2812 LEDs
    // Adjust the GPIO pin number based on your wiring
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let mut led = RmtSmartLeds::<{ buffer_size::<RGB8>(LED_COUNT) }, _, RGB8, color_order::Grb>::new_with_memsize(
        WS2812_TIMING,
        rmt.channel0,
        peripherals.GPIO0,
        4, // RMT memory blocks (2-8 available, 4 should be plenty for 80 LEDs)
    ).unwrap();

    let delay = Delay::new();

    let mut time_offset = 0.0f32;
    let mut frame_buffer: [RGB8; LED_COUNT] = [RGB8::default(); LED_COUNT];

    info!("Starting animation loop...");

    loop {
        // Generate noise pattern for each LED
        for y in 0..MATRIX_HEIGHT {
            for x in 0..MATRIX_WIDTH {
                // Calculate noise value with time animation
                let noise_x = (x as f32) * 0.3;
                let noise_y = (y as f32) * 0.3;
                let noise_z = time_offset;

                // Two decorrelated noise samples: one drives hue, one drives
                // value. Sampling the same field for both makes bright pixels
                // and red pixels the same pixels.
                let hue_noise = perlin_3d(noise_x, noise_y, noise_z);
                let val_noise = perlin_3d(noise_x, noise_y, noise_z + 64.0);

                // perlin_3d peaks well short of +/-1.0, so apply gain before
                // mapping into the 0..=255 range.
                let hue = map_noise(hue_noise, 0, 255);
                // Keep a floor under val so no cell ever goes fully black.
                let val = map_noise(val_noise, 96, 255);

                let hsv = Hsv { hue, sat: 255, val };

                let rgb = hsv2rgb(hsv);

                // Map 2D coordinates to 1D array
                // Adjust indexing based on your matrix wiring (zigzag vs straight)
                let idx = y * MATRIX_WIDTH + x;
                frame_buffer[idx] = rgb;
            }
        }

        // Gamma correction MUST come before the brightness reduction: applying
        // it afterwards pushes every channel into the flat 0/1 region of the
        // gamma table and the whole matrix goes dark.
        led.write(brightness(gamma(frame_buffer.iter().cloned()), 64)).ok();

        // Advance time for animation
        time_offset += 0.025;

        // Control animation speed
        delay.delay_millis(25);
    }
}
