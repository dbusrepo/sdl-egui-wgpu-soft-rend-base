//! `sdl_egui_wgpu_base`
//!
//! ...
#![allow(unused_results)]

use anyhow::Result;
use log::LevelFilter;
#[cfg(target_os = "linux")]
use tikv_jemallocator::Jemalloc;

mod app;
use app::{App, AppConfiguration, constants};
use constants::{HEIGHT, WIDTH};

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    let env_logger = Box::new(env_logger::builder().filter_level(LevelFilter::Debug).build());
    let egui_logger = Box::new(egui_logger::builder().build());
    multi_log::MultiLogger::init(vec![egui_logger, env_logger], log::Level::Debug)?;

    let app_configuration = AppConfiguration { width: WIDTH, height: HEIGHT };
    App::new(app_configuration)?.borrow().run()
}
