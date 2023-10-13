use paddle::{Frame, FrameHandle};

use crate::render::{RenderSettings, RenderTask};
use crate::{
    EnqueueNewRender, Main, RequestNewRender, SECONDARY_H, SECONDARY_W, SECONDARY_X, SECONDARY_Y,
};

pub(crate) struct RenderSettingsView {
    current_quality: u32,
}

impl Frame for RenderSettingsView {
    type State = ();

    const WIDTH: u32 = SECONDARY_W;
    const HEIGHT: u32 = SECONDARY_H;
}

impl RenderSettingsView {
    pub(crate) fn init() -> FrameHandle<Self> {
        let this = Self { current_quality: 0 };
        let handle = paddle::register_frame_no_state(this, (SECONDARY_X, SECONDARY_Y));
        handle.listen(Self::ping_next_job);
        handle.listen(Self::render_done);
        handle
    }

    /// Construct a new job from current settings and send it to main.
    ///
    /// paddle event listener
    fn ping_next_job(&mut self, _: &mut (), _msg: &RequestNewRender) {
        let settings = self.render_settings();
        let num_jobs = match self.current_quality {
            0..=2 => 32,
            3 => 128,
            4 => 256,
            5 => 512,
            _ => 1024,
        };

        let jobs = RenderTask::new(Main::area(), settings).divide(num_jobs);
        paddle::send::<_, Main>(EnqueueNewRender(jobs));
    }

    /// paddle event listener
    fn render_done(&mut self, _: &mut (), _msg: &crate::RenderFinished) {
        self.current_quality += 1;
    }

    pub(crate) fn render_settings(&mut self) -> RenderSettings {
        // self.current_quality += 1;

        let mut resolution = 1.0;
        let samples;
        let recursion;
        match self.current_quality {
            0 => {
                resolution = 0.25;
                samples = 1;
                recursion = 2;
            }
            1 => {
                resolution = 0.5;
                samples = 1;
                recursion = 4;
            }
            2 => {
                samples = 4;
                recursion = 4;
            }
            3 => {
                samples = 4;
                recursion = 100;
            }
            4 => {
                samples = 64;
                recursion = 100;
            }
            5 => {
                samples = 256;
                recursion = 200;
            }
            6 | _ => {
                // very slightly better bright spot reflection
                samples = 1024;
                recursion = 512;
            }
        }
        RenderSettings {
            resolution: (
                (Main::WIDTH as f32 * resolution) as u32,
                (Main::HEIGHT as f32 * resolution) as u32,
            ),
            samples,
            recursion,
        }
    }
}
