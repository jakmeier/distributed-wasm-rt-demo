use std::fmt::Display;
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug)]
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

    pub fn to_vec(&self) -> Vec<u32> {
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

    pub fn try_from_slice(data: &[u32]) -> Result<RenderJob, RenderJobParseError> {
        const EXPECTED_LEN: usize = 8;
        (data.len() == EXPECTED_LEN)
            .then(|| {
                Self::new(
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                )
            })
            .ok_or_else(|| RenderJobParseError::IncorrectLength {
                expected: EXPECTED_LEN,
                actual: data.len(),
            })
    }
}

impl Display for RenderJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut vec = self.to_vec();
        let mut nums = vec.drain(..);
        write!(f, "{}", nums.next().unwrap())?;
        for num in nums {
            write!(f, ",{num}")?;
        }
        Ok(())
    }
}

impl FromStr for RenderJob {
    type Err = RenderJobParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec: Vec<u32> = s
            .split(',')
            .map(u32::from_str)
            .collect::<Result<_, ParseIntError>>()?;

        Self::try_from_slice(&vec)
    }
}

#[derive(Error, Debug)]
pub enum RenderJobParseError {
    #[error("could not parse integer")]
    InvalidInt(#[from] ParseIntError),
    #[error("job contains wrong amount of numbers, expected {expected} but was {actual}")]
    IncorrectLength { expected: usize, actual: usize },
}
