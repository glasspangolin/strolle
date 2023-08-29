use crate::{
    Camera, CameraBuffers, CameraComputePass, CameraController, Engine, Params,
};

#[derive(Debug)]
pub struct IndirectSpecularResolvingPass {
    pass: CameraComputePass<()>,
}

impl IndirectSpecularResolvingPass {
    pub fn new<P>(
        engine: &Engine<P>,
        device: &wgpu::Device,
        _: &Camera,
        buffers: &CameraBuffers,
    ) -> Self
    where
        P: Params,
    {
        let pass = CameraComputePass::builder("indirect_specular_resolving")
            .bind([
                &buffers.camera.bind_readable(),
                &buffers.direct_gbuffer_d0.bind_readable(),
                &buffers.direct_gbuffer_d1.bind_readable(),
                &buffers.indirect_specular_reservoirs.curr().bind_readable(),
                &buffers.indirect_specular_samples.bind_writable(),
            ])
            .build(device, &engine.shaders.indirect_specular_resolving);

        Self { pass }
    }

    pub fn run(
        &self,
        camera: &CameraController,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // This pass uses 8x8 warps:
        let size = camera.camera.viewport.size / 8;

        self.pass.run(camera, encoder, size, &());
    }
}