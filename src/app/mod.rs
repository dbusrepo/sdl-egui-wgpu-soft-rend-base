use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};

use anyhow::{Result, anyhow};
use egui_sdl2_platform::sdl2::EventPump;
use egui_sdl2_platform::{Platform, sdl2};
use enum_map::{Enum, EnumMap, enum_map};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use time::Duration;

pub(crate) mod constants;
mod egui_render;
mod engine;
mod frame_history;
mod gui;
mod input_action;
mod input_manager;
pub(crate) mod log_utils;
mod screen_quad;
mod sdl_wgpu;
mod terminal;

use egui_render::EguiRender;
use engine::{Engine, EngineConfiguration};
use frame_history::FrameHistory;
use gui::Gui;
use input_action::{InputAction, InputActionBuilder};
use input_manager::InputManager;
use screen_quad::ScreenQuad;
use sdl_wgpu::{SdlWgpu, SdlWgpuConfiguration};

#[derive(Copy, Clone, Debug, Enum)]
enum InputActionType {
    ActionA,
    // MoveForward,
    // MoveBackward,
    // MoveLeft,
    // MoveRight,
    // MoveUp,
    // MoveDown,
    // LookLeft,
    // LookRight,
    // LookUp,
    // LookDown,
}

type InputActionMap = EnumMap<InputActionType, Rc<RefCell<InputAction>>>;

pub(crate) struct AppConfiguration {
    sdl_wgpu_cfg: Rc<RefCell<SdlWgpuConfiguration>>,
    engine_cfg:   Rc<RefCell<EngineConfiguration>>,
    target_fps:   i32,
}

impl AppConfiguration {
    pub(crate) fn new(
        title: &'static str,
        width: u32,
        height: u32,
        fullscreen: bool,
        vsync: bool,
        target_fps: i32,
    ) -> Self {
        let sdl_wgpu_cfg =
            Rc::new(RefCell::new(SdlWgpuConfiguration { title, width, height, fullscreen, vsync }));

        let engine_cfg = Rc::new(RefCell::new(EngineConfiguration {}));

        AppConfiguration { sdl_wgpu_cfg, engine_cfg, target_fps }
    }
}

struct AppStats {
    frame_history:   FrameHistory,
    mean_frame_time: f32,
    fps:             f32,
}

pub(crate) struct App<'a> {
    cfg:             AppConfiguration,
    sdl_wgpu:        Rc<RefCell<SdlWgpu<'a>>>,
    platform:        Rc<RefCell<Platform>>,
    engine:          Rc<RefCell<Engine<'a>>>,
    gui:             RefCell<Gui<'a>>,
    input_actions:   InputActionMap,
    input_manager:   RefCell<InputManager>,
    stats:           RefCell<AppStats>,
    time_multiplier: f32,
}

pub(crate) enum EventOutcome {
    Quit,
    Continue,
}

