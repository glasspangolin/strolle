#![no_std]

use spirv_std::glam::{
    vec2, UVec2, UVec3, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles,
};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use spirv_std::spirv;
use strolle_gpu::*;

#[rustfmt::skip]
#[spirv(compute(threads(8, 8)))]
#[allow(clippy::too_many_arguments)]
pub fn main(
    #[spirv(global_invocation_id)]
    global_id: UVec3,
    #[spirv(push_constant)]
    params: &IndirectTemporalResamplingPassParams,
    #[spirv(descriptor_set = 0, binding = 0, uniform)]
    camera: &Camera,
    #[spirv(descriptor_set = 0, binding = 1)]
    geometry_map: TexRgba32f,
    #[spirv(descriptor_set = 0, binding = 2)]
    past_geometry_map: TexRgba32f,
    #[spirv(descriptor_set = 0, binding = 3)]
    reprojection_map: TexRgba32f,
    #[spirv(descriptor_set = 0, binding = 4, storage_buffer)]
    indirect_initial_samples: &[Vec4],
    #[spirv(descriptor_set = 0, binding = 5, storage_buffer)]
    indirect_temporal_reservoirs: &mut [Vec4],
    #[spirv(descriptor_set = 0, binding = 6, storage_buffer)]
    past_indirect_temporal_reservoirs: &[Vec4],
) {
    main_inner(
        global_id.xy(),
        params,
        camera,
        GeometryMap::new(geometry_map),
        GeometryMap::new(past_geometry_map),
        ReprojectionMap::new(reprojection_map),
        indirect_initial_samples,
        indirect_temporal_reservoirs,
        past_indirect_temporal_reservoirs,
    )
}

#[allow(clippy::too_many_arguments)]
fn main_inner(
    global_id: UVec2,
    params: &IndirectTemporalResamplingPassParams,
    camera: &Camera,
    geometry_map: GeometryMap,
    past_geometry_map: GeometryMap,
    reprojection_map: ReprojectionMap,
    indirect_initial_samples: &[Vec4],
    indirect_temporal_reservoirs: &mut [Vec4],
    past_indirect_temporal_reservoirs: &[Vec4],
) {
    let mut noise = Noise::new(params.seed, global_id);
    let global_idx = camera.half_screen_to_idx(global_id);

    let sample = {
        let d0 = indirect_initial_samples[3 * global_idx];
        let d1 = indirect_initial_samples[3 * global_idx + 1];
        let d2 = indirect_initial_samples[3 * global_idx + 2];

        // Setting a mininimum radiance is technically wrong but at least we
        // won't have to deal with negative p_hat later:
        let radiance = d0.xyz().max(Vec3::splat(0.0001));

        IndirectReservoirSample {
            radiance,
            hit_point: d1.xyz(),
            sample_point: d2.xyz(),
            sample_normal: Normal::decode(vec2(d0.w, d1.w)),
        }
    };

    let mut p_hat = sample.p_hat();
    let mut reservoir = IndirectReservoir::new(sample, p_hat, params.frame);

    // -------------------------------------------------------------------------

    let reprojection =
        reprojection_map.get(upsample(global_id, params.frame - 1));

    if reprojection.is_valid() {
        // Where our reservoir was located in the previous frame
        let from_screen_pos =
            upsample(reprojection.past_screen_pos() / 2, params.frame - 1);

        // Where our reservoir is going to be located in the current frame
        let to_screen_pos = upsample(global_id, params.frame);

        // Now, because we're going to use our past reservoir's sample and kinda
        // "migrate" it into a new screen position, we've got an important
        // factor to consider:
        //
        // What if our past reservoir's surface is different from our to-be
        // reservoir's surface?
        //
        // For instance, if our past-reservoir is tracking a background object,
        // we can't suddently reproject it into a foreground pixel because that
        // would cause the light to bleed.
        //
        // Usually this is solved by relying solely on the reprojection map, but
        // because we're rendering reservoirs at half-res (and our reprojection
        // map is full-res), we have to additionally check if our reprojected
        // pixel's reservoir is "reprojectable" here.
        //
        // In particular:
        //
        // - score of 1.0 means that probably the camera is stationary and we're
        //   just reprojecting exactly the same reservoir into exactly the same
        //   pixel,
        //
        // - score of 0.0 means that we'd try to reproject a totally different
        //   reservoir into current pixel, so let's better not.
        let migration_compatibility = past_geometry_map
            .get(from_screen_pos)
            .evaluate_similarity_to(geometry_map.get(to_screen_pos));

        let mut past_reservoir = IndirectReservoir::read(
            past_indirect_temporal_reservoirs,
            camera.half_screen_to_idx(from_screen_pos / 2),
        );

        let past_p_hat = past_reservoir.sample.p_hat();

        // Older reservoirs are worse because they represent older state of the
        // world - and so if we're dealing with an older reservoir, let's reduce
        // its score:
        let past_age = past_reservoir.age(params.frame);

        if past_age > 16 {
            past_reservoir.m_sum *= 1.0 - ((16 - past_age) as f32 / 32.0);
        }

        past_reservoir.m_sum *= reprojection.confidence.powi(2);
        past_reservoir.m_sum *= migration_compatibility;

        if reservoir.merge(&mut noise, &past_reservoir, past_p_hat) {
            p_hat = past_p_hat;
            reservoir.frame = past_reservoir.frame;
        }
    }

    // -------------------------------------------------------------------------

    reservoir.normalize(p_hat, 10.0, 30.0);
    reservoir.write(indirect_temporal_reservoirs, global_idx);
}
