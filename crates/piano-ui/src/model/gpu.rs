//! The 3D view drawn by the GPU, inside egui's own OpenGL painter (an
//! [`egui_glow::CallbackFn`]): the scene into an offscreen multisampled
//! target with a depth buffer, lit by the same light as the plan, then
//! copied into the view's rectangle. Its GL objects are made on the first
//! frame, on the painter's context. Needs OpenGL 3.3 or GLES 3.0; on
//! anything else, or if a step fails, it reports why and the page falls
//! back to egui's drawing. The realistic look ([`super::real`]) is drawn
//! by the same callback when asked for; if it fails, the plain one is.

use std::sync::{Arc, Mutex};

use egui_glow::glow::{self, HasContext};

use super::cam::{Camera, Mat4};
use super::paint::LIGHT;
use super::real::Real;
use super::scene::{Scene, Vertex};

#[derive(Clone, Default)]
pub struct Gpu {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    res: Option<Resources>,
    failed: Option<String>,
    real: Option<Real>,
    real_failed: Option<String>,
}

struct Resources {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    /// What `vbo` holds: tris, lines, glass and lid counts per layer.
    runs: Runs<(i32, i32, i32)>,
    u_mvp: Option<glow::UniformLocation>,
    u_light: Option<glow::UniformLocation>,
    u_lit: Option<glow::UniformLocation>,
    target: Option<Target>,
    samples: i32,
}

/// The offscreen target, as large as the view in pixels.
struct Target {
    fbo: glow::Framebuffer,
    color: glow::Renderbuffer,
    depth: glow::Renderbuffer,
    size: (i32, i32),
}

const VERTEX: &str = r#"
in vec3 a_pos;
in vec3 a_normal;
in vec4 a_color;
uniform mat4 u_mvp;
uniform vec3 u_light;
uniform float u_lit;
out vec4 v_color;
void main() {
    gl_Position = u_mvp * vec4(a_pos, 1.0);
    float d = abs(dot(normalize(a_normal), normalize(u_light)));
    v_color = vec4(a_color.rgb * mix(1.0, 0.55 + 0.45 * d, u_lit), a_color.a);
}
"#;

const FRAGMENT: &str = r#"
precision mediump float;
in vec4 v_color;
out vec4 frag;
void main() {
    frag = v_color;
}
"#;

impl Gpu {
    /// Why the GPU view does not work here, once it was tried.
    pub fn failed(&self) -> Option<String> {
        self.state.lock().ok()?.failed.clone()
    }

    /// Why the realistic look does not work here, once it was tried.
    pub fn real_failed(&self) -> Option<String> {
        self.state.lock().ok()?.real_failed.clone()
    }

    /// Queues the scene's drawing into `rect` (filled with `background`),
    /// in the realistic look through `real`'s camera if given.
    pub fn paint(&self, ui: &egui::Ui, rect: egui::Rect, scene: [Arc<Scene>; 3], mvp: Mat4, background: egui::Color32, real: Option<Camera>) {
        let state = self.state.clone();
        let bg = egui::Rgba::from(background);
        let callback = egui_glow::CallbackFn::new(move |info, painter| {
            let Ok(mut st) = state.lock() else { return };
            if st.failed.is_some() {
                return;
            }
            let gl = painter.gl();
            if let Some(cam) = real
                && st.real_failed.is_none()
            {
                if st.real.is_none() {
                    match unsafe { Real::new(gl) } {
                        Ok(r) => st.real = Some(r),
                        Err(e) => st.real_failed = Some(e),
                    }
                }
                if let Some(r) = st.real.as_mut() {
                    match unsafe { r.draw(gl, &info, painter.intermediate_fbo(), &scene, &cam) } {
                        Ok(()) => return,
                        Err(e) => st.real_failed = Some(e),
                    }
                }
            }
            if st.res.is_none() {
                match unsafe { Resources::new(gl) } {
                    Ok(r) => st.res = Some(r),
                    Err(e) => {
                        st.failed = Some(e);
                        return;
                    }
                }
            }
            let res = st.res.as_mut().unwrap();
            if let Err(e) = unsafe { res.draw(gl, &info, painter.intermediate_fbo(), &scene, &mvp, [bg.r(), bg.g(), bg.b()]) } {
                st.failed = Some(e);
            }
        });
        ui.painter().add(egui::PaintCallback { rect, callback: Arc::new(callback) });
    }
}

