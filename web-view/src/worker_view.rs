use paddle::quicksilver_compat::{Color, Shape};
use paddle::{Frame, PointerEventType, Rectangle};

use crate::render::RenderTask;
use crate::worker::{self, PngRenderWorker, WorkerReady, WorkerResult};
use crate::{progress, PngPart};

const BACKGROUND: Color = Color::new(0.1, 0.1, 0.2);

pub(crate) struct WorkerView {
    buttons: Vec<Button>,
    workers: Vec<PngRenderWorker>,
    job_pool: Vec<RenderTask>,
}

#[derive(Clone, Copy)]
pub struct AddWorker {
    pub remote: bool,
}

impl WorkerView {
    pub fn new() -> Self {
        Self {
            buttons: vec![
                Button::new(
                    Rectangle::new((10, 10), (50, 50)),
                    worker::LOCAL_WORKER_COL,
                    AddWorker { remote: false },
                ),
                Button::new(
                    Rectangle::new((10, 65), (50, 50)),
                    worker::REMOTE_WORKER_COL,
                    AddWorker { remote: true },
                ),
            ],
            workers: vec![],
            job_pool: vec![],
        }
    }
}

impl Frame for WorkerView {
    type State = ();

    const WIDTH: u32 = 665;
    const HEIGHT: u32 = 320;

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        if let PointerEventType::PrimaryClick = event.event_type() {
            for button in &self.buttons {
                if event.pos().overlaps(&button.area) {
                    (button.trigger)()
                }
            }
        }
    }

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &BACKGROUND);

        for button in &self.buttons {
            canvas.draw(&button.area, &button.color)
        }

        for (i, worker) in self.workers.iter().enumerate() {
            let x = i % 3;
            let y = i / 3;
            let area = Rectangle::new((65 + x * 100, 5 + y * 100), (100, 100)).padded(3.0);
            worker.draw(canvas, area);
        }
    }

    fn update(&mut self, _state: &mut Self::State) {
        for worker in &mut self.workers {
            if worker.ready() && worker.current_task().is_none() {
                if let Some(job) = self.job_pool.pop() {
                    worker.accept_task(job);
                }
            }
        }
    }
}

impl WorkerView {
    /// paddle event listener
    pub fn add_worker(&mut self, _state: &mut (), msg: &AddWorker) {
        if msg.remote {
            self.workers.push(PngRenderWorker::new(
                self.workers.len(),
                Some("http://127.0.0.1:3000".to_owned()),
            ));
        } else {
            self.workers
                .push(PngRenderWorker::new(self.workers.len(), None));
        }
    }

    /// paddle event listener
    pub fn worker_ready(&mut self, _state: &mut (), WorkerReady(worker_id): WorkerReady) {
        self.workers[worker_id].set_ready(true);
    }

    /// paddle event listener
    pub fn new_jobs(&mut self, _state: &mut (), job_pool: Vec<RenderTask>) {
        self.job_pool = job_pool;
    }

    /// paddle event listener
    pub fn job_done(&mut self, _state: &mut (), WorkerResult { worker_id, img }: WorkerResult) {
        let (job, duration) = self.workers[worker_id]
            .clear_task()
            .expect("result must belong to a job");
        paddle::send::<_, crate::Main>(PngPart {
            img,
            screen_area: job.screen_area,
        });
        self.workers[worker_id].record_time(duration);
        paddle::send::<_, progress::RenderProgress>(progress::ProgressMade {
            worker_id: worker_id,
            time: duration,
        });

        self.workers[worker_id].set_ready(true);
    }
}

struct Button {
    area: Rectangle,
    color: Color,
    trigger: Box<dyn Fn()>,
}

impl Button {
    fn new<T: 'static + Clone>(area: Rectangle, color: Color, msg: T) -> Self {
        Self {
            area,
            color,
            trigger: Box::new(move || paddle::share(msg.clone())),
        }
    }
}
