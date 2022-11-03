#![no_std]

use palette::rgb::Rgb;
use palette::FromColor;
use smart_leds::RGB;

use palette::{encoding::Srgb, Hsl};
use smart_leds::{SmartLedsWrite, RGB8};

use embedded_hal::blocking::delay::DelayMs;

// use micromath::F32Ext;

use embedded_graphics::{pixelcolor::Rgb888, prelude::Point, Pixel};

pub struct Row {
    speed: u8,
    x: u8,
    y_start: u8,
    y: u8,
    y_sub: u8,
    height: u8,
    fade: u8,
    age: u16,
}

impl Row {
    pub fn new(speed: u8, x: u8, y: u8, height: u8, fade: u8) -> Self {
        Self {
            speed,
            x,
            y_start: y,
            y,
            height,
            y_sub: 0,
            fade,
            age: 0,
        }
    }
    pub fn tick(&mut self) -> bool {
        let (new_y, wrapped) = self.y_sub.overflowing_add(self.speed);
        self.y_sub = new_y;
        if wrapped {
            self.y = self.y.saturating_add(1);
        }

        self.age = self.age.saturating_add(1);

        // TODO figure this out
        self.y >= self.height
            && self.age > 20000u16.saturating_div(self.fade as u16 * self.speed as u16)
    }
    pub fn iter(&self) -> impl Iterator<Item = Pixel<Rgb888>> + '_ {
        RowIterator::new(self.x, self.y_start, self.y, self.fade)
    }
}

struct RowIterator {
    x: u8,
    y: u8,
    y_start: u8,
    y_end: u8,
    fade: u8,
}

impl RowIterator {
    fn new(x: u8, y: u8, y_end: u8, fade: u8) -> Self {
        Self {
            x,
            y,
            y_start: y,
            y_end,
            fade,
        }
    }
}

impl Iterator for RowIterator {
    type Item = Pixel<Rgb888>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = if self.y > self.y_end {
            None
        } else {
            let color = if self.y == self.y_end {
                Rgb888::new(90, 120, 110)
            } else {
                let delta_max = self.y_end - self.y_start;
                let delta = self.y_end - self.y;

                let green_max = 255u8.saturating_sub(delta_max.saturating_mul(14));
                let g = green_max.saturating_sub(delta.saturating_mul(self.fade + 1));
                Rgb888::new(0, g, 0)
            };
            Some(Pixel(Point::new(self.x as i32, self.y as i32), color))
        };
        self.y += 1;
        res
    }
}

#[test]
fn wraps() {
    let mut row = Row::new(126, 0, 0, 5, 1);
    for y in [0, 0, 1, 1, 2] {
        row.tick();
        assert_eq!(y, row.y);
    }
}

#[test]
fn dies() {
    let mut row = Row::new(128, 0, 0, 2, 1);
    for shall_die in [false, false, false, true] {
        let dead = row.tick();
        assert_eq!(dead, shall_die);
    }
}

pub fn rainborrow<const NUM_LEDS: usize>(time: u8, brightness: f32, data: &mut [RGB8; NUM_LEDS]) {
    for i in 0..NUM_LEDS {
        let color: Hsl = Hsl::new(
            360. * time.wrapping_add((i as u8).wrapping_mul(4)) as f32 / 255.,
            1.0f32,
            brightness,
        );

        let rgb = palette::Srgb::from_color(color);
        let rgb = rgb.into_linear().into_format::<u8>();
        // let rgb: Rgb<Srgb, u8> = rgb.into_format::<u8>();
        data[i] = RGB8 {
            g: rgb.green,
            r: rgb.red,
            b: rgb.blue,
        };
    }
}

// ETOOMANYPUNS
// oklab-rainborrow
pub fn rainborrok<const NUM_LEDS: usize>(
    time: u16,
    lightness: f32,
    chroma: f32,
    aaa_my_eyes: f32,
    data: &mut [RGB8; NUM_LEDS],
) {
    // let aaa_my_eyes = 0.104;
    for i in 0..NUM_LEDS {
        let t_i = time as f32 + i as f32 * ((16 * 16) / NUM_LEDS) as f32;
        let color = palette::Oklch::new(lightness, chroma, t_i);
        //let gammad = lin.into_format();
        let rgb = palette::Srgb::from_color(color);
        let rgb = rgb.into_linear() * aaa_my_eyes;
        let rgb = rgb.into_format::<u8>();
        data[i] = RGB8 {
            g: rgb.green,
            r: rgb.red,
            b: rgb.blue,
        };
    }
}

pub fn chaser<const NUM_LEDS: usize>(time: u16, data: &mut [RGB8; NUM_LEDS]) {
    let offset = time as usize % NUM_LEDS;
    data[(offset + NUM_LEDS - 1) % NUM_LEDS] = RGB8 { g: 0, r: 0, b: 0 };

    let t_i = time as f32 * ((16 * 16) / NUM_LEDS) as f32;
    let color = palette::Oklch::new(0.9, 0.15, t_i);

    let rgb = palette::Srgb::from_color(color);
    let rgb = rgb.into_format::<u8>();
    data[offset] = RGB8 {
        g: rgb.green,
        r: rgb.red,
        b: rgb.blue,
    };
}

