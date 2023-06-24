mod direct;
mod indirect;

pub use self::direct::*;
pub use self::indirect::*;
use crate::Noise;

/// Reservoir for sampling using ReSTIR.
///
/// https://benedikt-bitterli.me/restir/bitterli20restir.pdf
/// https://d1qx31qr3h6wln.cloudfront.net/publications/ReSTIR%20GI.pdf
#[derive(Clone, Copy, Default)]
pub struct Reservoir<T> {
    /// Selected sample; might contain light id, its contribution etc.
    pub sample: T,

    /// Sum of the weights of seen samples.
    pub w_sum: f32,

    /// Number of seen samples¹.
    ///
    /// It's capped to a certain limit, depending on the reservoir's kind, over
    /// the temporal and spatial resampling passes.
    ///
    /// ¹ so technically kinda-sorta u32, but using f32 allows for convenient
    ///   things like `m_sum *= 0.25;`
    pub m_sum: f32,

    /// Re-weighting factor, following the ReSTIR paper.
    ///
    /// It's capped to a certain limit, depending on the reservoir's kind, over
    /// the temporal and spatial resampling passes.
    pub w: f32,
}

impl<T> Reservoir<T>
where
    T: Clone + Copy,
{
    pub fn new(sample: T, weight: f32) -> Self {
        Self {
            sample,
            w_sum: weight,
            w: 1.0,
            m_sum: if weight == 0.0 { 0.0 } else { 1.0 },
        }
    }

    pub fn add(&mut self, noise: &mut Noise, s_new: T, w_new: f32) -> bool {
        self.w_sum += w_new;
        self.m_sum += 1.0;

        if noise.sample() <= w_new / self.w_sum {
            self.sample = s_new;
            true
        } else {
            false
        }
    }

    pub fn merge(&mut self, noise: &mut Noise, rhs: &Self, p_hat: f32) -> bool {
        if rhs.m_sum <= 0.0 {
            return false;
        }

        self.m_sum += rhs.m_sum - 1.0;
        self.add(noise, rhs.sample, rhs.w * rhs.m_sum * p_hat)
    }

    pub fn normalize(&mut self, p_hat: f32, max_w: f32, max_m_sum: f32) {
        self.w = self.w_sum / (self.m_sum * p_hat).max(0.001);
        self.w = self.w.min(max_w);
        self.m_sum = self.m_sum.min(max_m_sum);
    }
}