/// The scene's layers ([`super::scene::Layer::PARTS`]) in one vertex
/// buffer, one run of vertices each, in their order: each sent to the GPU
/// only when it is another scene than the one it holds, so that the
/// still layer goes once and the others as often as they change. `M` is
/// what the draws need to know of a run.
pub(super) struct Runs<M> {
    /// Per layer: the scene, its run's first vertex and length, what to
    /// know of it.
    held: Vec<(Arc<Scene>, i32, i32, M)>,
    /// The buffer's size (bytes).
    cap: usize,
}

impl<M> Default for Runs<M> {
    fn default() -> Self {
        Self { held: Vec::new(), cap: 0 }
    }
}

impl<M: Copy> Runs<M> {
    /// Brings the bound array buffer up to `layers`, `run` giving a
    /// layer's vertices (of `T`, plain data) and what to know of them;
    /// returns each run's first vertex and that.
    pub(super) unsafe fn upload<T: Copy>(&mut self, gl: &glow::Context, layers: &[Arc<Scene>], run: impl Fn(&Scene) -> (Vec<T>, M)) -> Vec<(i32, M)> {
        let size = std::mem::size_of::<T>();
        // SAFETY: the vertices are repr(C) f32 records with no padding.
        let as_bytes = |v: &[T]| unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v)) };
        let changed = (0..layers.len()).find(|&i| self.held.get(i).is_none_or(|h| !Arc::ptr_eq(&h.0, &layers[i])));
        if let Some(k) = changed {
            // From the first layer changed on (those after it move).
            let fresh: Vec<_> = layers[k..].iter().map(|s| run(s)).collect();
            let from = self.held.get(k.wrapping_sub(1)).map_or(0, |h| (h.1 + h.2) as usize);
            let end = from + fresh.iter().map(|f| f.0.len()).sum::<usize>();
            self.held.truncate(k);
            unsafe {
                let mut at = from;
                let runs = if k > 0 && end * size <= self.cap {
                    fresh
                } else {
                    // All of it again, with room for what moves to grow
                    // (keys pressed, strings drawn in segments as they
                    // sound).
                    let mut all: Vec<_> = layers[..k].iter().map(|s| run(s)).collect();
                    all.extend(fresh);
                    let first = all[0].0.len();
                    let total: usize = all.iter().map(|f| f.0.len()).sum();
                    self.cap = (first + 2 * (total - first).max(1)) * size;
                    gl.buffer_data_size(glow::ARRAY_BUFFER, self.cap as i32, glow::DYNAMIC_DRAW);
                    self.held.clear();
                    at = 0;
                    all
                };
                for ((v, m), s) in runs.into_iter().zip(&layers[self.held.len()..]) {
                    gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, (at * size) as i32, as_bytes(&v));
                    self.held.push((s.clone(), at as i32, v.len() as i32, m));
                    at += v.len();
                }
            }
        }
        self.held.iter().map(|h| (h.1, h.3)).collect()
    }
}

