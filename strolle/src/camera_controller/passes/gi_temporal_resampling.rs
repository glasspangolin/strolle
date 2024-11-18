use crate::{
    Camera, CameraBuffers, CameraComputePass, CameraController, Engine, Params,
};
use crate::utils::ToGpu;

#[derive(Debug)]
pub struct GiTemporalResamplingPass {
    pass: CameraComputePass,
}

impl GiTemporalResamplingPass {
    pub fn new<P>(
        engine: &Engine<P>,
        device: &wgpu::Device,
        _: &Camera,
        buffers: &CameraBuffers,
    ) -> Self
    where
        P: Params,
    {
        let pass = CameraComputePass::builder("gi_temporal_resampling")
            .bind([
                &buffers.curr_camera.bind_readable(),
                &buffers.prev_camera.bind_readable(),
                &buffers.prim_gbuffer_d0.curr().bind_readable(),
                &buffers.prim_gbuffer_d1.curr().bind_readable(),
                &buffers.prim_gbuffer_d0.prev().bind_readable(),
                &buffers.prim_gbuffer_d1.prev().bind_readable(),
                &buffers.reprojection_map.bind_readable(),
                &buffers.gi_reservoirs[2].bind_readable(),
                &buffers.gi_reservoirs[1].bind_writable(),
            ])
            .build(device, &engine.shaders.gi_temporal_resampling);

        Self { pass }
    }

    pub fn run(
        &self,
        camera: &CameraController,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // This pass uses 8x8 warps:
        let size = (camera.camera.viewport.size + 7) / 8;

        self.pass.run(camera, encoder, size.to_gpu(), camera.pass_params());
    }
}
