use anyhow::Result;

pub(super) struct World {}

impl World {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub(super) fn update(&mut self, step_time: f64) -> Result<()> {
        Ok(())
    }
}
