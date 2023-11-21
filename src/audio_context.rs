use std::f32::consts::PI;

use cpal::traits::*;
use cpal::Device;
use ringbuf::*;

pub fn initialize_audio() -> UIThreadContext {
    // init code goes here
    let (prod, cons) = RingBuffer::<f32>::new(44100).split();
    let host = cpal::default_host();
    let device = host.default_output_device().expect("Failed to retrieve default output device");
    println!("Output device : {}", device.name().expect("couldnt get device name (??? idk)"));
    let config = device.default_output_config().expect("failed to get default output config");
    println!("Default output config : {:?}", config);
    let sample_rate = config.sample_rate().0;
    let sample_format = config.sample_format();
    let channels = config.channels();

    let mut ac = AudioThreadContext {
        cons,
    };

    let output_callback = move |output: &mut [f32], info: &cpal::OutputCallbackInfo| {
        ac.write_chunk(output, info);
    };

    let config = cpal::StreamConfig {
        channels: channels,
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(&config, output_callback, |_| panic!("error"), None),
        _ => panic!("unsupported"),
    }.expect("failed to make stream");
    stream.play().expect("failed to play stream");
    UIThreadContext {
        stream,
        prod,
        tx: 0,
    }
}

pub struct UIThreadContext {
    stream: cpal::Stream,
    prod: Producer<f32>,   
    tx: usize,
}

impl UIThreadContext {
    pub fn new() -> Self {
        initialize_audio()
    }
    pub fn send_samples(&mut self, samples: &[f32]) {
        self.tx += self.prod.push_slice(samples);
    }
}

pub struct AudioThreadContext {
    cons: Consumer<f32>,
}

impl AudioThreadContext {
    fn write_chunk(&mut self, output: &mut [f32], info: &cpal::OutputCallbackInfo) {
        self.cons.pop_slice(output);
    }
}

// more eg panning sin function etc
// fft view always good