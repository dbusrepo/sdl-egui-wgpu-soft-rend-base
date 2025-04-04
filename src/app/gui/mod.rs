use std::cell::RefCell;
use std::rc::{Rc, Weak};

use anyhow::{Context, Result, anyhow};
mod egui_render;
use egui_render::EguiRender;

use super::App;

pub(super) struct Gui<'a> {
    app:               Option<Weak<RefCell<App<'a>>>>,
    egui_render:       Option<EguiRender<'a>>,
    checkbox1_checked: bool,
}

impl<'a> Gui<'a> {
    pub(super) fn new() -> Self {
        Self { app: None, egui_render: None, checkbox1_checked: false }
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
        let Some(ref app_weak) = self.app else {
            return Err(anyhow!("App not initialized"));
        };

        let Some(app_rc) = app_weak.upgrade() else {
            return Err(anyhow!("App has been dropped"));
        };
        //
        let _app = app_rc.borrow();
        // let engine = app.engine_rc.borrow_mut();

        egui::Window::new("Settings").resizable(true).vscroll(true).show(ctx, |ui| {
            ui.label("This");
            ui.label("is");
            ui.label("a");
            ui.label("long");
            ui.label("list");
            ui.label("of");
            ui.label("labels");
            ui.label("to");
            ui.label("demonstrate");
            ui.label("scrolling!");
            #[allow(clippy::print_stdout)]
            if ui.button("Press me").clicked() {
                // println!("{}", app.get_msg());
                println!("you pressed me!");
            }
            ui.checkbox(&mut self.checkbox1_checked, "checkbox1");
            ui.end_row();
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
