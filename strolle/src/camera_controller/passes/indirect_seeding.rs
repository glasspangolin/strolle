use rand::Rng;

use crate::{
    gpu, Camera, CameraBuffers, CameraComputePass, CameraController, Engine,
    Params,
};

#[derive(Debug)]
pub struct IndirectSeedingPass {
    pass: CameraComputePass<gpu::IndirectPassParams>,
}

impl IndirectSeedingPass {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P>(
        engine: &Engine<P>,
        device: &wgpu::Device,
        _: &Camera,
        buffers: &CameraBuffers,
    ) -> Self
    where
        P: Params,
    {
        let pass = CameraComputePass::builder("indirect_seeding")
            .bind([
                &engine.triangles.bind_readable(),
                &engine.bvh.bind_readable(),
                &engine.lights.bind_readable(),
                &engine.materials.bind_readable(),
                &engine.images.bind_atlas(),
                &engine.world.bind_readable(),
            ])
            .bind([
                &buffers.camera.bind_readable(),
                &buffers.atmosphere_transmittance_lut.bind_sampled(),
                &buffers.atmosphere_sky_lut.bind_sampled(),
                &buffers.direct_gbuffer_d0.bind_readable(),
                &buffers.direct_gbuffer_d1.bind_readable(),
                &buffers.indirect_rays.bind_readable(),
                &buffers.indirect_gbuffer_d0.bind_readable(),
                &buffers.indirect_gbuffer_d1.bind_readable(),
                &buffers.indirect_samples.bind_writable(),
            ])
            .build(device, &engine.shaders.indirect_seeding);

        Self { pass }
    }

    pub fn run(
        &self,
        camera: &CameraController,
        encoder: &mut wgpu::CommandEncoder,
        mode: u32,
    ) {
        // This pass uses 8x8 warps:
        let size = (camera.camera.viewport.size + 7) / 8;

        let params = gpu::IndirectPassParams {
            seed: rand::thread_rng().gen(),
            frame: camera.frame,
            mode,
        };

        self.pass.run(camera, encoder, size, params);
    }
}