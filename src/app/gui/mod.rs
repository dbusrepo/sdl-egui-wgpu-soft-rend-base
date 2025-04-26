use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use anyhow::{Context, Result};
use egui::{FontFamily, FontId, TextStyle, Window};
use log::Level;

use super::egui_render::EguiRender;
use super::{App, AppStats};

pub(super) struct Gui<'a> {
    app:                 Option<Weak<RefCell<App<'a>>>>,
    egui_render:         Option<EguiRender<'a>>,
    perf_window_visible: bool,
    log_window_visible:  bool,
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    let text_styles: BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(18.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        (TextStyle::Button, FontId::new(14.0, Proportional)),
        (TextStyle::Small, FontId::new(10.0, Proportional)),
    ]
    .into();

    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

impl<'a> Gui<'a> {
    pub(super) fn new() -> Self {
        Self {
            app:                 None,
            egui_render:         None,
            perf_window_visible: true,
            log_window_visible:  false,
        }
    }

    pub(super) fn init_gui(&mut self, app: &Rc<RefCell<App<'a>>>, egui_render: EguiRender<'a>) {
        self.app = Some(Rc::downgrade(app));
        self.egui_render = Some(egui_render);
    }

    pub(super) fn show_ui(&mut self, ctx: &egui::Context) -> Result<()> {
        configure_text_styles(ctx);

        let app = self.app.as_ref().context("App not initialized")?;
        let app = app.upgrade().context("App has been dropped")?;
        let app = app.borrow();

        // let engine = app.engine.borrow_mut();

        if self.perf_window_visible {
            Window::new("Performance").show(ctx, |ui| {
                let AppStats { fps, mean_frame_time, .. } = *app.stats.borrow();
                ui.label(format!("Mean Frame Time: {:.2} ms", mean_frame_time * 1e3));
                ui.label(format!("Mean FPS: {fps:.2}"));
            });
        }

        if self.log_window_visible {
            Window::new("Log").show(ctx, |ui| {
                egui_logger::logger_ui()
                    .enable_regex(true)
                    .set_filter_level(Level::Debug)
                    .set_max_log_length(250)
                    .show_target(false)
                    .show(ui);
            });
        }

        Window::new("Settings").resizable(false).vscroll(false).show(ctx, |ui| {
            ui.checkbox(&mut self.perf_window_visible, "Show perf");
            ui.checkbox(&mut self.log_window_visible, "Show log");
        });

        Ok(())
    }

    pub(super) fn render(&mut self) -> Result<()> {
        let egui_render = self.egui_render.as_mut().context("EguiRender not initialized")?;
        egui_render.render()
    }

    pub(super) fn clean(&mut self) -> Result<()> {
        let egui_render = self.egui_render.as_mut().context("EguiRender not initialized")?;
        egui_render.clean()
    }
}
