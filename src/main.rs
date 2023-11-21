use glow::HasContext;
use glutin::event::ElementState;
use glutin::event::MouseButton;
use audio_context::*;
use minvect::*;
use glow_mesh::xyzrgba::*;
use glutin::event::{Event, WindowEvent};

mod audio_context;
mod instrument;

use crate::instrument::*;

pub fn main() {
        let event_loop = glutin::event_loop::EventLoop::new();
        let mut demo = Instrument::new(&event_loop);
        event_loop.run(move |event, _, _| demo.handle_event(event));
}

