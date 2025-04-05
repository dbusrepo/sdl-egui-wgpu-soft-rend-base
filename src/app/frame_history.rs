use egui::util::History;

pub(super) struct FrameHistory {
    frame_times: History<f32>,
}

impl FrameHistory {
    pub(super) fn new(max_len: usize, max_age: f32) -> Self {
        Self { frame_times: History::new(0..max_len, max_age) }
    }

    /// Call this once per frame.
    /// `now` is the current time in seconds (e.g. from a high-precision timer).
    /// `previous_frame_time` is the duration (in seconds) that the last frame took.
    pub(super) fn on_new_frame(&mut self, now: f64, maybe_previous_frame_time: Option<f32>) {
        let previous_frame_time = maybe_previous_frame_time.unwrap_or_default();
        // Update the latest entry with the known frame time.
        if let Some(latest) = self.frame_times.latest_mut() {
            *latest = previous_frame_time;
        }
        // Add a new projected entry.
        self.frame_times.add(now, previous_frame_time);
    }

    /// Returns the average frame time (in seconds) over the history window.
    pub(super) fn mean_frame_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    /// Returns the average FPS computed from the mean time interval.
    pub(super) fn fps(&self) -> f32 {
        // mean_time_interval returns the average time difference between consecutive frames.
        let mean_interval = self.frame_times.mean_time_interval().unwrap_or_default();
        if mean_interval > 0.0 { 1.0 / mean_interval } else { 0.0 }
    }
}
