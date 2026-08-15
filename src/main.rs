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
use heapless::Vec;

esp_bootloader_esp_idf::esp_app_desc!();

const LED_COUNT: usize = 80; // 8x10 matrix
const MATRIX_WIDTH: usize = 8;
const MATRIX_HEIGHT: usize = 10;

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

                let noise_value = perlin_3d(noise_x, noise_y, noise_z);

                // Map noise (-1.0 to 1.0) to hue (0 to 255)
                let hue = ((noise_value + 1.0) * 127.5) as u8;

                // Create HSV color with full saturation and varying brightness
                let brightness_val = ((noise_value + 1.0) * 127.5) as u8;
                let hsv = Hsv {
                    hue,
                    sat: 255,
                    val: brightness_val,
                };

                let rgb = hsv2rgb(hsv);

                // Map 2D coordinates to 1D array
                // Adjust indexing based on your matrix wiring (zigzag vs straight)
                let idx = y * MATRIX_WIDTH + x;
                frame_buffer[idx] = rgb;
            }
        }

        // Apply gamma correction and brightness limiting
        let corrected: Vec<RGB8, LED_COUNT> = gamma(brightness(frame_buffer.iter().cloned(), 32)).collect();

        // Write to LEDs
        led.write(corrected.iter().cloned()).ok();

        // Advance time for animation
        time_offset += 0.05;

        // Control animation speed
        delay.delay_millis(50);
    }
}
