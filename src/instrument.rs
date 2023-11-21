use crate::audio_context::*;
use std::collections::VecDeque;
use std::time::{Instant, Duration};
use minvect::*;
use glow_mesh::xyzrgba::*;
use glow::HasContext;
use glutin::event::{Event, WindowEvent, ElementState, MouseButton};
use std::f32::consts::PI;

// was my fft backwards? see vecdeque shit
// audio sync: defintiely needs to be in the buffer like pick ms, 5ms. 220 0s at the start
// 880 samples i guess- 20ms latency, dont drop any frames on the vsync bro. maybe want to run that shit at higher framerate. anyway
// dont want gpu to block or whatever


/// this is just main struct in disguise: make it main

pub struct ControlSquare {
    p: Vec2,
    r: Rect,
}

impl ControlSquare {
    pub fn new(initial: Vec2, r: Rect) -> Self {
        ControlSquare { p: initial, r }
    }
    /// may update if p is within r
    pub fn update(&mut self, p: Vec2) {
        if self.r.contains(p) {
            self.p = (p - self.r.xy) / self.r.wh
        }
    }
}

/// Instrument
/// responsibilities:
///     output render geometry
///     handle inputs
///     send samples
/// maybe gets updated with t and/or dt. or use performance counter or something.
pub struct Instrument {
    xres: i32,
    yres: i32,
    window: glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>,
    gl: glow::Context,

    prog: ProgramXYZRGBA,

    mouse_pos: Vec2,
    mouse_lmb_held: bool,

    cx: UIThreadContext,
    samples_generated: usize,
    rb: VecDeque<f32>,
    last_sent: Instant,

    play: bool,

    // audio params = comb depth, comb delay
    // saw frequency and octaves

    // lets do fm 
    // carrier amplitude and volume
    // fm depth and frequency relation
    // fm phase shift
    // delay

    phase: f32,
    freq_vol: ControlSquare,
}

impl Instrument {
    pub fn new(event_loop: &glutin::event_loop::EventLoop<()>) -> Self {
        let xres = 800;
        let yres = 512;
    
        unsafe {
            let window_builder = glutin::window::WindowBuilder::new()
                .with_title("crazy synth")
                .with_inner_size(glutin::dpi::PhysicalSize::new(xres, yres));
            let window = glutin::ContextBuilder::new()
                .with_vsync(true)
                .build_windowed(window_builder, &event_loop)
                .unwrap()
                .make_current()
                .unwrap();
    
            let gl = glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);
    
            let prog = ProgramXYZRGBA::default(&gl);
            prog.bind(&gl);
            let mat4_ident = [1.0f32, 0., 0., 0., 0., -1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1. ];
            prog.set_proj(&mat4_ident, &gl);

            let mut cx = UIThreadContext::new();
            cx.send_samples(&[0.0; 880]);
            let r = rect(-1.0, 0.0, 2.0, 1.0);
            let r = rectv(r.xy + r.wh*0.1, r.wh * 0.8);
            let freq_vol = ControlSquare::new(vec2(0.5, 0.5), r);
            Instrument {
                xres,
                yres,
                window,
                gl,
                prog,
                mouse_lmb_held: false,
                mouse_pos: vec2(0.0, 0.0),
                cx,
                samples_generated: 0,
                rb: VecDeque::new(),
                last_sent: Instant::now(),
                freq_vol,
                play: false,
                phase: 0.0,
            }
        }
    }
    /// this writes all the samples that current time should have
    /// hmmm tapped delay line is in reverse order. 
    pub fn write_all_samples(&mut self) {
        let t_now = Instant::now();
        let d = t_now.duration_since(self.last_sent);

        let n_samples_needed = d.as_micros() * 44100 / 1_000_000;
        self.last_sent = t_now;
        for i in 0..n_samples_needed {
            self.next_sample();
        }
    }

    pub fn next_sample(&mut self) {
        self.samples_generated += 1;
        let mut x = 0.0;
        
        let freq = (5.0 + self.freq_vol.p.x * 9.0).exp2();
        let vol = self.freq_vol.p.y;

        self.phase += freq * 2.0 * PI / 44100.0;
        if self.phase > 2.0*PI { self.phase -= 2.0 * PI }
        x = vol * self.phase.sin();

        self.rb.push_back(x);
        self.cx.send_samples(&[x]);
    }

    pub fn handle_event(&mut self, event: glutin::event::Event<()>) {
        unsafe {
            match event {
                Event::LoopDestroyed |
                Event::WindowEvent {event: WindowEvent::CloseRequested, ..} => {
                    std::process::exit(0);
                },

                Event::WindowEvent {event, .. } => {
                    match event {
                        WindowEvent::Resized(size) => {
                            self.xres = size.width as i32;
                            self.yres = size.height as i32;
                            self.window.resize(size);
                            self.gl.viewport(0, 0, size.width as i32, size.height as i32);
                        },
                        WindowEvent::MouseInput{device_id, state, button, modifiers} => {
                            if button == MouseButton::Left {
                                if state == ElementState::Pressed {
                                    self.mouse_lmb_held = true;
                                } else {
                                    self.mouse_lmb_held = false;
                                }
                            }
                        },
                        WindowEvent::CursorMoved { device_id, position, modifiers } => {
                            if self.mouse_lmb_held {
                                self.freq_vol.update(vec2(position.x as f32 / self.xres as f32, position.y as f32 / self.yres as f32))
                            }
                            let x = position.x as f32 / self.xres as f32;
                            let y = position.y as f32 / self.yres as f32;
                            self.mouse_pos = vec2(x, y);
                        }
                        _ => {},
                    }
                },
                Event::MainEventsCleared => {
                    self.gl.clear_color(0.5, 0.5, 0.5, 1.0);
                    self.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

                    self.write_all_samples();
                    
                    let depth = 0.0;
                    let mut buf = self.draw();
                    let h = upload_xyzrgba_mesh(&buf, &self.gl);
                    h.render(&self.gl);
                    self.window.swap_buffers().unwrap();
                    h.free(&self.gl);
                },
                _ => {},
            }
        }
    }

    pub fn draw(&self) -> Vec<XYZRGBA> {
        let v = vec![];
        v
    }
}

