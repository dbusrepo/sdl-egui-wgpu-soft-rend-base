//! `sdl_egui_wgpu_base`
//!
//! ...
#![allow(unused_results)]

use anyhow::Result;
#[cfg(target_os = "linux")]
use tikv_jemallocator::Jemalloc;

mod app;
use app::{App, AppConfiguration, constants};
use constants::{HEIGHT, WIDTH};

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    let app_configuration = AppConfiguration { width: WIDTH, height: HEIGHT };
    App::new(app_configuration)?.borrow().run()
}
