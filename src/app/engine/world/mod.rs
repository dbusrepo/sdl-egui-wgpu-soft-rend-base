use anyhow::Result;

mod camera;

use camera::Camera;

pub(super) struct World {
    camera: Camera,
}

impl World {
    pub(super) fn new() -> Result<Self> {
        Ok(Self { camera: Camera::new() })
    }

    pub(super) fn update(&mut self, step_time: f64) -> Result<()> {
        self.camera.update(step_time);

        Ok(())
    }
}
