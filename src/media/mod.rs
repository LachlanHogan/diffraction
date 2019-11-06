#[cfg(target_os = "windows")]
use crate::platform::windows::Interface;

#[cfg(target_os = "linux")]
use crate::platform::linux::Interface;

pub trait InterfaceTrait {
    fn init(&self);
    fn start_playback(&self) -> Result<(), ()>;
    fn start_recording(&self) -> Result<(), ()>;
}

#[cfg(target_os = "windows")]
pub fn create_audio_interface() -> Interface {
    Interface::new()
}

#[cfg(target_os = "linux")]
pub fn create_audio_interface() -> Interface {
    Interface::new()
}
