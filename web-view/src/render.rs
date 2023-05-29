use js_sys::Uint32Array;
use paddle::Rectangle;
use web_sys::Worker;

#[derive(Debug)]
pub struct RenderTask {
    pub area: paddle::Rectangle,
    settings: RenderSettings,
}

#[derive(Debug, Clone)]
struct RenderSettings {
    resolution: (u32, u32),
    samples: u32,
    recursion: u32,
}

impl RenderTask {
    pub fn new(area: paddle::Rectangle) -> Self {
        Self {
            area,
            settings: RenderSettings {
                resolution: (960, 720),
                samples: 4,
                recursion: 8,
            },
        }
    }

    pub fn submit_to_worker(&self, worker: &Worker) {
        let msg = self.marshal();
        worker
            .post_message(&msg)
            .expect("Failed posting job to worker");
    }

    fn marshal(&self) -> Uint32Array {
        let job = api::RenderJob::new(
            self.area.x() as u32,
            self.area.y() as u32,
            self.area.width() as u32,
            self.area.height() as u32,
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
        let width = self.area.width() as u32;
        let height = self.area.height() as u32;

        let num_columns = (num_tasks as f32).sqrt().ceil() as u32;
        let num_rows = (num_tasks as f32 / num_columns as f32).ceil() as u32;

        let task_width = width / num_columns;
        let task_height = height / num_rows;

        let mut tasks = Vec::new();

        for row in 0..num_rows {
            for col in 0..num_columns {
                let x = col * task_width;
                let y = row * task_height;
                let width = task_width.min(width - x);
                let height = task_height.min(height - y);

                tasks.push(RenderTask {
                    area: Rectangle::new((x, y), (width, height)),
                    settings: self.settings.clone(),
                });
            }
        }

        tasks
    }
}
