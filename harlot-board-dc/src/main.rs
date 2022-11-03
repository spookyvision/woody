#![feature(generic_const_exprs)]

//! # Hardware Check
//!
//! This `libstd` program is for the ESP32-C3-DevKitC-02 board.

// Logging macros

use std::{
    collections::HashMap,
    num::Wrapping,
    sync::{Condvar, Mutex},
};

mod apa_spi;
mod wifi;

use std::{
    cell::RefCell,
    env,
    sync::{atomic::*, Arc},
    thread,
    time::*,
};

use apa_spi::{Apa, Pixel};
use color_mixer::strip::{Control, Segment, Srgb8, State};
use embedded_svc::{
    httpd::{Request, Response},
    io::{Io, Read, Write},
    storage::RawStorage,
    wifi::*,
};
use esp_idf_svc::{
    httpd::{Server, ServerRegistry},
    netif::EspNetifStack,
    nvs::EspDefaultNvs,
    nvs_storage::EspNvsStorage,
    sysloop::EspSysLoopStack,
    wifi::*,
};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use indexmap::IndexMap;
use log::*;

const FS_NAMESPACE: &'static str = "fs";

struct StdReader<R>(R);

impl<R: Read> std::io::Read for StdReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let res = Read::read(&mut self.0, buf)
            .map_err(|_e| std::io::Error::new(std::io::ErrorKind::Other, "oh no"));
        res
    }
}

const SEGMENTS_FILE: &'static str = "segments.json";
fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let app_config = CONFIG;

    let mut now = 0;
    let sys_start = Instant::now();

    println!("Hello, world!");

    log::debug!("Hello, log!");
    log::info!("Hello, log!");
    log::warn!("Hello, log!");

    let nvs = Arc::new(esp_idf_svc::nvs::EspDefaultNvs::new()?);
    let storage = EspNvsStorage::new_default(nvs.clone(), FS_NAMESPACE, true)?;

    let load_segments = || {
        let len = storage.len(SEGMENTS_FILE)?.unwrap_or_default();
        let mut buf = vec![];
        buf.resize(len, 0u8);
        let (loaded_buf, _) = storage
            .get_raw(SEGMENTS_FILE, &mut buf)?
            .unwrap_or_default();
        let de: IndexMap<String, Segment> = serde_json::from_slice(loaded_buf)?;
        Ok(de)
    };

    let res: Result<_, anyhow::Error> = load_segments();

    if let Err(e) = &res {
        log::error!("could not load data: {:?}", e);
    }
    let mut segments = res.unwrap_or_default();

    let brightness = 10;
    if segments.is_empty() {
        let chill_fac = 100;
        let some_segs = [
            Segment::new(
                1,
                false,
                Srgb8::new(255, 150, 0),
                Srgb8::new(255, 10, 120),
                0,
                chill_fac,
                brightness,
            ),
            Segment::new(
                1,
                false,
                Srgb8::new(166, 0, 255),
                Srgb8::new(2, 192, 192),
                1,
                chill_fac,
                brightness,
            ),
            Segment::new(
                1,
                false,
                Srgb8::new(20, 200, 141),
                Srgb8::new(200, 176, 20),
                2,
                chill_fac,
                brightness,
            ),
            Segment::new(
                1,
                false,
                Srgb8::new(200, 20, 30),
                Srgb8::new(200, 200, 10),
                3,
                chill_fac,
                brightness,
            ),
        ];

        segments.extend(some_segs.into_iter().map(|s| (s.to_uuid_string(), s)));
    }

    let segments = Arc::new(Mutex::new(segments));

    let mut apa_config = apa_spi::Config::default();
    apa_config.length = 512;
    const LEN: usize = 32;
    let mut apa: Apa = Apa::new(apa_config);
    let moar_chill = 1000;
    let state = State::new(
        segments
            .lock()
            .unwrap()
            .iter()
            .map(|(id, seg)| seg)
            .cloned(),
    );

    loop {
        let mut led_start = 0;

        let log_f = |s: String| log::warn!("{s}");
        let log_f = |_s| {};
        let segments = segments.lock().unwrap().clone();

        for (_id, seg) in segments {
            let color = seg.color_at(now);
            let segment_color = Pixel::new(color.red, color.green, color.blue, seg.brightness());
            for i in led_start..led_start + seg.length() {
                apa.set_pixel(i, segment_color, log_f);
            }
            led_start += seg.length();
            apa.flush();
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        let dt = Instant::now().duration_since(sys_start);
        now = dt.as_millis() as u32;
    }
}
