use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use egui_sdl2_platform::sdl2;
use nohash_hasher::BuildNoHashHasher;
use sdl2::keyboard::Keycode;

use super::input_action::InputAction;

type KeycodeHasher = BuildNoHashHasher<i32>;
type KeyActionMap = HashMap<i32, Rc<RefCell<InputAction>>, KeycodeHasher>;

pub(super) struct InputManager {
    key_actions:  KeyActionMap,
    pressed_keys: KeyActionMap,
}

impl InputManager {
    pub(super) fn new() -> Self {
        Self {
            key_actions:  HashMap::with_hasher(KeycodeHasher::default()),
            pressed_keys: HashMap::with_hasher(KeycodeHasher::default()),
        }
    }

    pub(super) fn map_to_key(&mut self, key: Keycode, action: &Rc<RefCell<InputAction>>) {
        self.key_actions.insert(key.into_i32(), action.clone());
    }

    pub(super) fn get_key_action(&self, key: Keycode) -> Option<Rc<RefCell<InputAction>>> {
        self.key_actions.get(&key.into_i32()).cloned()
    }

    pub(super) fn key_pressed(&mut self, key: Keycode) {
        if let Some(action) = self.key_actions.get(&key.into_i32()) {
            action.borrow_mut().press();
            self.pressed_keys.insert(key.into_i32(), action.clone());
        }
    }

    pub(super) fn key_released(&mut self, key: Keycode) {
        if let Some(action) = self.key_actions.get(&key.into_i32()) {
            action.borrow_mut().release();
            self.pressed_keys.remove(&key.into_i32());
        }
    }

    pub(super) fn release_all(&mut self) {
        for action in self.pressed_keys.values() {
            action.borrow_mut().release();
        }
        self.pressed_keys.clear();
    }
}
