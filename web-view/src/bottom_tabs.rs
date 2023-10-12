use paddle::nuts::UncheckedActivityId;
use paddle::quicksilver_compat::Color;
use paddle::{Frame, FrameHandle, ImageDesc, Rectangle, UiElement};

use crate::images::Images;
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
        images: &Images,
    ) -> FrameHandle<Self> {
        let home_button = tab_button(images.home, 0);
        let network_button = tab_button(images.web, 1);
        let settings_button = tab_button(images.settings, 2);
        let stats_button = tab_button(images.stats, 3);
        let data = Self {
            buttons: vec![
                home_button.0,
                home_button.1,
                network_button.0,
                network_button.1,
                settings_button.0,
                settings_button.1,
                stats_button.0,
                stats_button.1,
            ],
            active_tab: 0,
            tab_activities: vec![
                vec![
                    main_handle.activity().into(),
                    progress_handle.activity().into(),
                    worker_handle.activity().into(),
                ],
                vec![
                    main_handle.activity().into(),
                    network_handle.activity().into(),
                ],
                vec![main_handle.activity().into()],
                vec![main_handle.activity().into()],
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

fn tab_button(img: ImageDesc, index: usize) -> (UiElement, UiElement) {
    let area = Rectangle::new((10 + 150 * index, 2), (140, 140));
    let background = UiElement::new(area.padded(10.0), img).with_z(1);
    let image = UiElement::new(area, BUTTON_COLOR.with_alpha(0.5))
        .with_rounded_corners(15.0)
        .with_pointer_interaction(
            paddle::PointerEventType::PrimaryClick,
            SwitchTabMsg { index },
        )
        .with_z(0);

    (background, image)
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
