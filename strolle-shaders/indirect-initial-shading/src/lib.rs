#![no_std]

use strolle_gpu::prelude::*;

#[rustfmt::skip]
#[spirv(compute(threads(8, 8)))]
#[allow(clippy::too_many_arguments)]
pub fn main(
    #[spirv(global_invocation_id)]
    global_id: UVec3,
    #[spirv(local_invocation_index)]
    local_idx: u32,
    #[spirv(push_constant)]
    params: &IndirectInitialShadingPassParams,
    #[spirv(workgroup)]
    stack: BvhTraversingStack,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)]
    triangles: &[Triangle],
    #[spirv(descriptor_set = 0, binding = 1, storage_buffer)]
    bvh: &[BvhNode],
    #[spirv(descriptor_set = 0, binding = 2, storage_buffer)]
    lights: &[Light],
    #[spirv(descriptor_set = 0, binding = 3, storage_buffer)]
    materials: &[Material],
    #[spirv(descriptor_set = 0, binding = 4)]
    atlas_tex: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 0, binding = 5)]
    atlas_sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 6, uniform)]
    world: &World,
    #[spirv(descriptor_set = 1, binding = 0, uniform)]
    camera: &Camera,
    #[spirv(descriptor_set = 1, binding = 1)]
    atmosphere_transmittance_lut_tex: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 1, binding = 2)]
    atmosphere_transmittance_lut_sampler: &Sampler,
    #[spirv(descriptor_set = 1, binding = 3)]
    atmosphere_sky_lut_tex: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 1, binding = 4)]
    atmosphere_sky_lut_sampler: &Sampler,
    #[spirv(descriptor_set = 1, binding = 5)]
    direct_hits_d0: TexRgba32f,
    #[spirv(descriptor_set = 1, binding = 6)]
    direct_hits_d1: TexRgba32f,
    #[spirv(descriptor_set = 1, binding = 7)]
    indirect_hits_d0: TexRgba32f,
    #[spirv(descriptor_set = 1, binding = 8)]
    indirect_hits_d1: TexRgba32f,
    #[spirv(descriptor_set = 1, binding = 9, storage_buffer)]
    indirect_initial_samples: &mut [Vec4],
) {
    main_inner(
        global_id.xy(),
        local_idx,
        params,
        stack,
        TrianglesView::new(triangles),
        BvhView::new(bvh),
        LightsView::new(lights),
        MaterialsView::new(materials),
        Atmosphere::new(
            atmosphere_transmittance_lut_tex,
            atmosphere_transmittance_lut_sampler,
            atmosphere_sky_lut_tex,
            atmosphere_sky_lut_sampler,
        ),
        atlas_tex,
        atlas_sampler,
        world,
        camera,
        direct_hits_d0,
        direct_hits_d1,
        indirect_hits_d0,
        indirect_hits_d1,
        indirect_initial_samples,
    )
}

