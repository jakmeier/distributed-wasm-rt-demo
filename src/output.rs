use crate::PixelPlane;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

impl PixelPlane {
    pub fn export_ppm(&self, path: &Path) -> std::io::Result<()> {
        let mut buffer = BufWriter::new(File::create(path)?);
        self.write_ppm(&mut buffer)?;
        buffer.flush()?;
        Ok(())
    }
    pub fn export_png(&self, path: &Path) -> std::io::Result<()> {
        let buffer = BufWriter::new(File::create(path)?);
        self.write_png(buffer)?;
        Ok(())
    }
    pub fn write_png(&self, out: impl Write) -> Result<(), std::io::Error> {
        let mut encoder = png::Encoder::new(out, self.w as u32, self.h as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        Ok(unsafe {
            writer.write_image_data(&self.raw_data())?;
        })
    }
    fn write_ppm(&self, out: &mut impl Write) -> std::io::Result<()> {
        writeln!(out, "P3")?;
        writeln!(out, "{} {}", self.w, self.h)?;
        writeln!(out, "255")?;
        for y in 0..self.h {
            for x in 0..self.w {
                if x != 0 {
                    write!(out, "  ")?;
                }
                let col = self.pixel(x, y).col;
                write!(out, "{:>3} {:>3} {:>3}", col.x, col.y, col.z)?;
            }
            write!(out, "\n")?;
        }
        Ok(())
    }
}
