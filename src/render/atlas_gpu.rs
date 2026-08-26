use crate::render::grid_draw::GlyphInfo;
use crate::text::atlas::{GlyphKey, GlyphRect, ShelfPacker};
use std::collections::HashMap;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::FontRef;

pub const ATLAS_SIZE: u32 = 1024;

pub struct GpuAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    packer: ShelfPacker,
    uvs: HashMap<GlyphKey, GlyphInfo>,
    font: Vec<u8>,
    px: f32,
    scale_cx: ScaleContext,
}

impl GpuAtlas {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        font: Vec<u8>,
        px: f32,
    ) -> GpuAtlas {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        GpuAtlas {
            texture,
            view,
            sampler,
            bind_group,
            packer: ShelfPacker::new(ATLAS_SIZE),
            uvs: HashMap::new(),
            font,
            px,
            scale_cx: ScaleContext::new(),
        }
    }

    pub fn uv_for(
        &mut self,
        queue: &wgpu::Queue,
        ch: char,
    ) -> GlyphInfo {
        let key = GlyphKey { ch, bold: false, italic: false };
        if let Some(info) = self.uvs.get(&key) {
            return *info;
        }
        let font = FontRef::from_index(&self.font, 0).unwrap();
        let glyph_id = font.charmap().map(ch);
        let mut scaler = self
            .scale_cx
            .builder(font)
            .size(self.px)
            .hint(true)
            .build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id);

        // Some glyphs (zero-width joiners, combining marks, whitespace) rasterize
        // to a Some(image) with a 0-sized placement and empty data. Writing a
        // 1x1 extent with 0 bytes of data panics in wgpu, so treat any empty
        // placement as a blank 1x1 cell.
        let (w, h, left, top, data) = match image {
            Some(img) if img.placement.width > 0 && img.placement.height > 0 => (
                img.placement.width,
                img.placement.height,
                img.placement.left,
                img.placement.top,
                img.data,
            ),
            _ => (1, 1, 0, 0, vec![0u8]),
        };

        // Guard against the shelf packer overflowing the atlas height: writing
        // past the texture bounds would also panic in wgpu. Once full, new
        // glyphs render blank rather than crashing the app.
        if self.packer.would_overflow(w, h, ATLAS_SIZE) {
            let info = GlyphInfo {
                uv_min: [0.0, 0.0],
                uv_max: [0.0, 0.0],
                px_size: [0.0, 0.0],
                offset: [0.0, 0.0],
            };
            self.uvs.insert(key, info);
            return info;
        }
        let rect: GlyphRect = self.packer.insert(w, h);
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: rect.x, y: rect.y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let f = ATLAS_SIZE as f32;
        let info = GlyphInfo {
            uv_min: [rect.x as f32 / f, rect.y as f32 / f],
            uv_max: [(rect.x + w) as f32 / f, (rect.y + h) as f32 / f],
            px_size: [w as f32, h as f32],
            offset: [left as f32, top as f32],
        };
        self.uvs.insert(key, info);
        info
    }
}
