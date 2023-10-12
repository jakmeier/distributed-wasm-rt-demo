use paddle::nuts::UncheckedActivityId;
use paddle::quicksilver_compat::Color;
use paddle::{Frame, FrameHandle, Rectangle, UiElement};

use crate::{palette, SCREEN_H, SCREEN_W};

const BUTTON_COLOR: Color = palette::SHADE;
const BACKGROUND_COLOR: Color = palette::MAIN;

/// Shows the buttons to switch tabs.
pub(crate) struct Tabs {
    buttons: Vec<UiElement>,
    active_tab: usize,
    tab_activities: Vec<Vec<UncheckedActivityId>>,
}

#[derive(Clone)]
struct SwitchTabMsg {
    index: usize,
}

impl Tabs {
    pub(crate) fn init(
        main_handle: FrameHandle<crate::Main>,
        progress_handle: FrameHandle<crate::RenderProgress>,
        worker_handle: FrameHandle<crate::WorkerView>,
        network_handle: FrameHandle<crate::NetworkView>,
    ) -> FrameHandle<Self> {
        let home_button = tab_button("Home", 0);
        let network_button = tab_button("Network", 1);
        let data = Self {
            buttons: vec![home_button, network_button],
            active_tab: 0,
            tab_activities: vec![
                vec![
                    main_handle.activity().into(),
                    progress_handle.activity().into(),
                    worker_handle.activity().into(),
                ],
                vec![network_handle.activity().into()],
            ],
        };
        let handle = paddle::register_frame_no_state(data, (0, SCREEN_H - Self::HEIGHT));
        handle.listen(Self::switch_tab);
        handle
    }

    fn switch_tab(&mut self, _state: &mut (), SwitchTabMsg { index }: &SwitchTabMsg) {
        for activity in &self.tab_activities[self.active_tab] {
            activity.set_status(paddle::nuts::LifecycleStatus::Inactive);
        }
        self.active_tab = *index;
        for activity in &self.tab_activities[self.active_tab] {
            activity.set_status(paddle::nuts::LifecycleStatus::Active);
        }
    }
}

fn tab_button(text: &'static str, index: usize) -> UiElement {
    let network = UiElement::new(
        Rectangle::new((2 + 150 * index, 2), (146, 146)),
        BUTTON_COLOR,
    )
    .with_text(text.to_string())
    .unwrap()
    .with_rounded_corners(15.0)
    .with_pointer_interaction(
        paddle::PointerEventType::PrimaryClick,
        SwitchTabMsg { index },
    );
    network
}

impl Frame for Tabs {
    type State = ();

    const WIDTH: u32 = SCREEN_W;
    const HEIGHT: u32 = 150;

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &BACKGROUND_COLOR);
        for button in &self.buttons {
            button.draw(canvas);
        }
    }

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        for button in &self.buttons {
            button.pointer(event);
        }
    }
}
