use paddle::quicksilver_compat::Color;
use paddle::{Frame, ImageDesc, Rectangle, UiElement};

use crate::progress::RenderProgress;
use crate::render::RenderTask;
use crate::worker::{
    self, PngRenderWorker, WorkerReady, WorkerResult, LOCAL_WORKER_COL, REMOTE_WORKER_COL,
};
use crate::{button, progress, PngPart, SCREEN_W};

const BACKGROUND: Color = Color::new(0.1, 0.1, 0.2);

pub(crate) struct WorkerView {
    buttons: Vec<UiElement>,
    workers: Vec<PngRenderWorker>,
    job_pool: Vec<RenderTask>,
    fermyon_img: ImageDesc,
}

#[derive(Clone, Copy)]
pub enum AddWorker {
    InBrowser,
    Localhost,
    Fermyon,
    // TODO: Any URL
}

impl WorkerView {
    pub fn new(fermyon_img: ImageDesc) -> Self {
        Self {
            buttons: vec![
                button(
                    Rectangle::new((10, 10), (50, 50)),
                    worker::LOCAL_WORKER_COL,
                    AddWorker::InBrowser,
                    "local".to_owned(),
                ),
                button(
                    Rectangle::new((10, 65), (50, 50)),
                    worker::REMOTE_WORKER_COL,
                    AddWorker::Localhost,
                    "remote".to_owned(),
                ),
                button(
                    Rectangle::new((10, 120), (50, 50)),
                    Color::from_rgba(100, 100, 100, 1.0),
                    AddWorker::Fermyon,
                    "fermyon".to_owned(),
                ),
            ],
            workers: vec![],
            job_pool: vec![],
            fermyon_img,
        }
    }
}

impl Frame for WorkerView {
    type State = ();

    const WIDTH: u32 = SCREEN_W - RenderProgress::WIDTH - 5;
    const HEIGHT: u32 = RenderProgress::HEIGHT;

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        for button in &self.buttons {
            button.pointer(event);
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
            button.draw(canvas);
        }

        for (i, worker) in self.workers.iter().enumerate() {
            let x = i / 5;
            let y = i % 5;
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
        match msg {
            AddWorker::InBrowser => {
                self.workers.push(PngRenderWorker::new(
                    self.workers.len(),
                    None,
                    Box::new(LOCAL_WORKER_COL),
                ));
            }
            AddWorker::Localhost => {
                self.workers.push(PngRenderWorker::new(
                    self.workers.len(),
                    Some("http://127.0.0.1:3000".to_owned()),
                    Box::new(REMOTE_WORKER_COL),
                ));
            }
            AddWorker::Fermyon => {
                self.workers.push(PngRenderWorker::new(
                    self.workers.len(),
                    Some("http://jakmeier-clumsy-rt-demo.fermyon.app".to_owned()),
                    Box::new(self.fermyon_img),
                ));
            }
        }
    }

    /// paddle event listener
    pub fn worker_ready(&mut self, _state: &mut (), WorkerReady(worker_id): WorkerReady) {
        self.workers[worker_id].set_ready(true);
    }

    /// paddle event listener
    pub fn new_jobs(&mut self, _state: &mut (), job_pool: Vec<RenderTask>) {
        self.job_pool = job_pool;
        self.workers.iter_mut().for_each(PngRenderWorker::clear);
    }

    /// paddle event listener
    pub fn job_done(&mut self, _state: &mut (), WorkerResult { worker_id, img }: WorkerResult) {
        if self.workers[worker_id].clear_interrupt() {
            self.workers[worker_id].set_ready(true);
            self.workers[worker_id].clear();
            return;
        }
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

    /// paddle event listener
    pub fn stop(&mut self, _state: &mut (), _msg: &crate::Stop) {
        self.job_pool.clear();
        self.workers.iter_mut().for_each(PngRenderWorker::interrupt);
    }
}
