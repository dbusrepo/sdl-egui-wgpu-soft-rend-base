use anyhow::Result;
use log::LevelFilter;

use super::terminal::clear_terminal;

pub(crate) fn init_logging() -> Result<()> {
    let env_logger = Box::new(env_logger::builder().filter_level(LevelFilter::Debug).build());
    let egui_logger = Box::new(egui_logger::builder().build());
    multi_log::MultiLogger::init(vec![egui_logger, env_logger], log::Level::Debug)?;
    Ok(())
}

pub(crate) fn clear_logs() {
    #[allow(clippy::unwrap_used)]
    clear_terminal().unwrap();
    egui_logger::clear_log();
}