#[allow(clippy::too_many_arguments)]
fn main_inner(
    global_id: UVec2,
    local_idx: u32,
    params: &IndirectInitialShadingPassParams,
    stack: BvhTraversingStack,
    triangles: TrianglesView,
    bvh: BvhView,
    lights: LightsView,
    materials: MaterialsView,
    atmosphere: Atmosphere,
    atlas_tex: &Image!(2D, type=f32, sampled),
    atlas_sampler: &Sampler,
    world: &World,
    camera: &Camera,
    direct_hits_d0: TexRgba32f,
    direct_hits_d1: TexRgba32f,
    indirect_hits_d0: TexRgba32f,
    indirect_hits_d1: TexRgba32f,
    indirect_initial_samples: &mut [Vec4],
) {
    let mut noise = Noise::new(params.seed, global_id);
    let global_idx = camera.half_screen_to_idx(global_id);
    let screen_pos = upsample(global_id, params.frame);

    // -------------------------------------------------------------------------

    let direct_hit = Hit::deserialize(
        direct_hits_d0.read(screen_pos),
        direct_hits_d1.read(screen_pos),
    );

    if direct_hit.is_none() {
        unsafe {
            *indirect_initial_samples.get_unchecked_mut(3 * global_idx) =
                Default::default();

            *indirect_initial_samples.get_unchecked_mut(3 * global_idx + 1) =
                Default::default();

            *indirect_initial_samples.get_unchecked_mut(3 * global_idx + 2) =
                Default::default();
        }

        return;
    }

    let indirect_ray =
        Ray::new(direct_hit.point, noise.sample_hemisphere(direct_hit.normal));

    let indirect_hit = Hit::deserialize(
        indirect_hits_d0.read(global_id),
        indirect_hits_d1.read(global_id),
    );

    // -------------------------------------------------------------------------
    // Phase 1:
    //
    // Similarly as for direct lightning, let's start by selecting the best
    // light-candidate, judging lights by their *unshadowed* contribution.
    //
    // This algorithm follows a similar logic as direct initial shading, so
    // comments were skipped for brevity.

    let mut reservoir = DirectReservoir::default();

    if indirect_hit.is_some() {
        let material = materials.get(MaterialId::new(indirect_hit.material_id));

        let albedo = material
            .albedo(atlas_tex, atlas_sampler, indirect_hit.uv)
            .xyz();

        let mut light_idx = 0;

        while light_idx < world.light_count {
            let light_id = LightId::new(light_idx);

            let light_contribution = lights
                .get(light_id)
                .contribution(material, indirect_hit, indirect_ray, albedo)
                .sum();

            let sample = DirectReservoirSample {
                light_id,
                light_contribution,
            };

            reservoir.add(&mut noise, sample, sample.p_hat());
            light_idx += 1;
        }
    }

    let sky_weight = if reservoir.w_sum == 0.0 {
        1.0
    } else {
        0.25 * reservoir.w_sum
    };

    let mut sky_normal = Vec3::ZERO;

    if sky_weight > 0.0 {
        // If we hit nothing, we know that our indirect-hit's normal must point
        // towards the sky - great, let's use it!
        //
        // If we hit something, we don't know in which way we can sample the
        // sky, so just take a random guess on the hemisphere on our surface.
        sky_normal = if indirect_hit.is_none() {
            indirect_hit.normal
        } else {
            noise.sample_hemisphere(indirect_hit.normal)
        };

        // Cursed:
        //
        // Since we only support single-bounce GI, let's arbitrarily boost the
        // sky's exposure to compensate for the missing bounces.
        //
        // It's pretty so-so (and increases variance), but it helps a bit as
        // well.
        let sky_exposure = if indirect_hit.is_none() { 9.0 } else { 4.5 };

        let sky =
            sky_exposure * atmosphere.eval(world.sun_direction(), sky_normal);

        let sample = DirectReservoirSample::sky(sky);

        reservoir.add(&mut noise, sample, sample.p_hat());
    }

    // -------------------------------------------------------------------------
    // Phase 2:
    //
    // Select the best light-candidate and cast a shadow ray to check if that
    // light (which might be sun) is actually visible to us.

    let DirectReservoirSample {
        light_id,
        light_contribution,
    } = reservoir.sample;

    let light = if reservoir.sample.is_sky() {
        Light::sun(sky_normal * World::SUN_DISTANCE)
    } else {
        lights.get(light_id)
    };

    let light_visibility = light.visibility(
        local_idx,
        triangles,
        bvh,
        stack,
        &mut noise,
        indirect_hit,
    );

    let mut color = light_contribution * light_visibility;

    if reservoir.sample.is_sky() {
        // Cursed:
        //
        // Since we only support single-bounce GI, let's avoid getting the image
        // extra dark by skipping the cosine term of the rendering equation.
        //
        // psst don't tell anybody
    } else {
        color *= indirect_ray.direction().dot(direct_hit.normal);
    }

    // Setting a mininimum radiance is technically wrong, but at least this way
    // we don't have to deal with zero p_hats:
    let color = color.max(Vec3::splat(0.000001));

    let indirect_normal;
    let indirect_point;

    if indirect_hit.is_some() {
        indirect_normal = Normal::encode(indirect_hit.normal);
        indirect_point = indirect_hit.point;
    } else {
        indirect_normal = Normal::encode(-indirect_ray.direction());
        indirect_point = sky_normal * World::SUN_DISTANCE;
    }

    unsafe {
        *indirect_initial_samples.get_unchecked_mut(3 * global_idx) =
            color.extend(indirect_normal.x);

        *indirect_initial_samples.get_unchecked_mut(3 * global_idx + 1) =
            direct_hit.point.extend(indirect_normal.y);

        *indirect_initial_samples.get_unchecked_mut(3 * global_idx + 2) =
            indirect_point.extend(Default::default());
    }
}