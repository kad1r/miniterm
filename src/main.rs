mod render;
mod terminal;
mod text;

use render::atlas_gpu::GpuAtlas;
use render::grid_draw::{build_instances, CellView};
use render::renderer::Renderer;
use text::metrics::{measure, CellMetrics};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

const FONT_BYTES: &[u8] = include_bytes!("../assets/font/CascadiaMono.ttf");
const FONT_PX: f32 = 18.0;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("miniterm")
        .build(&event_loop)
        .unwrap();
    let mut renderer = Renderer::new(&window);

    // Build the GpuAtlas once (stored alongside renderer).
    let mut atlas = GpuAtlas::new(
        renderer.device(),
        renderer.atlas_bind_group_layout(),
        FONT_BYTES.to_vec(),
        FONT_PX,
    );

    // Measure cell metrics using the same font bytes.
    let metrics: CellMetrics = measure(FONT_BYTES, FONT_PX);

    event_loop
        .run(move |event, elwt| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => renderer.resize(size),
                    WindowEvent::RedrawRequested => {
                        // Temp block: render hard-coded "miniterm" string.
                        let text = "miniterm";
                        let cells: Vec<Vec<CellView>> = vec![text
                            .chars()
                            .map(|ch| CellView {
                                ch,
                                fg: [1.0, 1.0, 1.0],
                                bg: [0.05, 0.05, 0.06],
                            })
                            .collect()];

                        // Pre-resolve each char's UV from the atlas.
                        let queue = renderer.queue();
                        // Collect UVs into a map first (borrow of atlas ends before draw_quads).
                        let uv_map: std::collections::HashMap<char, ([f32; 2], [f32; 2])> =
                            text.chars()
                                .map(|ch| (ch, atlas.uv_for(queue, ch)))
                                .collect();

                        let (bg, glyphs) = build_instances(
                            &cells,
                            &metrics,
                            [20.0, 20.0],
                            &|ch| uv_map.get(&ch).copied().unwrap_or(([0.0, 0.0], [0.0, 0.0])),
                        );

                        renderer.draw_quads(&bg, &glyphs, &atlas);
                    }
                    _ => {}
                }
            }
        })
        .unwrap();
}
