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
mod engine;
mod frame_history;
mod gui;
mod input_action;
mod input_manager;
mod sdl_wgpu;
mod terminal;

use engine::{Engine, EngineConfiguration};
use frame_history::FrameHistory;
use gui::Gui;
use input_action::{InputAction, InputActionBuilder};
use input_manager::InputManager;
use sdl_wgpu::{SdlWgpu, SdlWgpuConfiguration};
use terminal::clear_terminal;

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
        title: &'static str, width: u32, height: u32, fullscreen: bool, vsync: bool,
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
    last_update:     u64,
    update_interval: u64,
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
    perf_frequency:  f64,
    time_multiplier: f64,
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

        let engine = Rc::new(RefCell::new(Engine::new(cfg.engine_cfg.clone(), sdl_wgpu.clone())?));

        let (input_actions, input_manager) = Self::init_input()?;

        let gui = Gui::new();

        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        let frame_history = FrameHistory::new(300, Self::get_performance_frequency() as f32);

        let stats = RefCell::new(AppStats {
            frame_history,
            mean_frame_time: 0.0,
            fps: 0.0,
            last_update: Self::get_performance_counter(),
            update_interval: Self::get_performance_frequency() / 4,
        });

        let app = Rc::new(RefCell::new(App {
            cfg,
            sdl_wgpu,
            platform,
            engine,
            gui: RefCell::new(gui),
            input_actions,
            input_manager: RefCell::new(input_manager),
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            perf_frequency: Self::get_performance_frequency() as f64,
            stats,
            time_multiplier: 1.0,
        }));

        app.borrow().gui.borrow_mut().init_gui(&app);

        clear_terminal()?;
        egui_logger::clear_log();

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

    fn update_stats(&self) {
        let mut stats = self.stats.borrow_mut();
        let now = Self::get_performance_counter();
        #[allow(clippy::arithmetic_side_effects)]
        if now - stats.last_update >= stats.update_interval {
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss
            )]
            {
                stats.mean_frame_time =
                    (f64::from(stats.frame_history.mean_frame_time()) / self.perf_frequency) as f32;

                stats.fps = (f64::from(stats.frame_history.fps()) * self.perf_frequency) as f32;
            }
            stats.last_update = now;
        }
    }

    fn update(&self, frame_period: f64) -> Result<()> {
        self.update_stats();
        let step_time = frame_period * self.time_multiplier;
        self.engine.borrow_mut().update(step_time)?;
        Ok(())
    }

    fn add_frame_time(&self, before_time: u64, end_time: u64) {
        #[allow(clippy::arithmetic_side_effects)]
        let frame_time = end_time - before_time;
        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        self.stats
            .borrow_mut()
            .frame_history
            .on_new_frame(end_time as f64, Some(frame_time as f32));
    }

    pub(crate) fn start(cfg: AppConfiguration) -> Result<()> {
        App::new(cfg)?.borrow().run()
    }

    fn run(&self) -> Result<()> {
        let mut event_pump = self
            .sdl_wgpu
            .borrow()
            .context
            .event_pump()
            .map_err(|e| anyhow!("Failed to get sdl event pump: {}", e))?;

        let frame_period = self.perf_frequency / f64::from(self.cfg.target_fps);
        let start_time = Self::get_performance_counter();
        let mut before_time = start_time;
        let mut over_sleep_time = 0_f64;
        let mut num_delays = 0_u32;
        let mut excess_time = 0_f64;
        let mut end_time: u64;
        let mut _frame_skips = 0_u32;

        'main: loop {
            if let EventOutcome::Quit = self.handle_events(&mut event_pump) {
                break 'main;
            }

            let App { sdl_wgpu, platform, engine, gui, .. } = self;

            {
                let ctx = {
                    let mut platform = platform.borrow_mut();
                    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                    let elapsed_sec =
                        before_time.saturating_sub(start_time) as f64 / self.perf_frequency;
                    platform.update_time(elapsed_sec);
                    platform.context()
                };

                gui.borrow_mut().show_ui(&ctx)?;
            }

            self.update(frame_period)?;

            sdl_wgpu.borrow_mut().init_render()?;
            engine.borrow_mut().render()?;
            gui.borrow_mut().render()?;

            sdl_wgpu.borrow_mut().present();
            gui.borrow_mut().clean()?;

            let after_time = Self::get_performance_counter();

            #[allow(
                clippy::arithmetic_side_effects,
                clippy::cast_precision_loss,
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            {
                let proc_time = (after_time - before_time) as f64 + over_sleep_time;

                if frame_period >= proc_time {
                    let sleep_time = frame_period - proc_time;
                    let sleep_ms = ((sleep_time) * 1e3 / self.perf_frequency) as u64;
                    if sleep_ms > 0 {
                        thread::sleep(Duration::from_millis(sleep_ms));
                    }
                    end_time = Self::get_performance_counter();
                    over_sleep_time = (end_time - after_time) as f64 - sleep_time;
                    if over_sleep_time < 0. {
                        over_sleep_time = 0.0;
                        loop {
                            if (end_time - before_time) as f64 >= frame_period {
                                break;
                            }
                            end_time = Self::get_performance_counter();
                        }
                    }
                    num_delays = 0;
                } else {
                    num_delays += 1;
                    if num_delays >= Self::NUM_DELAYS_PER_YIELD {
                        thread::yield_now();
                        num_delays = 0;
                    }
                    over_sleep_time = 0.0;
                    excess_time += proc_time - frame_period;
                    end_time = Self::get_performance_counter();
                }

                self.add_frame_time(before_time, end_time);

                before_time = end_time;

                let mut skips = 0;
                while excess_time >= frame_period && skips < Self::MAX_FRAME_SKIPS {
                    self.update(frame_period)?;
                    excess_time -= frame_period;
                    skips += 1;
                }
                _frame_skips += skips;
            }
        }

        Ok(())
    }

    fn get_input_action(&self, input_action_type: InputActionType) -> Rc<RefCell<InputAction>> {
        self.input_actions[input_action_type].clone()
    }

    fn process_input_actions(&self) {
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

        self.process_input_actions();

        EventOutcome::Continue
    }
}
