use paddle::Frame;
use paddle::quicksilver_compat::Color;

use crate::{SCREEN_H, SCREEN_W};

/// Shown once the instance has turned into a worker node.
pub(crate) struct WorkerNodeView {}

impl Frame for WorkerNodeView {
    type State = ();

    const WIDTH: u32 = SCREEN_W - 5;
    const HEIGHT: u32 = SCREEN_H - 5;

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &Color::INDIGO);
    }
}

/// Prompt that asks if this instance should be turned into worker node.
pub(crate) struct WorkerNodePrompt {}

impl Frame for WorkerNodePrompt {
    type State = ();

    const WIDTH: u32 = SCREEN_W - 50;
    const HEIGHT: u32 = SCREEN_H - 50;


    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &Color::ORANGE);
    }
}

