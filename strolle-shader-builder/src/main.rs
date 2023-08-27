use std::env;
use std::error::Error;
use std::path::Path;

use spirv_builder::{Capability, SpirvBuilder};

const CRATES: &[&str] = &[
    "atmosphere",
    "bvh-heatmap",
    "direct-denoising",
    "direct-initial-shading",
    "direct-raster",
    "direct-resolving",
    "direct-spatial-resampling",
    "direct-temporal-resampling",
    "frame-composition",
    "frame-reprojection",
    "indirect-diffuse-denoising",
    "indirect-diffuse-resolving",
    "indirect-diffuse-spatial-resampling",
    "indirect-diffuse-temporal-resampling",
    "indirect-initial-shading",
    "indirect-initial-tracing",
    "indirect-specular-denoising",
    "indirect-specular-resampling",
    "indirect-specular-resolving",
    "reference-shading",
    "reference-tracing",
];

fn main() -> Result<(), Box<dyn Error>> {
    for crate_name in CRATES {
        let crate_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("strolle-shaders")
            .join(crate_name);

        SpirvBuilder::new(crate_path, "spirv-unknown-spv1.3")
            .capability(Capability::Int8)
            .release(true)
            .build()?;
    }

    Ok(())
}
