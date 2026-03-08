use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer as _, Split as _};
use ringbuf::HeapRb;

pub const SAMPLE_RATE: u32 = 44_100;

pub struct AudioOutput {
    pub producer: ringbuf::HeapProd<f32>,
    _stream: cpal::Stream,
}

impl AudioOutput {
    pub fn new() -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate::from(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let rb = HeapRb::<f32>::new(4096);
        let (producer, mut consumer) = rb.split();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for sample in data.iter_mut() {
                        *sample = consumer.try_pop().unwrap_or(0.0);
                    }
                },
                |err| eprintln!("APU stream error: {err}"),
                None,
            )
            .ok()?;

        stream.play().ok()?;

        Some(Self {
            producer,
            _stream: stream,
        })
    }
}
