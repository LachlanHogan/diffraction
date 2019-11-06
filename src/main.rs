use media::{create_audio_interface, InterfaceTrait};

mod media;
mod platform;

fn main() {
    let audio_interface = create_audio_interface();
    audio_interface.start_playback();
}
