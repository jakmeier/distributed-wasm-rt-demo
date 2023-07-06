use paddle::{Frame, Rectangle};

use crate::Main;

#[derive(Debug)]
pub struct RenderTask {
    pub screen_area: paddle::Rectangle,
    settings: RenderSettings,
}

#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub resolution: (u32, u32),
    pub samples: u32,
    pub recursion: u32,
}

impl RenderTask {
    pub fn new(screen_area: paddle::Rectangle, settings: RenderSettings) -> Self {
        Self {
            screen_area,
            settings,
        }
    }

    pub(crate) fn marshal(&self) -> api::RenderJob {
        let rx = self.settings.resolution.0 as f32 / Main::WIDTH as f32;
        let ry = self.settings.resolution.1 as f32 / Main::HEIGHT as f32;
        api::RenderJob::new(
            (self.screen_area.x() * rx).round() as u32,
            (self.screen_area.y() * ry).round() as u32,
            (self.screen_area.width() * rx).round() as u32,
            (self.screen_area.height() * ry).round() as u32,
            self.settings.resolution.0,
            self.settings.resolution.1,
            self.settings.samples,
            self.settings.recursion,
        )
    }

    pub fn divide(&self, num_tasks: u32) -> Vec<Self> {
        let width = self.screen_area.width();
        let height = self.screen_area.height();

        let num_columns = (num_tasks as f32).sqrt().ceil() as u32;
        let num_rows = (num_tasks as f32 / num_columns as f32).ceil() as u32;

        let task_width = width / num_columns as f32;
        let task_height = height / num_rows as f32;

        let mut tasks = Vec::new();

        for row in 0..num_rows {
            for col in 0..num_columns {
                let x = col as f32 * task_width;
                let y = row as f32 * task_height;
                let w = task_width.min(width - x);
                let h = task_height.min(height - y);

                tasks.push(RenderTask {
                    screen_area: Rectangle::new((x, y), (w, h)),
                    settings: self.settings.clone(),
                });
            }
        }

        tasks
    }
}
