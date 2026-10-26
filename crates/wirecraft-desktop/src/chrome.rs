use gpui::{Rgba, rgb};
use std::sync::atomic::{AtomicBool, Ordering};
static LIGHT: AtomicBool = AtomicBool::new(false);
pub fn set_light(light: bool) {
    LIGHT.store(light, Ordering::Relaxed);
}
pub fn color(color: u32) -> Rgba {
    rgb(if LIGHT.load(Ordering::Relaxed) {
        match color {
            0x10161e => 0xf5f7f9,
            0x171d25 => 0xe9eef3,
            0x191f28 => 0xebf0f4,
            0x1c242e => 0xffffff,
            0x242c36 => 0xe1e7ec,
            0x34414d | 0x2d3b48 | 0x29323d => 0xcdd8df,
            0x255452 => 0xc3e9e1,
            0x1b2631 | 0x171f28 => 0xedf2f6,
            0x252f3c | 0x202b35 => 0xe2eaf1,
            0x173b3a => 0xd9efea,
            0x293846 | 0x253f45 => 0xc3dedf,
            0xe1e9ef | 0xe6edf3 => 0x243b53,
            0x70d5c6 | 0x75d4d6 => 0x0d766c,
            0x7f9bae | 0x8a9aab | 0x8fa6b5 | 0x96a9bb | 0x9bb0c3 | 0x92a8b8 | 0x91a5b7
            | 0x9babb9 | 0xa0b0c0 => 0x526b7c,
            0x303c49 | 0x2d3947 | 0x303f4b | 0x40505c => 0xc1cdd6,
            0x19242c => 0xedf2f6,
            0x257e81 => 0x84c9c2,
            other => other,
        }
    } else {
        color
    })
}
