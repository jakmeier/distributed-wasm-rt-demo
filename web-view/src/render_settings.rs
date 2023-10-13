use paddle::{Frame, FrameHandle, Rectangle};

use crate::render::{RenderSettings, RenderTask};
use crate::ui_slider::Slider;
use crate::{
    palette, EnqueueNewRender, Main, RequestNewRender, PADDING, SECONDARY_H, SECONDARY_W,
    SECONDARY_X, SECONDARY_Y,
};

pub(crate) struct RenderSettingsView {
    preset_level: Option<u32>,
    recursion: Slider<u32>,
    samples: Slider<u32>,
}

impl Frame for RenderSettingsView {
    type State = ();

    const WIDTH: u32 = SECONDARY_W;
    const HEIGHT: u32 = SECONDARY_H;

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        self.recursion.draw(canvas);
        self.samples.draw(canvas);
    }

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        let mut adjusted = false;
        adjusted |= self.recursion.adjust(event);
        adjusted |= self.samples.adjust(event);
        if adjusted {
            self.preset_level = None;
        }
    }

    fn enter(&mut self, _state: &mut Self::State) {
        self.recursion.active();
        self.samples.active();
    }

    fn leave(&mut self, _state: &mut Self::State) {
        self.recursion.inactive();
        self.samples.inactive();
    }
}

impl RenderSettingsView {
    pub(crate) fn init() -> FrameHandle<Self> {
        let main_color = palette::SHADE;
        let secondary_color = palette::NEUTRAL;
        let knob_color = palette::NEUTRAL_DARK;
        let mut recursion = Slider::new(
            Rectangle::new((PADDING, PADDING), (Self::WIDTH - 2 * PADDING, 200)),
            "Reflection depth (color)".to_owned(),
            (1..31).collect(),
            main_color,
            secondary_color,
            knob_color,
        );
        let mut samples = Slider::new(
            Rectangle::new(
                (PADDING, 2 * PADDING + 200),
                (Self::WIDTH - 2 * PADDING, 200),
            ),
            "Samples (smoothness)".to_owned(),
            (1..256).collect(),
            main_color,
            secondary_color,
            knob_color,
        );
        let init = RenderSettings::preset(0);
        recursion.set_value(&init.recursion);
        samples.set_value(&init.samples);
        let this = Self {
            preset_level: Some(0),
            recursion,
            samples,
        };
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
        let num_jobs = settings.proposed_num_jobs();
        let job = RenderTask::new(Main::area(), settings);
        let jobs = job.divide(num_jobs);
        paddle::send::<_, Main>(EnqueueNewRender(jobs));
    }

    /// paddle event listener
    fn render_done(&mut self, _: &mut (), _msg: &crate::RenderFinished) {
        if let Some(level) = &mut self.preset_level {
            *level += 1;
            let new = RenderSettings::preset(*level);
            self.samples.set_value(&new.samples);
            self.recursion.set_value(&new.recursion);
        }
    }

    pub(crate) fn render_settings(&mut self) -> RenderSettings {
        RenderSettings {
            resolution: (Main::WIDTH, Main::HEIGHT),
            samples: *self.samples.value(),
            recursion: *self.recursion.value(),
        }
    }
}

impl RenderSettings {
    fn proposed_num_jobs(&self) -> u32 {
        match self.recursion * self.samples {
            n if n < 4 => 16,
            n if n < 10 => 32,
            n if n < 50 => 64,
            n if n < 100 => 128,
            n if n < 200 => 256,
            n if n < 500 => 512,
            _ => 1024,
        }
    }

    fn preset(level: u32) -> Self {
        let samples;
        let recursion;
        match level {
            0 => {
                samples = 1;
                recursion = 2;
            }
            1 => {
                samples = 1;
                recursion = 2;
            }
            2 => {
                samples = 4;
                recursion = 4;
            }
            3 => {
                samples = 4;
                recursion = 6;
            }
            4 => {
                samples = 64;
                recursion = 8;
            }
            5 => {
                samples = 128;
                recursion = 12;
            }
            6 | _ => {
                samples = 255;
                recursion = 16;
            }
        }
        Self {
            resolution: (Main::WIDTH, Main::HEIGHT),
            samples,
            recursion,
        }
    }
}
