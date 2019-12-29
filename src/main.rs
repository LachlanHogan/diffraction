use media::{create_audio_interface, InterfaceTrait};

mod media;
mod platform;
mod network;

fn main() {
    let audio_interface = create_audio_interface();
    let _ = audio_interface.start_recording();
}