// todo put cross thing etc

fn put_crosshair(buf: &mut Vec<XYZRGBA>, p: Vec2) {
    let d = -0.5;
    let c = vec4(1.0, 1.0, 0.0, 1.0);
    let w = 0.02;
    let h = 0.06;
    
    let wx = vec2(w, 0.0);
    let wy = vec2(0.0, w);
    
    let hx = vec2(h, 0.0);
    let hy = vec2(0.0, h);

    put_rect(buf, p - wx - wy, p + wx - wy - hy, c, d);
    put_rect(buf, p - wx + wy, p + wx + wy + hy, c, d);
    put_rect(buf, p - wx - wy - hx, p - wx + wy, c, d);
    put_rect(buf, p + wx - wy, p + wx + wy + hx, c, d);
}

// lol needed or not. might be one in this version of glow_mesh
pub fn put_rect(buf: &mut Vec<XYZRGBA>, min: Vec2, max: Vec2, col: Vec4, depth: f32) {
    let a = min;
    let b = vec2(max.x, min.y);
    let c = max;
    let d = vec2(min.x, max.y);

    put_quad(buf, a, b, c, d, col, depth);
}

// // give it the input state. it can have make n samples method or whatever
// // caller has the UIThread end of the audiocontext and will dump floats wherever

// fn next_sample(&mut self) -> f32 {
//     let a = self.p.a / 2.0 + 0.5;
//     let b = self.p.b / 2.0 + 0.5;
//     let c = self.p.c / 2.0 + 0.5;
//     let d = self.p.d / 2.0 + 0.5;
//     let e = self.p.e / 2.0 + 0.5;
//     let f = self.p.f / 2.0 + 0.5;

//     let period = a * 2.0 + 0.1;
//     let duty_cycle = b;
//     let et = 5.0 + e * 9.0;
//     let freq = et.exp2();
//     let ct = c * 8.0;
//     let fm_freq1 = ct.exp2();
//     let dt = 5.0 + d * 9.0;
//     // let fm_freq2 = dt.exp2();
//     let fm_freq2 = d*2.0;
//     // c and d can be fm freq multiplier and amplitude
//     // what about f cuz. harmonics? yea dont set it to begin with
//     // f be amplitude and make it maybe exp shit too

//     // sort out this shit

//     let period_samples = (period * 44100.0) as u64;
//     let n = self.n % period_samples;
//     let t = n as f32 / 44100.0;

//     let wn = 2.0 * PI / 44100.0;
//     let mut f_curr = freq;
//     self.fm_phase += wn * fm_freq1;
//     f_curr += self.fm_phase.sin() * fm_freq2 * freq;


//     self.phase += wn * f_curr;
//     if self.phase > 2.0*PI {
//         self.phase -= 2.0*PI;
//     }
//     if self.fm_phase > 2.0*PI {
//         self.fm_phase -= 2.0*PI;
//     }
//     self.n += 1;

//     // todo obviously window
//     if t/period < duty_cycle {
//         f * self.phase.sin() * 0.1
//     } else {
//         0.0
//     }


// }