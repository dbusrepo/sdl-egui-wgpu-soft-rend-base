use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use anyhow::{Context, Result, anyhow};
use log::Level;
mod egui_render;
use egui::{FontFamily, FontId, TextStyle, Window};
use egui_render::EguiRender;

use super::App;

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

    pub(super) fn init_gui(&mut self, app_rc: &Rc<RefCell<App<'a>>>) {
        let app_ref = app_rc.borrow();

        let egui_pass = Rc::new(RefCell::new(egui_wgpu_backend::RenderPass::new(
            &app_ref.sdl_wgpu_rc.borrow().device,
            app_ref.sdl_wgpu_rc.borrow().surface_format,
            1,
        )));

        let egui_render =
            EguiRender::new(egui_pass, app_ref.platform_rc.clone(), app_ref.sdl_wgpu_rc.clone());

        self.egui_render = Some(egui_render);
        self.app = Some(Rc::downgrade(app_rc));
    }

    pub(super) fn show_ui(&mut self, ctx: &egui::Context) -> Result<()> {
        configure_text_styles(ctx);

        let Some(ref app_weak) = self.app else {
            return Err(anyhow!("App not initialized"));
        };

        let Some(app_rc) = app_weak.upgrade() else {
            return Err(anyhow!("App has been dropped"));
        };

        let app = app_rc.borrow();
        // let engine = app.engine_rc.borrow_mut();

        if self.perf_window_visible {
            Window::new("Performance").show(ctx, |ui| {
                ui.label(format!("Mean Frame Time: {:.2} ms", app.mean_frame_time_sec() * 1e3));
                ui.label(format!("Mean FPS: {:.2}", app.fps()));
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

        // Window::new("Settings").resizable(true).vscroll(true).show(ctx, |ui| {
        Window::new("Settings").resizable(false).vscroll(false).show(ctx, |ui| {
            if ui.button("Show perf").clicked() {
                self.perf_window_visible = !self.perf_window_visible;
            }
            if ui.button("Show log").clicked() {
                self.log_window_visible = !self.log_window_visible;
            }
            if ui.button("Press me to add a debug log").clicked() {
                log::debug!("A Debug Info");
            }

            // ui.label("This");
            // ui.label("is");
            // ui.label("a");
            // ui.label("long");
            // ui.label("list");
            // ui.label("of");
            // ui.label("labels");
            // ui.label("to");
            // ui.label("demonstrate");
            // ui.label("scrolling!");

            // #[allow(clippy::print_stdout)]
            // if ui.button("Press me").clicked() {
            //     // println!("{}", app.get_msg());
            //     println!("you pressed me!");
            // }

            // ui.checkbox(&mut self.checkbox1_checked, "checkbox1");
            // ui.end_row();
            //     ui.label("Hello, world!");
            //     if ui.button("Greet").clicked() {
            //         println!("Hello, world!");
            //     }
            //     ui.horizontal(|ui| {
            //         ui.label("Color: ");
            //         ui.color_edit_button_rgba_premultiplied(&mut color);
            //     });
            //     ui.code_editor(&mut text);
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
