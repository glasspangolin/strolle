use glam::Vec3;

use crate::{gpu, BoundingBox};

#[derive(Clone, Copy, Debug)]
pub struct BvhTriangle {
    pub bb: BoundingBox,
    pub center: Vec3,
    pub triangle_id: gpu::TriangleId,
    pub material_id: gpu::MaterialId,
}