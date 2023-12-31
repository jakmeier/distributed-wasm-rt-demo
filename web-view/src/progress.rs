use paddle::quicksilver_compat::{Circle, Color, Shape};
use paddle::{FloatingText, Frame, PointerEventType, Rectangle, Transform, UiElement};

use crate::{palette, PADDING, SCREEN_H};

const BACKGROUND: Color = palette::NEUTRAL_DARK;
const EMPTY: Color = palette::NEUTRAL;
const FULL: Color = palette::MAIN;

pub struct RenderProgress {
    total: usize,
    done: usize,
    bar_text: FloatingText,

    total_time: std::time::Duration,
    start: chrono::NaiveDateTime,
    sub_text: Vec<FloatingText>,

    stop_button: UiElement,
}

pub enum ProgressMade {
    /// Work has been performed by a worker controlled by this instance.
    Domestic {
        worker_id: usize,
        time: std::time::Duration,
    },
    /// Work has been performed by a remote peer.
    Foreign,
}

pub struct ProgressReset {
    pub work_items: usize,
}

impl Frame for RenderProgress {
    type State = ();

    const WIDTH: u32 = 620;
    const HEIGHT: u32 = SCREEN_H - crate::Main::HEIGHT - crate::Tabs::HEIGHT - 2 * PADDING;

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        if let PointerEventType::PrimaryClick = event.event_type() {
            let (area, transform) = self.progress_bar_pos();
            let hitbox = area.transformed_bounding_box(transform);
            if event.pos().overlaps(&hitbox) {
                paddle::share(crate::RequestNewRender);
            }
        }
        self.stop_button.pointer(event);
    }

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        let z = 1;
        canvas.draw(&Self::area(), &BACKGROUND);

        let done = self.total <= self.done;
        let progress = if self.total == 0 {
            1.0
        } else {
            self.done as f32 / self.total as f32
        };

        let (full_progress_bar, bar_pos) = self.progress_bar_pos();

        let circle = Circle::new((0, 50), 50);
        let left_col = if !done && self.done == 0 {
            &EMPTY
        } else {
            &FULL
        };
        let right_col = if done { &FULL } else { &EMPTY };
        canvas.draw_ex(&circle, left_col, bar_pos, z);
        canvas.draw_ex(
            &circle,
            right_col,
            Transform::translate((full_progress_bar.width(), 0)) * bar_pos,
            z,
        );
        canvas.draw_ex(&full_progress_bar, &EMPTY, bar_pos, z);
        // let progress_bar = full_progress_bar
        canvas.draw_ex(
            &full_progress_bar,
            &FULL,
            bar_pos * Transform::scale((progress, 1.0)),
            z,
        );

        #[allow(unused_assignments)]
        let mut tmp = String::new();
        let msg = if done {
            "Start"
        } else {
            tmp = format!("{:2.0}%", progress * 100.0);
            &tmp
        };
        self.bar_text
            .write(
                &canvas,
                &full_progress_bar.transformed_bounding_box(bar_pos),
                z,
                paddle::FitStrategy::Center,
                msg,
            )
            .unwrap();
        self.bar_text.draw();

        for (i, text) in self.sub_text.iter_mut().enumerate() {
            text.update_position(
                &canvas
                    .frame_to_display_area(Rectangle::new((10, 150 + i * 100), (Self::WIDTH, 100))),
                z,
            )
            .unwrap();
            text.draw();
        }

        self.stop_button.draw(canvas);
    }

    fn leave(&mut self, _state: &mut Self::State) {
        self.bar_text.hide().unwrap();
        for text in &self.sub_text {
            text.hide().unwrap();
        }
        self.stop_button.inactive();
    }

    fn enter(&mut self, _state: &mut Self::State) {
        self.bar_text.show().unwrap();
        for text in &self.sub_text {
            text.show().unwrap();
        }
        self.stop_button.active();
    }
}

impl RenderProgress {
    pub fn new() -> Self {
        let bar_text = FloatingText::new_styled(
            &Rectangle::default(),
            "".to_owned(),
            &[("color", palette::CSS_FONT_DARK), ("font-size", "x-large")],
            &[],
        )
        .unwrap();
        fn subtext() -> FloatingText {
            let mut text = FloatingText::new_styled(
                &Rectangle::default(),
                "".to_owned(),
                &[("color", palette::CSS_FONT_LIGHT), ("font-size", "large")],
                &[],
            )
            .unwrap();
            text.update_fit_strategy(paddle::FitStrategy::Center)
                .unwrap();
            text
        }

        let sub_text = vec![subtext(), subtext()];
        let mut stop_button = crate::button(
            Rectangle::new((10, Self::HEIGHT - 110), (Self::WIDTH - 20, 100)),
            palette::ACCENT,
            crate::Stop,
            "Stop".to_owned(),
            50.0,
        );
        stop_button.add_text_css("color", palette::CSS_FONT_DARK);
        stop_button.add_text_css("font-size", "x-large");
        Self {
            done: 0,
            total: 0,
            bar_text,
            start: Default::default(),
            sub_text,
            total_time: Default::default(),
            stop_button,
        }
    }

    fn progress_bar_pos(&self) -> (Rectangle, Transform) {
        let full_progress_bar = Rectangle::new_sized((Self::WIDTH - 120, 100));
        let bar_pos = Transform::translate((60, 20));
        (full_progress_bar, bar_pos)
    }

    /// paddle event listener
    pub fn progress_update(&mut self, _state: &mut (), msg: ProgressMade) {
        match msg {
            ProgressMade::Domestic { time, .. } => {
                self.total_time += time;
            }
            ProgressMade::Foreign => (),
        }
        self.done += 1;
        if self.done == self.total {
            let latency = paddle::utc_now()
                .signed_duration_since(self.start)
                .to_std()
                .unwrap();
            self.sub_text[0].update_text(&format!("Finished in {latency:#.1?}"));
            paddle::share(crate::RenderFinished);
        }
        self.sub_text[1].update_text(&format!("Total compute: {:#.1?}", self.total_time));
    }

    /// paddle event listener
    pub fn progress_reset(&mut self, _state: &mut (), msg: ProgressReset) {
        self.total = msg.work_items;
        self.done = 0;
        self.total_time = Default::default();
        self.start = paddle::utc_now();
        self.sub_text[0].update_text("");
        self.sub_text[1].update_text("");
    }

    /// paddle event listener
    pub(crate) fn stop(&mut self, _state: &mut (), _msg: &crate::Stop) {
        self.progress_reset(_state, ProgressReset { work_items: 0 });
    }
}
