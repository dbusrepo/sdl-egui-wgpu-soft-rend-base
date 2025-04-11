use derive_builder::Builder;
use smartstring::alias::String;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum InputActionBehavior {
    Normal,
    DetectRepeat,
    DetectInitialPressOnly,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum InputActionState {
    Released,
    Pressed,
    WaitingForRelease,
}

#[derive(Builder, Debug)]
pub(super) struct InputAction {
    #[builder(setter(into))]
    pub name: String,
    #[builder(default = "InputActionBehavior::DetectRepeat")]
    behavior: InputActionBehavior,
    #[builder(setter(skip), default = "InputActionState::Released")]
    state:    InputActionState,
    #[builder(setter(skip), default = "0")]
    amount:   i32,
}

impl InputAction {
    pub(super) fn tap(&mut self) {
        self.press();
        self.release();
    }

    pub(super) fn press(&mut self) {
        self.press_with(1);
    }

    pub(super) fn press_with(&mut self, amount: i32) {
        if self.state != InputActionState::WaitingForRelease {
            let mut add_amount = || self.amount = self.amount.saturating_add(amount);
            match self.behavior {
                InputActionBehavior::Normal => {
                    add_amount();
                },
                InputActionBehavior::DetectRepeat if self.state != InputActionState::Pressed => {
                    add_amount();
                },
                _ => {},
            }
            self.state = InputActionState::Pressed;
        }
    }

    pub(super) fn release(&mut self) {
        self.state = InputActionState::Released;
    }

    pub(super) fn is_pressed(&self) -> bool {
        self.amount != 0
    }

    pub(super) fn reset(&mut self) {
        self.state = InputActionState::Released;
        self.amount = 0;
    }

    /// Returns the current amount, and resets it according to the state and behavior.
    ///
    /// For keys, this is the number of times the key was pressed since it was last checked.
    /// For mouse movement, this is the distance moved.
    pub(super) fn get_amount(&mut self) -> i32 {
        let ret_val = self.amount;
        if ret_val != 0 {
            if self.state == InputActionState::Released {
                self.amount = 0;
            } else if self.behavior == InputActionBehavior::DetectInitialPressOnly {
                self.state = InputActionState::WaitingForRelease;
                self.amount = 0;
            }
        }
        ret_val
    }
}
