use super::*;

#[derive(ugli::Vertex, Debug)]
struct Vertex {
    a_pos: Vec2<f32>,
    a_vt: Vec2<f32>,
}

pub struct Font {
    font: rusttype::Font<'static>,
    cache: RefCell<rusttype::gpu_cache::Cache<'static>>,
    cache_texture: RefCell<ugli::Texture>,
    geometry: RefCell<ugli::VertexBuffer<Vertex>>,
    program: ugli::Program,
    descent: f32,
}

const CACHE_SIZE: usize = 1024;

impl Font {
    pub fn new(geng: &Rc<Geng>, data: Vec<u8>) -> Result<Font, anyhow::Error> {
        Self::new_with(geng.ugli(), geng.shader_lib(), data)
    }
    pub(crate) fn new_with(
        geng: &Rc<Ugli>,
        shader_lib: &ShaderLib,
        data: Vec<u8>,
    ) -> Result<Font, anyhow::Error> {
        let font = rusttype::Font::try_from_vec(data).ok_or(anyhow!("Failed to read font"))?;
        let descent = font.v_metrics(rusttype::Scale { x: 1.0, y: 1.0 }).descent;
        Ok(Font {
            font,
            cache: RefCell::new(
                rusttype::gpu_cache::Cache::builder()
                    .dimensions(CACHE_SIZE as u32, CACHE_SIZE as u32)
                    .scale_tolerance(0.1)
                    .position_tolerance(0.1)
                    .build(),
            ),
            cache_texture: RefCell::new(ugli::Texture2d::new_uninitialized(
                geng,
                vec2(CACHE_SIZE, CACHE_SIZE),
            )),
            geometry: RefCell::new(ugli::VertexBuffer::new_dynamic(geng, Vec::new())),
            program: shader_lib.compile(include_str!("shader.glsl")).unwrap(),
            descent,
        })
    }
    pub fn measure_at(&self, text: &str, mut pos: Vec2<f32>, size: f32) -> AABB<f32> {
        pos.y -= self.descent * size;
        let scale = rusttype::Scale { x: size, y: size };
        let pos = rusttype::Point {
            x: pos.x,
            y: -pos.y,
        };
        let mut result: Option<AABB<f32>> = None;
        for glyph in self.font.layout(text, scale, pos) {
            if let Some(bb) = glyph.pixel_bounding_box() {
                if let Some(cur) = result {
                    result = Some(AABB::from_corners(
                        vec2(
                            partial_min(bb.min.x as f32, cur.x_min),
                            partial_min(bb.min.y as f32, cur.y_min),
                        ),
                        vec2(
                            partial_max(bb.max.x as f32, cur.x_max),
                            partial_max(bb.max.y as f32, cur.y_max),
                        ),
                    ));
                } else {
                    result = Some(AABB::from_corners(
                        vec2(bb.min.x as f32, bb.min.y as f32),
                        vec2(bb.max.x as f32, bb.max.y as f32),
                    ));
                }
            }
        }
        let mut result = result.unwrap_or(AABB {
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
        });
        let (bottom, top) = (-result.y_max, -result.y_min);
        result.y_min = bottom;
        result.y_max = top;
        result
    }
    pub fn measure(&self, text: &str, size: f32) -> AABB<f32> {
        self.measure_at(text, vec2(0.0, 0.0), size)
    }
    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        text: &str,
        mut pos: Vec2<f32>,
        size: f32,
        color: Color<f32>,
    ) {
        pos.y -= self.descent * size;
        let scale = rusttype::Scale { x: size, y: size };
        let pos = rusttype::Point {
            x: pos.x,
            y: framebuffer.size().y as f32 - pos.y,
        };

        let mut cache = self.cache.borrow_mut();
        let mut cache_texture = self.cache_texture.borrow_mut();

        // Workaround for https://gitlab.redox-os.org/redox-os/rusttype/-/merge_requests/158
        let glyphs: Vec<_> = text
            .chars()
            .map(|c| self.font.glyph(c))
            .scan((None, 0.0), |(last, x), g| {
                let g = g.scaled(scale);
                if let Some(last) = last {
                    *x += self.font.pair_kerning(scale, *last, g.id());
                }
                let w = g.h_metrics().advance_width;
                let next = g.positioned(pos + rusttype::vector(*x, 0.0));
                *last = Some(next.id());
                *x += w;
                Some(next)
            })
            .collect();
        for glyph in &glyphs {
            cache.queue_glyph(0, glyph.clone());
        }

        cache
            .cache_queued(|rect, data| {
                let x = rect.min.x as usize;
                let y = rect.min.y as usize;
                let width = rect.width() as usize;
                let height = rect.height() as usize;

                let mut rgba_data = vec![0xff; data.len() * 4];
                for i in 0..data.len() {
                    rgba_data[i * 4 + 3] = data[i];
                }

                unsafe {
                    cache_texture.sub_image(vec2(x, y), vec2(width, height), &rgba_data);
                }
            })
            .unwrap();

        let mut geometry = self.geometry.borrow_mut();
        geometry.clear();
        for glyph in &glyphs {
            if let Some((texture_rect, rect)) = cache.rect_for(0, glyph).unwrap() {
                let x1 = rect.min.x as f32;
                let y1 = rect.min.y as f32;
                let x2 = rect.max.x as f32;
                let y2 = rect.max.y as f32;
                let u1 = texture_rect.min.x;
                let u2 = texture_rect.max.x;
                let v1 = texture_rect.min.y;
                let v2 = texture_rect.max.y;
                geometry.push(Vertex {
                    a_pos: vec2(x1, y1),
                    a_vt: vec2(u1, v1),
                });
                geometry.push(Vertex {
                    a_pos: vec2(x2, y1),
                    a_vt: vec2(u2, v1),
                });
                geometry.push(Vertex {
                    a_pos: vec2(x2, y2),
                    a_vt: vec2(u2, v2),
                });

                geometry.push(Vertex {
                    a_pos: vec2(x1, y1),
                    a_vt: vec2(u1, v1),
                });
                geometry.push(Vertex {
                    a_pos: vec2(x2, y2),
                    a_vt: vec2(u2, v2),
                });
                geometry.push(Vertex {
                    a_pos: vec2(x1, y2),
                    a_vt: vec2(u1, v2),
                });
            }
        }

        let framebuffer_size = framebuffer.size();

        ugli::draw(
            framebuffer,
            &self.program,
            ugli::DrawMode::Triangles,
            &*geometry,
            ugli::uniforms! {
                u_color: color,
                u_cache_texture: &*cache_texture,
                u_framebuffer_size: framebuffer_size,
            },
            ugli::DrawParameters {
                depth_func: None,
                blend_mode: Some(ugli::BlendMode::Alpha),
                ..default()
            },
        );
    }
    pub fn draw_aligned(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        text: &str,
        pos: Vec2<f32>,
        align: f32,
        size: f32,
        color: Color<f32>,
    ) {
        self.draw(
            framebuffer,
            text,
            vec2(pos.x - self.measure(text, size).width() * align, pos.y),
            size,
            color,
        );
    }
}
