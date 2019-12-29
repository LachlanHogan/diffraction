use crate::media::InterfaceTrait;
use crate::network::accept_clients;
use byteorder::{ByteOrder, LittleEndian};
use wasapi::{COM, DeviceEnumerator};
use winapi::um::audiosessiontypes::AUDCLNT_STREAMFLAGS_LOOPBACK;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc::sync_channel;

mod wasapi;

pub struct Interface;

impl Interface {
    pub fn new() -> Self {
        Interface
    }
}

impl InterfaceTrait for Interface {
    fn init(&self) {
    }

    fn start_playback(&self) -> Result<(), ()> {
        let mut stream = TcpStream::connect("192.168.0.63:42795").expect("Could not connect to TCP stream");

        COM::init()?;

        let device_enumerator = DeviceEnumerator::create()?;
        let device = device_enumerator.get_default_audio_endpoint()?;

        let audio_client = device.activate()?;
        let mix_format = audio_client.get_mix_format()?;
        let bytes_per_frame = unsafe { (*mix_format.ptr).nBlockAlign };
        audio_client.initialize(0, mix_format)?;

        let buffer_size = audio_client.get_buffer_size()?;
        let render_client = audio_client.get_render_service()?;

        let buffer = render_client.get_buffer(buffer_size, bytes_per_frame)?;

        let mut input = vec![0; buffer.len() / 2];
        stream.read_exact(&mut input).expect("Could not read samples from stream");
        let floating_point_input = convert_signed_pcm_to_floating_point(input);

        for i in 0..floating_point_input.len() {
            buffer[i] = floating_point_input[i];
        }

        render_client.release_buffer(buffer_size)?;
        audio_client.start()?;

        loop {
            let num_frames_padding = audio_client.get_current_padding()?;
            let num_frames_available = buffer_size - num_frames_padding;
            if num_frames_available > 0 {
                let buffer = render_client.get_buffer(num_frames_available, bytes_per_frame)?;
                input = vec![0; buffer.len() / 2];
                stream.read_exact(&mut input).expect("Could not read samples from stream");
                let floating_point_input = convert_signed_pcm_to_floating_point(input);

                for i in 0..floating_point_input.len() {
                    buffer[i] = floating_point_input[i];
                }

                render_client.release_buffer(num_frames_available)?;
            }
        }
    }

    fn start_recording(&self) -> Result<(), ()> {
        COM::init()?;

        let device_enumerator = DeviceEnumerator::create()?;
        let device = device_enumerator.get_default_audio_endpoint()?;

        let audio_client = device.activate()?;
        let mix_format = audio_client.get_mix_format()?;
        let bytes_per_frame = unsafe { (*mix_format.ptr).nBlockAlign };
        audio_client.initialize(AUDCLNT_STREAMFLAGS_LOOPBACK, mix_format.clone())?;

        let capture_client = audio_client.get_capture_service()?;

        audio_client.start()?;

        let (sender, receiver) = sync_channel(1);
        let _serve_thread = std::thread::spawn(move || {
            accept_clients(receiver);
        });

        'main: loop {
            let mut packet_size = capture_client.get_next_packet_size()?;

            while packet_size > 0 {
                let (audio, num_frames_available) = capture_client.get_buffer(bytes_per_frame)?;
                let signed_pcm = convert_floating_point_to_signed_pcm(&audio);

                sender.send(signed_pcm).expect("Could not send PCM data");

                capture_client.release_buffer(num_frames_available)?;
                packet_size = capture_client.get_next_packet_size()?;
            }
        }
    }
}

fn convert_signed_pcm_to_floating_point(signed_pcm: Vec<u8>) -> Vec<u8> {
    let mut floating_point_input = Vec::with_capacity(signed_pcm.len() * 2);
    for i in (0..signed_pcm.len()).step_by(2) {
        let bytes = [signed_pcm[i], signed_pcm[i + 1]];
        let signed_pcm = LittleEndian::read_i16(&bytes);
        let normalised_signed_pcm = signed_pcm as f32 / 32_767.0;

        let mut buffer = [0; 4];
        LittleEndian::write_f32(&mut buffer, normalised_signed_pcm);

        floating_point_input.extend_from_slice(&buffer);
    }
    floating_point_input
}

fn convert_floating_point_to_signed_pcm(floating_point: &[u8]) -> Vec<u8> {
    let mut signed_pcm = Vec::with_capacity(floating_point.len() / 2);
    for i in (0..floating_point.len()).step_by(4) {
        let bytes = [floating_point[i], floating_point[i + 1], floating_point[i + 2], floating_point[i + 3]];
        let floating_point = LittleEndian::read_f32(&bytes);
        let normalised_floating_point = (floating_point * 32_767.0) as i16;

        let mut buffer = [0; 2];
        LittleEndian::write_i16(&mut buffer, normalised_floating_point);

        signed_pcm.extend_from_slice(&buffer);
    }
    signed_pcm
}
