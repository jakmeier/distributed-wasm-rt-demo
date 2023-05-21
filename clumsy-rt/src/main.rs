use clumsy_rt::*;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let n_threads: usize = std::env::var("N_THREADS")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(12);

    let n_samples: usize = std::env::var("N_SAMPLES")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(9);
    let size_scalar: usize = std::env::var("SIZE_SCALAR")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(120);
    let n_recursion: usize = std::env::var("N_RECURSION")
        .map(|s| s.parse::<usize>().expect("invalid value"))
        .unwrap_or(50);

    // let scene = clumsy_rt::sample_scenes::build_simple_scene();
    let scene = clumsy_rt::sample_scenes::build_cool_scene();
    let camera = Camera::new(n_samples, n_recursion);

    let w = 4 * size_scalar;
    let h = 3 * size_scalar;
    let mut img = PixelPlane::new(w, h);

    println!("{}x{}", w, h);
    println!("{}x multi-sampling", n_samples);
    println!("{}x ray-bouncing", n_recursion);

    camera.render(scene, &mut img, n_threads);

    img.export_png(Path::new("out.png"))?;
    // img.export_ppm(Path::new("out.ppm"))?;

    Ok(())
}

#[test]
fn produce_test_img() -> std::io::Result<()> {
    const W: usize = 256;
    const H: usize = 256;

    let mut img = PixelPlane::new(W, H);
    for y in (0..H).rev() {
        for x in 0..W {
            let r = x as f32 / (W - 1) as f32;
            let g = (H - y - 1) as f32 / (H - 1) as f32;
            let b = 0.25;
            img.set_pixel(x, y, Pixel::rgb(r, g, b));
        }
    }
    img.export_ppm(Path::new("test.ppm"))?;
    img.export_png(Path::new("test.png"))?;
    Ok(())
}
