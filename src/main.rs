//! `sdl_egui_wgpu_base`
//!
//! ...
#![allow(unused_results)]

use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
#[cfg(target_os = "linux")]
use tikv_jemallocator::Jemalloc;

mod app;
use app::{App, AppConfiguration, constants, log_utils};
use constants::{HEIGHT, TARGET_FPS, TITLE, WIDTH};

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long = "width", default_value_t = WIDTH)]
    /// Window width
    width: u32,

    #[arg(long = "height", default_value_t = HEIGHT)]
    /// Window height
    height: u32,

    #[arg(long = "fullscreen", default_value_t = false)]
    /// Enable fullscreen mode
    fullscreen: bool,

    #[arg(long = "vsync", default_value_t = true)]
    /// Enable vsync
    vsync: bool,

    #[arg(long = "target_fps", default_value_t = TARGET_FPS)]
    /// Target frames per second
    target_fps: i32,
}

impl From<Cli> for AppConfiguration {
    fn from(cli: Cli) -> Self {
        AppConfiguration::new(
            TITLE,
            cli.width,
            cli.height,
            cli.fullscreen,
            cli.vsync,
            cli.target_fps,
        )
    }
}

fn main() -> Result<()> {
    dotenv().ok();
    log_utils::init_logging()?;
    App::start(Cli::parse().into())
}