impl Resources {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let version = egui_glow::ShaderVersion::get(gl);
        if !version.is_new_shader_interface() {
            return Err(format!("the 3D view needs OpenGL 3.3 or GLES 3.0 (this context offers {version:?})"));
        }
        unsafe {
            let program = gl.create_program()?;
            let mut shaders = Vec::new();
            for (kind, src) in [(glow::VERTEX_SHADER, VERTEX), (glow::FRAGMENT_SHADER, FRAGMENT)] {
                let s = gl.create_shader(kind)?;
                gl.shader_source(s, &format!("{}\n{src}", version.version_declaration()));
                gl.compile_shader(s);
                if !gl.get_shader_compile_status(s) {
                    return Err(format!("3D view shader: {}", gl.get_shader_info_log(s)));
                }
                gl.attach_shader(program, s);
                shaders.push(s);
            }
            for (i, name) in ["a_pos", "a_normal", "a_color"].into_iter().enumerate() {
                gl.bind_attrib_location(program, i as u32, name);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(format!("3D view shaders: {}", gl.get_program_info_log(program)));
            }
            for s in shaders {
                gl.detach_shader(program, s);
                gl.delete_shader(s);
            }
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = std::mem::size_of::<Vertex>() as i32;
            for (i, (size, offset)) in [(3, 0), (3, 12), (4, 24)].into_iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32);
                gl.vertex_attrib_pointer_f32(i as u32, size, glow::FLOAT, false, stride, offset);
            }
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let samples = gl.get_parameter_i32(glow::MAX_SAMPLES).clamp(1, 8);
            Ok(Self {
                u_mvp: gl.get_uniform_location(program, "u_mvp"),
                u_light: gl.get_uniform_location(program, "u_light"),
                u_lit: gl.get_uniform_location(program, "u_lit"),
                program,
                vao,
                vbo,
                runs: Runs::default(),
                target: None,
                samples,
            })
        }
    }

    unsafe fn target(&mut self, gl: &glow::Context, size: (i32, i32)) -> Result<glow::Framebuffer, String> {
        if let Some(t) = &self.target
            && t.size == size
        {
            return Ok(t.fbo);
        }
        unsafe {
            if let Some(t) = self.target.take() {
                gl.delete_framebuffer(t.fbo);
                gl.delete_renderbuffer(t.color);
                gl.delete_renderbuffer(t.depth);
            }
            let fbo = gl.create_framebuffer()?;
            let color = gl.create_renderbuffer()?;
            let depth = gl.create_renderbuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            for (rb, format, attachment) in [(color, glow::RGBA8, glow::COLOR_ATTACHMENT0), (depth, glow::DEPTH_COMPONENT24, glow::DEPTH_ATTACHMENT)] {
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
                gl.renderbuffer_storage_multisample(glow::RENDERBUFFER, self.samples, format, size.0, size.1);
                gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, attachment, glow::RENDERBUFFER, Some(rb));
            }
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            self.target = Some(Target { fbo, color, depth, size });
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!("3D view: offscreen target incomplete (0x{status:x})"));
            }
            Ok(fbo)
        }
    }

    unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        info: &egui::PaintCallbackInfo,
        out: Option<glow::Framebuffer>,
        scene: &[Arc<Scene>],
        mvp: &Mat4,
        bg: [f32; 3],
    ) -> Result<(), String> {
        let vp = info.viewport_in_pixels();
        let clip = info.clip_rect_in_pixels();
        let size = (vp.width_px.max(1), vp.height_px.max(1));
        unsafe {
            // An error left by an earlier drawing is not ours.
            for _ in 0..8 {
                if gl.get_error() == glow::NO_ERROR {
                    break;
                }
            }
            let fbo = self.target(gl, size)?;
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.viewport(0, 0, size.0, size.1);
            gl.clear_color(bg[0], bg[1], bg[2], 1.0);
            gl.depth_mask(true);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.u_mvp.as_ref(), false, &mvp.0);
            gl.uniform_3_f32(self.u_light.as_ref(), LIGHT[0], LIGHT[1], LIGHT[2]);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let runs = self.runs.upload(gl, scene, |s| {
                let all = [&s.tris[..], &s.lines, &s.glass, &s.lid].concat();
                (all, (s.tris.len() as i32, s.lines.len() as i32, (s.glass.len() + s.lid.len()) as i32))
            });
            gl.uniform_1_f32(self.u_lit.as_ref(), 1.0);
            for &(first, (nt, _, _)) in &runs {
                gl.draw_arrays(glow::TRIANGLES, first, nt);
            }
            gl.uniform_1_f32(self.u_lit.as_ref(), 0.0);
            for &(first, (nt, nl, _)) in &runs {
                gl.draw_arrays(glow::LINES, first + nt, nl);
            }
            // The see-through parts over the rest, not hiding each other.
            gl.enable(glow::BLEND);
            // The target's alpha stays 1: copied into a window surface that
            // has an alpha channel, less would let the compositor show what
            // lies behind the window through the frame and the lid.
            gl.blend_func_separate(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA, glow::ZERO, glow::ONE);
            gl.depth_mask(false);
            gl.uniform_1_f32(self.u_lit.as_ref(), 1.0);
            for &(first, (nt, nl, ng)) in &runs {
                gl.draw_arrays(glow::TRIANGLES, first + nt + nl, ng);
            }
            gl.depth_mask(true);
            gl.disable(glow::DEPTH_TEST);
            // Into egui's target, clipped as egui clips the view.
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, out);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            gl.blit_framebuffer(
                0,
                0,
                size.0,
                size.1,
                vp.left_px,
                vp.from_bottom_px,
                vp.left_px + size.0,
                vp.from_bottom_px + size.1,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, out);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let err = gl.get_error();
            if err != glow::NO_ERROR {
                return Err(format!("3D view: OpenGL error 0x{err:x}"));
            }
        }
        Ok(())
    }
}
