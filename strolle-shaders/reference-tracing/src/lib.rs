#![no_std]

use strolle_gpu::prelude::*;

#[spirv(compute(threads(8, 8)))]
#[allow(clippy::too_many_arguments)]
pub fn main(
    #[spirv(global_invocation_id)] global_id: UVec3,
    #[spirv(local_invocation_index)] local_idx: u32,
    #[spirv(push_constant)] params: &ReferencePassParams,
    #[spirv(workgroup)] stack: BvhStack,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)]
    triangles: &[Triangle],
    #[spirv(descriptor_set = 0, binding = 1, storage_buffer)] bvh: &[Vec4],
    #[spirv(descriptor_set = 0, binding = 2, storage_buffer)]
    materials: &[Material],
    #[spirv(descriptor_set = 0, binding = 3)] atlas_tex: Tex,
    #[spirv(descriptor_set = 0, binding = 4)] atlas_sampler: &Sampler,
    #[spirv(descriptor_set = 1, binding = 0, uniform)] camera: &Camera,
    #[spirv(descriptor_set = 1, binding = 1, storage_buffer)]
    reference_rays: &[Vec4],
    #[spirv(descriptor_set = 1, binding = 2, storage_buffer)]
    reference_hits: &mut [Vec4],
) {
    let screen_pos = global_id.xy();
    let screen_idx = camera.screen_to_idx(screen_pos);
    let triangles = TrianglesView::new(triangles);
    let bvh = BvhView::new(bvh);
    let materials = MaterialsView::new(materials);

    let ray = if params.depth == 0 {
        camera.ray(screen_pos)
    } else {
        let d0 = reference_rays[3 * screen_idx + 0];
        let d1 = reference_rays[3 * screen_idx + 1];

        if d1 == Default::default() {
            return;
        }

        Ray::new(d0.xyz(), d1.xyz())
    };

    let (hit, _) = ray.trace(
        local_idx,
        stack,
        triangles,
        bvh,
        materials,
        atlas_tex,
        atlas_sampler,
    );

    let [hit_d0, hit_d1] = hit.pack();

    reference_hits[2 * screen_idx + 0] = hit_d0;
    reference_hits[2 * screen_idx + 1] = hit_d1;
}