pub fn progress<const NUM_LEDS: usize>(time: u16, data: &mut [RGB8; NUM_LEDS]) {
    // let aaa_my_eyes = 0.104;

    let completeness = (time as u32 * NUM_LEDS as u32) / (u16::MAX as u32);
    for i in 0..completeness as usize {
        let rgb = palette::LinSrgb::new(64u8, 64, 64);

        data[i] = RGB8 {
            g: rgb.green,
            r: rgb.red,
            b: rgb.blue,
        };
    }
}

// lol so generic .. not
pub fn expanding_circle<const NUM_LEDS: usize>(time: u8, dampen: u8, data: &mut [RGB8; NUM_LEDS]) {
    const MID: isize = 3;

    const DIST: [[u8; 4]; 4] = [
        [0, 60, 120, 180],
        [60, 84, 134, 189],
        [120, 134, 169, 216],
        [180, 189, 216, 254],
    ];

    fn mk(time: u8) -> u8 {
        if time == 0 {
            return 1;
        }
        let mut res: u8 = 1;
        for tt in 0..time {
            res = res.wrapping_add((12 - tt).max(2));
        }
        res
    }

    let kk = 0;

    let k = time.wrapping_mul(10);

    let spd = 60;

    // let mut k = ((spd - time) + (time) / 2).wrapping_mul(time);
    // if time >= spd {
    //     k = time.wrapping_mul(15);
    // }

    // OR SOMETHING

    let spd = 80u8;

    let mut k = spd
        .saturating_sub(time / 2)
        .max(1)
        .min(20)
        .wrapping_mul(time);
    if time >= spd * 2 {
        k = time.wrapping_mul(15);
    }

    // const K: u8 = 45;
    // let k = (K.saturating_sub(time)).max(5).wrapping_mul(time);
    // defmt::debug!("{}", k);
    for i in 0..NUM_LEDS.min(49) {
        let mut x = i as isize % 7;
        let y = i as isize / 7;

        let zigzag = false;

        if zigzag {
            if y % 2 == 1 {
                x = 6 - x;
            }
        }

        // defmt::debug!("i,x,y {},{},{}", i, x, y,);

        let x = (x - MID).abs() as usize;
        let y = (y - MID).abs() as usize;
        // defmt::debug!("i,x',y' {},{},{}", i, x, y,);
        let d = (255 - DIST[x][y]).wrapping_add(k).saturating_sub(time);
        // defmt::info!("i,d {},{}", i, d);
        data[i] = RGB8 {
            g: (d / 2) / dampen,
            r: 0,
            b: (d).saturating_sub(time / 2) / dampen,
        };
    }
}

pub fn expanding_circle_2(time: u8, dampen: u8, amplify: u8, data: &mut [RGB8], grb: bool) {
    const WH: isize = 7;
    const MID: isize = WH / 2;

    const DIST: [[u8; 4]; 4] = [
        [0, 60, 120, 180],
        [60, 84, 134, 189],
        [120, 134, 169, 216],
        [180, 189, 216, 254],
    ];

    let spd = 80u8;

    let mut k = spd
        .saturating_sub(time / 2)
        .max(1)
        .min(20)
        .wrapping_mul(time);
    if time >= spd * 2 {
        k = time.wrapping_mul(15);
    }

    for i in 0..49 {
        let x = i as isize % WH;
        let y = i as isize / WH;

        let x = (x - MID).abs() as usize;
        let y = (y - MID).abs() as usize;
        let d = (255 - DIST[x][y]).wrapping_add(k).saturating_sub(time);

        let r = ((d / 2) / dampen).saturating_mul(amplify);
        let g = 0;

        data[i] = RGB8 {
            r: if grb { g } else { r },
            g: if grb { r } else { g },
            b: ((d).saturating_sub(time / 2) / dampen).saturating_mul(amplify),
        };
    }
}

// nice hsv fade thing, put it in a loop{}
pub fn fader<WS, D, const NUM_LEDS: usize>(data: &mut [RGB8; NUM_LEDS], mut ws: WS, mut delay: D)
where
    WS: SmartLedsWrite,
    <WS as SmartLedsWrite>::Color: From<RGB<u8>>,
    <WS as SmartLedsWrite>::Error: core::fmt::Debug,
    D: DelayMs<u16>,
{
    let mut j = 0u16;
    for i in 0..NUM_LEDS {
        // j as f32 / (L - L / 2) as f32 * 180.
        let h = (((j * 7) % 360) - 180) as f32;
        let lin = Hsl::new(h, 0.96, 0.4);
        let rgb = palette::rgb::Srgb::from_color(lin);
        let rrr: Rgb<Srgb, u8> = rgb.into_format::<u8>();
        data[i] = RGB8 {
            r: rrr.green,
            g: rrr.red,
            b: rrr.blue,
        };
        j += 1;
    }
    ws.write(data.iter().cloned()).unwrap();

    delay.delay_ms(60 as u16);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
