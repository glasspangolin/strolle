#![no_std]

use spirv_std::glam::{vec2, UVec3, Vec3Swizzles, Vec4};
use spirv_std::{spirv, Image};
use strolle_models::*;

#[allow(clippy::too_many_arguments)]
#[spirv(compute(threads(8, 8)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(local_invocation_index)] local_idx: u32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)]
    geometry_tris: &[Vec4],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)]
    geometry_uvs: &[Vec4],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)]
    geometry_bvh: &[Vec4],
    #[spirv(uniform, descriptor_set = 0, binding = 3)] lights: &Lights,
    #[spirv(uniform, descriptor_set = 0, binding = 4)] materials: &Materials,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] camera: &Camera,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 1)] hits: &[u32],
    #[spirv(descriptor_set = 1, binding = 2)] image_tex: &Image!(2D, format=rgba16f, sampled=false),
    #[spirv(workgroup)] stack: RayTraversingStack,
) {
    let global_idx = id.y * camera.viewport_size().as_uvec2().x + id.x;

    let world = World {
        local_idx,
        geometry_tris: GeometryTrisView::new(geometry_tris),
        geometry_uvs: GeometryUvsView::new(geometry_uvs),
        geometry_bvh: GeometryBvhView::new(geometry_bvh),
        camera,
        lights,
        materials,
    };

    let hit = hits[global_idx as usize];

    let color = if hit == 0 {
        world.camera.clear_color().extend(1.0)
    } else {
        if debug::ENABLE_AABB {
            spirv_std::glam::Vec3::splat((hit as f32) / 100.0).extend(1.0)
        } else {
            let ray = world.camera.ray(vec2(id.x as f32, id.y as f32));

            let tri =
                world.geometry_tris.get(TriangleId::new((hit - 1) as usize));

            let hit = tri.hit(ray, Culling::Enabled);
            let mat = world.materials.get(hit.mat_id);

            mat.shade(&world, stack, hit)
        }
    };

    // TODO safety
    unsafe {
        image_tex.write(id.xy().as_ivec2(), color);
    }
}