impl App<'_> {
    const MAX_FRAME_SKIPS: u32 = 5;
    const NUM_DELAYS_PER_YIELD: u32 = 16;

    pub(crate) fn new(cfg: AppConfiguration) -> Result<Rc<RefCell<Self>>> {
        let sdl_wgpu = Rc::new(RefCell::new(SdlWgpu::new(cfg.sdl_wgpu_cfg.clone())?));

        let platform = Rc::new(RefCell::new(Platform::new(sdl_wgpu.borrow().window.size())?));

        let egui_render = EguiRender::new(platform.clone(), sdl_wgpu.clone());

        let screen_quad = ScreenQuad::new(sdl_wgpu.clone());

        log_utils::clear_logs();

        let engine = Rc::new(RefCell::new(Engine::new(cfg.engine_cfg.clone(), screen_quad)?));

        let (input_actions, input_manager) = Self::init_input()?;

        let gui = Gui::new();

        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        let frame_history = FrameHistory::new(300, 1.0);

        let stats = RefCell::new(AppStats { frame_history, mean_frame_time: 0., fps: 0. });

        let app = Rc::new(RefCell::new(App {
            cfg,
            sdl_wgpu,
            platform,
            engine,
            gui: RefCell::new(gui),
            input_actions,
            input_manager: RefCell::new(input_manager),
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            stats,
            time_multiplier: 1.0,
        }));

        app.borrow().gui.borrow_mut().init_gui(&app, egui_render);

        log::info!("App initialized");
        log::info!("Number of logical cores: {}", num_cpus::get());

        Ok(app)
    }

    fn init_input() -> Result<(InputActionMap, InputManager)> {
        let input_actions_map = Self::init_input_actions()?;

        let mut input_manager = InputManager::new();

        input_manager.map_to_key(Keycode::A, &input_actions_map[InputActionType::ActionA]);

        Ok((input_actions_map, input_manager))
    }

    fn init_input_actions() -> Result<InputActionMap> {
        let mut input_action_builder = InputActionBuilder::default();

        let action_a = Rc::new(RefCell::new(
            input_action_builder
                .name("pressA".to_string())
                .build()
                .map_err(|_err| anyhow!("Failed to build input action"))?,
        ));

        #[allow(clippy::mem_forget)]
        Ok(enum_map! {
            InputActionType::ActionA => action_a.clone(),
        })
    }

    #[allow(unsafe_code)]
    fn get_performance_counter() -> u64 {
        unsafe { sdl2::sys::SDL_GetPerformanceCounter() }
    }

    #[allow(unsafe_code)]
    fn get_performance_frequency() -> u64 {
        unsafe { sdl2::sys::SDL_GetPerformanceFrequency() }
    }

    fn update(&self, frame_time_s: f32) -> Result<()> {
        let dt = frame_time_s * self.time_multiplier;
        self.process_input_actions(dt);
        self.engine.borrow_mut().update(dt)?;
        Ok(())
    }

    pub(crate) fn start(cfg: AppConfiguration) -> Result<()> {
        App::new(cfg)?.borrow().run()
    }

    #[allow(
        clippy::arithmetic_side_effects,
        clippy::cast_precision_loss,
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn run(&self) -> Result<()> {
        let mut event_pump = self
            .sdl_wgpu
            .borrow()
            .context
            .event_pump()
            .map_err(|e| anyhow!("Failed to get sdl event pump: {}", e))?;

        let perf_frequency = Self::get_performance_frequency() as f64;
        let frame_ticks = perf_frequency / f64::from(self.cfg.target_fps);
        let start_ticks = Self::get_performance_counter();
        let stats_update_interval = perf_frequency as u64 / 4;
        let mut last_stats_update = start_ticks;
        let mut before_ticks = start_ticks;
        let mut over_sleep_ticks = 0_f64;
        let mut num_delays = 0_u32;
        let mut excess_ticks = 0_f64;
        let mut end_ticks: u64;
        let mut frame_skips = 0_u32;

        let tick_to_sec = |ticks: f64| -> f64 { ticks / perf_frequency };
        let tick_to_msec = |ticks: f64| -> f64 { tick_to_sec(ticks) * 1e3 };

        let mut update = || {
            let mut update_stats = || {
                let mut stats = self.stats.borrow_mut();
                let now = Self::get_performance_counter();
                if now - last_stats_update >= stats_update_interval {
                    stats.mean_frame_time = stats.frame_history.mean_frame_time();
                    stats.fps = stats.frame_history.fps();
                    last_stats_update = now;
                }
                self.sdl_wgpu.borrow_mut().set_window_title(
                    format!(
                        "{} - FPS: {:.2} - Mean frame time: {:.2} ms",
                        self.cfg.sdl_wgpu_cfg.borrow().title,
                        stats.fps,
                        stats.mean_frame_time * 1e3
                    )
                    .as_str(),
                );
            };

            update_stats();
            self.update(tick_to_sec(frame_ticks) as f32)
        };

        #[allow(clippy::shadow_unrelated)]
        let update_frame_history = |before_ticks: u64, end_ticks: u64| {
            let before_time_s = tick_to_sec(before_ticks as f64);
            let end_time_s = tick_to_sec(end_ticks as f64);
            let frame_duration_s = (end_time_s - before_time_s) as f32;
            self.stats.borrow_mut().frame_history.on_new_frame(end_time_s, Some(frame_duration_s));
        };

        'main: loop {
            if let EventOutcome::Quit = self.handle_events(&mut event_pump) {
                break 'main;
            }

            let App { sdl_wgpu, platform, engine, gui, .. } = self;

            {
                let ctx = {
                    let mut platform = platform.borrow_mut();
                    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                    let elapsed_time_s =
                        before_ticks.saturating_sub(start_ticks) as f64 / perf_frequency;
                    platform.update_time(elapsed_time_s);
                    platform.context()
                };

                gui.borrow_mut().show_ui(&ctx)?;
            }

            update()?;

            sdl_wgpu.borrow_mut().init_render()?;
            engine.borrow_mut().render()?;
            gui.borrow_mut().render()?;

            sdl_wgpu.borrow_mut().present();
            gui.borrow_mut().clean()?;

            let after_ticks = Self::get_performance_counter();

            {
                let proc_ticks = (after_ticks - before_ticks) as f64 + over_sleep_ticks;

                if frame_ticks >= proc_ticks {
                    let sleep_ticks = frame_ticks - proc_ticks;
                    let sleep_time_ms = tick_to_msec(sleep_ticks) as u64;
                    if sleep_time_ms > 0 {
                        thread::sleep(Duration::from_millis(sleep_time_ms));
                    }
                    end_ticks = Self::get_performance_counter();
                    over_sleep_ticks = (end_ticks - after_ticks) as f64 - sleep_ticks;
                    if over_sleep_ticks < 0. {
                        over_sleep_ticks = 0.;
                        loop {
                            if (end_ticks - before_ticks) as f64 >= frame_ticks {
                                break;
                            }
                            end_ticks = Self::get_performance_counter();
                        }
                    }
                    num_delays = 0;
                } else {
                    num_delays += 1;
                    if num_delays >= Self::NUM_DELAYS_PER_YIELD {
                        thread::yield_now();
                        num_delays = 0;
                    }
                    over_sleep_ticks = 0.;
                    excess_ticks += proc_ticks - frame_ticks;
                    end_ticks = Self::get_performance_counter();
                }

                update_frame_history(before_ticks, end_ticks);

                before_ticks = end_ticks;

                let mut skips = 0;
                while excess_ticks >= frame_ticks && skips < Self::MAX_FRAME_SKIPS {
                    update()?;
                    excess_ticks -= frame_ticks;
                    skips += 1;
                }
                frame_skips += skips;
            }
        }

        Ok(())
    }

    fn get_input_action(&self, input_action_type: InputActionType) -> Rc<RefCell<InputAction>> {
        self.input_actions[input_action_type].clone()
    }

    fn process_input_actions(&self, dt: f32) {
        let press_a = self.get_input_action(InputActionType::ActionA);
        let mut action = press_a.borrow_mut();
        if action.is_pressed() {
            action.get_amount();
        }
    }

    fn handle_events(&self, event_pump: &mut EventPump) -> EventOutcome {
        let ctx = self.platform.borrow_mut().context();
        // let egui_wants_pointer_input = ctx.wants_pointer_input();
        // let egui_is_context_menu_open = ctx.is_context_menu_open();
        // let egui_wants_keyboard_input = ctx.wants_keyboard_input();
        let egui_wants_keyboard_input = ctx.wants_pointer_input();

        let mut input_manager = self.input_manager.borrow_mut();

        if egui_wants_keyboard_input {
            input_manager.release_all();
        }

        let mut sdl_wgpu = self.sdl_wgpu.borrow_mut();

        // Handle sdl events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    return EventOutcome::Quit;
                },
                Event::Window { window_id, win_event, .. } if window_id == sdl_wgpu.window.id() =>
                    match win_event {
                        WindowEvent::Close => return EventOutcome::Quit,
                        WindowEvent::SizeChanged(w, h) =>
                            if w > 0 && h > 0 {
                                #[allow(clippy::as_conversions, clippy::cast_sign_loss)]
                                {
                                    sdl_wgpu.surface_configuration.width = w as u32;
                                    sdl_wgpu.surface_configuration.height = h as u32;
                                }
                                sdl_wgpu
                                    .surface
                                    .configure(&sdl_wgpu.device, &sdl_wgpu.surface_configuration);
                            },
                        _ => {},
                    },
                Event::KeyDown { keycode: Some(key), .. } if !egui_wants_keyboard_input =>
                    input_manager.key_pressed(key),
                Event::KeyUp { keycode: Some(key), .. } =>
                    if !egui_wants_keyboard_input {
                        input_manager.key_released(key);
                    },
                _ => {},
            }

            // Let the egui platform handle the event
            self.platform.borrow_mut().handle_event(&event, &sdl_wgpu.context, &sdl_wgpu.video);
        }

        EventOutcome::Continue
    }
}
