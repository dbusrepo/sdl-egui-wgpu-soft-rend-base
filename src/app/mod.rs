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

use engine::Engine;
use frame_history::FrameHistory;
use gui::Gui;
use input_action::{InputAction, InputActionBuilder};
use input_manager::InputManager;
use sdl_wgpu::SdlWgpu;

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

pub(crate) struct App<'a> {
    sdl_wgpu_rc:    Rc<RefCell<SdlWgpu<'a>>>,
    platform_rc:    Rc<RefCell<Platform>>,
    engine_rc:      Rc<RefCell<Engine<'a>>>,
    gui:            RefCell<Gui<'a>>,
    input_actions:  InputActionMap,
    input_manager:  RefCell<InputManager>,
    frame_history:  RefCell<FrameHistory>,
    perf_frequency: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct AppConfiguration {
    pub width:  u32,
    pub height: u32,
}

pub(crate) enum EventOutcome {
    Quit,
    Continue,
}

impl App<'_> {
    const MAX_FRAME_SKIPS: u32 = 5;
    const NUM_DELAYS_PER_YIELD: u32 = 16;
    const TARGET_FPS: f64 = 60.;
    const TITLE: &'static str = "App";

    pub(crate) fn new(cfg: AppConfiguration) -> Result<Rc<RefCell<Self>>> {
        let AppConfiguration { width, height } = cfg;

        let sdl_wgpu_rc = Rc::new(RefCell::new(SdlWgpu::new(Self::TITLE, width, height)?));

        let platform_rc = Rc::new(RefCell::new(Platform::new(sdl_wgpu_rc.borrow().window.size())?));

        let engine_rc = Rc::new(RefCell::new(Engine::new(sdl_wgpu_rc.clone())?));

        let (input_actions, input_manager) = Self::init_input()?;

        let gui = Gui::new();

        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        let frame_history = FrameHistory::new(300, Self::get_performance_frequency() as f32);

        let app_rc = Rc::new(RefCell::new(App {
            sdl_wgpu_rc,
            platform_rc,
            engine_rc,
            gui: RefCell::new(gui),
            input_actions,
            input_manager: RefCell::new(input_manager),
            frame_history: RefCell::new(frame_history),
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            perf_frequency: Self::get_performance_frequency() as f64,
        }));

        app_rc.borrow().gui.borrow_mut().init_gui(&app_rc);

        Ok(app_rc)
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

    fn mean_frame_time_sec(&self) -> f32 {
        #[allow(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss
        )]
        let mean_frame_time_sec =
            (f64::from(self.frame_history.borrow().mean_frame_time()) / self.perf_frequency) as f32;
        mean_frame_time_sec
    }

    fn fps(&self) -> f32 {
        #[allow(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss
        )]
        let fps = (f64::from(self.frame_history.borrow().fps()) * self.perf_frequency) as f32;
        fps
    }

    fn update(&self) -> Result<()> {
        self.engine_rc.borrow_mut().update()?;
        Ok(())
    }

    pub(crate) fn run(&self) -> Result<()> {
        let mut event_pump = self
            .sdl_wgpu_rc
            .borrow()
            .context
            .event_pump()
            .map_err(|e| anyhow!("Failed to get sdl event pump: {}", e))?;

        let frame_period = self.perf_frequency / Self::TARGET_FPS;
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

            let App { sdl_wgpu_rc, platform_rc, engine_rc, gui, frame_history, .. } = self;

            {
                let ctx = {
                    let mut platform = platform_rc.borrow_mut();
                    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                    let elapsed_sec =
                        before_time.saturating_sub(start_time) as f64 / self.perf_frequency;
                    platform.update_time(elapsed_sec);
                    platform.context()
                };

                gui.borrow_mut().show_ui(&ctx)?;
            }

            self.update()?;

            sdl_wgpu_rc.borrow_mut().init_render()?;
            engine_rc.borrow_mut().render()?;
            gui.borrow_mut().render()?;

            sdl_wgpu_rc.borrow_mut().present();
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

                let frame_time = end_time - before_time;

                #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                frame_history.borrow_mut().on_new_frame(end_time as f64, Some(frame_time as f32));

                before_time = end_time;

                let mut skips = 0;
                while excess_time >= frame_period && skips < Self::MAX_FRAME_SKIPS {
                    self.update()?;
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
        let ctx = self.platform_rc.borrow_mut().context();
        // let egui_wants_pointer_input = ctx.wants_pointer_input();
        // let egui_is_context_menu_open = ctx.is_context_menu_open();
        // let egui_wants_keyboard_input = ctx.wants_keyboard_input();
        let egui_wants_keyboard_input = ctx.wants_pointer_input();

        let mut input_manager = self.input_manager.borrow_mut();

        if egui_wants_keyboard_input {
            input_manager.release_all();
        }

        let mut sdl_wgpu = self.sdl_wgpu_rc.borrow_mut();

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
            self.platform_rc.borrow_mut().handle_event(&event, &sdl_wgpu.context, &sdl_wgpu.video);
        }

        self.process_input_actions();

        EventOutcome::Continue
    }
}
