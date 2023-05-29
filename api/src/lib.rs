pub struct RenderJob {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub camera_w: u32,
    pub camera_h: u32,
    pub n_samples: u32,
    pub n_recursion: u32,
}

impl RenderJob {
    pub fn new(
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        camera_w: u32,
        camera_h: u32,
        n_samples: u32,
        n_recursion: u32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            camera_w,
            camera_h,
            n_samples,
            n_recursion,
        }
    }

    pub fn as_vec(&self) -> Vec<u32> {
        vec![
            self.x,
            self.y,
            self.w,
            self.h,
            self.camera_w,
            self.camera_h,
            self.n_samples,
            self.n_recursion,
        ]
    }

    pub fn try_from_slice(data: &[u32]) -> Option<RenderJob> {
        (data.len() == 8).then(|| {
            Self::new(
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            )
        })
    }
}
