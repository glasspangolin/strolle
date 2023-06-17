use log::debug;
use spirv_std::glam::UVec2;

use crate::Bindable;

#[derive(Debug)]
pub struct Texture {
    tex: wgpu::Texture,
    format: wgpu::TextureFormat,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    filterable: bool,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        label: impl AsRef<str>,
        size: UVec2,
        format: wgpu::TextureFormat,
    ) -> Self {
        let label = label.as_ref();

        debug!("Allocating texture `{label}`; size={size:?}");

        assert!(size.x > 0);
        assert!(size.y > 0);

        let usage = if format == wgpu::TextureFormat::Depth32Float {
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
        } else {
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
        };

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}_tex")),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = tex.create_view(&Default::default());

        let xx = format!("{label}_sampler");

        let filterable = label.contains("atmosphere");

        // TODO
        let sampler = if filterable {
            wgpu::SamplerDescriptor {
                label: Some(&xx),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }
        } else {
            wgpu::SamplerDescriptor {
                label: Some(&xx),
                ..Default::default()
            }
        };

        let sampler = device.create_sampler(&sampler);

        Self {
            tex,
            format,
            view,
            sampler,
            filterable,
        }
    }

    pub fn tex(&self) -> &wgpu::Texture {
        &self.tex
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Creates an immutable texture+sampler binding:
    ///
    /// ```
    /// #[spirv(descriptor_set = ..., binding = ...)]
    /// tex: &Image!(2D, type=f32, sampled),
    ///
    /// #[spirv(descriptor_set = ..., binding = ...)]
    /// sampler: &Sampler,
    /// ```
    ///
    /// Sampler's binding follows the texture so e.g. if the texture has
    /// `binding = 3`, sampler will be `binding = 4`.
    pub fn bind_sampled(&self) -> impl Bindable + '_ {
        TextureSampledBinder { parent: self }
    }

    /// Creates an immutable storage-texture binding:
    ///
    /// ```
    /// #[spirv(descriptor_set = ..., binding = ...)]
    /// tex: &Image!(2D, format = ..., sampled = false),
    /// ```
    ///
    /// TODO naga and/or rust-gpu don't support read-only storage textures yet
    ///      so currently this is equivalent to a writable binding, just
    ///      separated for readability reasons
    pub fn bind_readable(&self) -> impl Bindable + '_ {
        TextureStorageBinder { parent: self }
    }

    /// Creates a mutable storage-texture binding:
    ///
    /// ```
    /// #[spirv(descriptor_set = ..., binding = ...)]
    /// tex: &Image!(2D, format = ..., sampled = false),
    /// ```
    pub fn bind_writable(&self) -> impl Bindable + '_ {
        TextureStorageBinder { parent: self }
    }
}

pub struct TextureSampledBinder<'a> {
    parent: &'a Texture,
}

impl Bindable for TextureSampledBinder<'_> {
    fn bind(
        &self,
        binding: u32,
    ) -> Vec<(wgpu::BindGroupLayoutEntry, wgpu::BindingResource)> {
        let tex_layout = wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float {
                    filterable: self.parent.filterable,
                },
            },
            count: None,
        };

        let sampler_layout = wgpu::BindGroupLayoutEntry {
            binding: binding + 1,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Sampler(if self.parent.filterable {
                wgpu::SamplerBindingType::Filtering
            } else {
                wgpu::SamplerBindingType::NonFiltering
            }),
            count: None,
        };

        let tex_resource =
            wgpu::BindingResource::TextureView(&self.parent.view);

        let sampler_resource =
            wgpu::BindingResource::Sampler(&self.parent.sampler);

        vec![
            (tex_layout, tex_resource),
            (sampler_layout, sampler_resource),
        ]
    }
}

pub struct TextureStorageBinder<'a> {
    parent: &'a Texture,
}

impl Bindable for TextureStorageBinder<'_> {
    fn bind(
        &self,
        binding: u32,
    ) -> Vec<(wgpu::BindGroupLayoutEntry, wgpu::BindingResource)> {
        let tex_layout = wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::ReadWrite,
                format: self.parent.format,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };

        let tex_resource =
            wgpu::BindingResource::TextureView(&self.parent.view);

        vec![(tex_layout, tex_resource)]
    }
}
