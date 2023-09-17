use paddle::quicksilver_compat::{Circle, Color, Shape};
use paddle::{FloatingText, Frame, PointerEventType, Rectangle, Transform, UiElement};

use crate::SCREEN_H;

const BACKGROUND: Color = Color::new(0.1, 0.1, 0.2);
const EMPTY: Color = Color::new(0.0, 0.0, 0.1);
const FULL: Color = Color::new(0.4, 0.4, 0.7);

pub struct RenderProgress {
    total: usize,
    done: usize,
    bar_text: FloatingText,

    total_time: std::time::Duration,
    start: chrono::NaiveDateTime,
    sub_text: Vec<FloatingText>,

    stop_button: UiElement,
}

pub struct ProgressMade {
    pub worker_id: usize,
    pub time: std::time::Duration,
}

pub struct ProgressReset {
    pub work_items: usize,
}

impl Frame for RenderProgress {
    type State = ();

    const WIDTH: u32 = 620;
    const HEIGHT: u32 = SCREEN_H - crate::Main::HEIGHT - 5;

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        if let PointerEventType::PrimaryClick = event.event_type() {
            let (area, transform) = self.progress_bar_pos();
            let hitbox = area.transformed_bounding_box(transform);
            if event.pos().overlaps(&hitbox) {
                paddle::send::<_, crate::Main>(crate::RequestNewRender);
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

        let done = self.total == self.done;
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
            "> render <"
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
        self.stop_button.active();
    }
}

impl RenderProgress {
    pub fn new() -> Self {
        let bar_text = FloatingText::new_styled(
            &Rectangle::default(),
            "".to_owned(),
            &[("color", "white"), ("font-size", "x-large")],
            &[],
        )
        .unwrap();
        fn subtext() -> FloatingText {
            FloatingText::new_styled(
                &Rectangle::default(),
                "".to_owned(),
                &[("color", "white"), ("font-size", "large")],
                &[],
            )
            .unwrap()
        }

        let sub_text = vec![subtext(), subtext()];
        Self {
            done: 0,
            total: 0,
            bar_text,
            start: Default::default(),
            sub_text,
            total_time: Default::default(),
            stop_button: crate::button(
                Rectangle::new((10, Self::HEIGHT - 110), (Self::WIDTH - 20, 100)),
                Color::RED,
                crate::Stop,
                "stop".to_owned(),
            ),
        }
    }

    fn progress_bar_pos(&self) -> (Rectangle, Transform) {
        let full_progress_bar = Rectangle::new_sized((Self::WIDTH - 120, 100));
        let bar_pos = Transform::translate((60, 20));
        (full_progress_bar, bar_pos)
    }

    /// paddle event listener
    pub fn progress_update(&mut self, _state: &mut (), msg: ProgressMade) {
        self.total_time += msg.time;
        self.done += 1;
        if self.done == self.total {
            let latency = paddle::utc_now()
                .signed_duration_since(self.start)
                .to_std()
                .unwrap();
            self.sub_text[0].update_text(&format!("Finished in {latency:#.1?}"));
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
