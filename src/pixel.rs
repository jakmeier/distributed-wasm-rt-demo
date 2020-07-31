use nalgebra::Vector3;

#[derive(Clone, Copy)]
pub struct Pixel {
    pub col: Vector3<u8>,
}
impl Pixel {
    pub fn rgb_vec(v: Vector3<f32>) -> Self {
        Self {
            col: Vector3::new(to_u8(v.x), to_u8(v.y), to_u8(v.z)),
        }
    }
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            col: Vector3::new(to_u8(r), to_u8(g), to_u8(b)),
        }
    }
    // pub fn rgb_i(r: u8, g: u8, b: u8) -> Self {
    //     Self {
    //         col: Vector3::new(r, g, b),
    //     }
    // }
}

pub struct PixelPlane {
    pub w: usize,
    pub h: usize,
    pixels: Vec<Pixel>,
}
pub struct PixelPlaneShard {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pixels: Vec<Pixel>,
}
// pub struct PixelPlaneShard<'a> {
//     pub x: usize,
//     pub y: usize,
//     pub w: usize,
//     pub h: usize,
//     pixels: &'a mut [Pixel],
// }
// pub struct PackedPixelPlaneShard {
//     pub x: usize,
//     pub y: usize,
//     pub w: usize,
//     pub h: usize,
//     pixels: *const Pixel,
// }

impl PixelPlane {
    pub fn new(w: usize, h: usize) -> Self {
        let mut pixels = Vec::with_capacity(w * h);
        pixels.resize(w * h, Default::default());
        Self { w, h, pixels }
    }
    pub fn pixel(&self, x: usize, y: usize) -> Pixel {
        self.pixels[y * self.w + x]
    }
    pub fn set_pixel(&mut self, x: usize, y: usize, p: Pixel) {
        self.pixels[y * self.w + x] = p;
    }
    pub fn split_into_shards(&mut self, n: usize) -> Vec<PixelPlaneShard> {
        let mut shards = vec![];
        let w = self.w;
        let h = self.h / n;
        let mut pixels = self.pixels.as_mut_slice();
        for i in 0..(n - 1) {
            let (left, right) = pixels.split_at_mut(w * h);
            shards.push(PixelPlaneShard::new(0, i * h, w, h, left));
            pixels = right;
        }
        shards.push(PixelPlaneShard::new(0, (n - 1) * h, w, h, pixels));
        shards
    }
    pub fn collect_shards(&mut self, shards: &[PixelPlaneShard]) {
        let n = shards.len();
        let w = shards[0].w;
        let h = shards[0].h;
        let pixels_in_shard = w * h;
        let end = n * pixels_in_shard;
        for (i, shard) in shards.iter().enumerate() {
            debug_assert_eq!(pixels_in_shard, shard.w * shard.h);
            let b = end - i * pixels_in_shard;
            let a = b - pixels_in_shard;
            self.pixels[a..b].clone_from_slice(&shard.pixels);
        }
    }
    pub unsafe fn raw_data(&self) -> &[u8] {
        std::slice::from_raw_parts(
            &self.pixels[0] as *const Pixel as *const u8,
            self.w * self.h * 3,
        )
    }
}
impl PixelPlaneShard {
    pub fn new(x: usize, y: usize, w: usize, h: usize, pixels: &[Pixel]) -> Self {
        assert_eq!(w * h, pixels.len(), "Invalid shardr split");
        Self {
            x,
            y,
            w,
            h,
            pixels: pixels.iter().cloned().collect(),
        }
    }
    pub fn pixel(&self, x: usize, y: usize) -> Pixel {
        self.pixels[y * self.w + x]
    }
    pub fn set_pixel(&mut self, x: usize, y: usize, p: Pixel) {
        self.pixels[y * self.w + x] = p;
    }
}

fn to_u8(f: f32) -> u8 {
    // apprpximate gamma correction with square root
    (255.999 * f.sqrt()) as u8
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            col: Vector3::new(0, 0, 0),
        }
    }
}

// // Bad and unsafe
// impl<'a> Into<PackedPixelPlaneShard> for PixelPlaneShard<'a> {
//     fn into(self) -> PackedPixelPlaneShard {
//         // let pixels = std::slice::from_raw_parts_mut(pointer, pixels.len());
//         PackedPixelPlaneShard {
//             x: self.x,
//             y: self.y,
//             w: self.w,
//             h: self.h,
//             pixels: self.pixels.first_mut().unwrap() as *mut Pixel,
//         }
//     }
// }
// impl Into<PixelPlaneShard<'static>> for PackedPixelPlaneShard {
//     fn into(self) -> PixelPlaneShard<'static> {
//         PixelPlaneShard {
//             x: self.x,
//             y: self.y,
//             w: self.w,
//             h: self.h,
//             pixels: std::slice::from_raw_parts_mut(
//                 std::mem::transmute(self.pixels),
//                 self.w * self.h,
//             ),
//         }
//     }
// }
