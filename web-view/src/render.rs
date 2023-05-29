use js_sys::Uint32Array;
use paddle::Rectangle;
use web_sys::Worker;

use crate::{SCREEN_H, SCREEN_W};

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

    pub fn submit_to_worker(&self, worker: &Worker) {
        let msg = self.marshal();
        worker
            .post_message(&msg)
            .expect("Failed posting job to worker");
    }

    fn marshal(&self) -> Uint32Array {
        let rx = self.settings.resolution.0 as f32 / SCREEN_W as f32;
        let ry = self.settings.resolution.1 as f32 / SCREEN_H as f32;
        let job = api::RenderJob::new(
            (self.screen_area.x() * rx) as u32,
            (self.screen_area.y() * ry) as u32,
            (self.screen_area.width() * rx) as u32,
            (self.screen_area.height() * ry) as u32,
            self.settings.resolution.0,
            self.settings.resolution.1,
            self.settings.samples,
            self.settings.recursion,
        );

        let vec = job.as_vec();
        let array = Uint32Array::new_with_length(vec.len() as u32);
        array.copy_from(&vec);
        array
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
                let width = task_width.min(width - x);
                let height = task_height.min(height - y);

                tasks.push(RenderTask {
                    screen_area: Rectangle::new((x, y), (width, height)),
                    settings: self.settings.clone(),
                });
            }
        }

        tasks
    }
}
