use std::ptr;
use winapi::Interface;
use winapi::shared::minwindef::{BYTE, DWORD};
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::audioclient::{IID_IAudioClient, IAudioClient, IAudioCaptureClient, IID_IAudioCaptureClient, IID_IAudioRenderClient, IAudioRenderClient};
use winapi::um::audiosessiontypes::AUDCLNT_SHAREMODE_SHARED;
use winapi::um::objbase::CoInitialize;
use winapi::um::mmdeviceapi::{CLSID_MMDeviceEnumerator, IMMDeviceEnumerator, IMMDevice, eRender, eConsole};
use winapi::um::combaseapi::{CoCreateInstance, CLSCTX_ALL, CoTaskMemFree};
use winapi::shared::mmreg::WAVEFORMATEX;
use winapi::um::strmif::REFERENCE_TIME;

pub struct COM();

impl COM {
    pub fn init() -> Result<(), ()> {
        let result = unsafe { CoInitialize(ptr::null_mut()) };
        if !SUCCEEDED(result) {
            eprintln!("CoInitialize failed! {:#x}", result);
            return Err(());
        }
        Ok(())
    }
}

pub struct DeviceEnumerator {
    ptr: *mut IMMDeviceEnumerator,
}

impl DeviceEnumerator {
    pub fn create() -> Result<DeviceEnumerator, ()> {
        let mut ptr: *mut IMMDeviceEnumerator = ptr::null_mut();
        let result = unsafe {
            CoCreateInstance(&CLSID_MMDeviceEnumerator, ptr::null_mut(), CLSCTX_ALL, &IMMDeviceEnumerator::uuidof(), &mut ptr as *mut _ as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("CoCreateInstance failed! {:#x}", result);
            return Err(());
        }

        Ok(DeviceEnumerator { ptr })
    }

    pub fn get_default_audio_endpoint(&self) -> Result<AudioDevice, ()> {
        let mut ptr: *mut IMMDevice = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).GetDefaultAudioEndpoint(eRender, eConsole, &mut ptr)
        };

        if !SUCCEEDED(result) {
            eprintln!("IMMDeviceEnumerator->GetDefaultAudioEndpoint failed! {:#x}", result);
            return Err(());
        }
        Ok(AudioDevice { ptr })
    }
}

impl Drop for DeviceEnumerator {
    fn drop(&mut self) {
        println!("Dropping DeviceEnumerator");
        unsafe { (*self.ptr).Release(); }
    }
}

pub struct AudioDevice {
    ptr: *mut IMMDevice,
}

impl AudioDevice {
    pub fn activate(&self) -> Result<AudioClient, ()> {
        let mut ptr: *mut IAudioClient = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).Activate(&IID_IAudioClient, CLSCTX_ALL, ptr::null_mut(), &mut ptr as *mut _ as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IMMDevice->Activate failed! {:#x}", result);
            return Err(());
        }
        Ok(AudioClient { ptr })
    }
}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        println!("Dropping AudioDevice");
        unsafe { (*self.ptr).Release(); }
    }
}

pub struct AudioClient {
    ptr: *mut IAudioClient,
}

impl AudioClient {
    pub fn get_mix_format(&self) -> Result<MixFormat, ()> {
        let mut ptr: *mut WAVEFORMATEX = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).GetMixFormat(&mut ptr)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->GetMixFormat failed! {:#x}", result);
            return Err(());
        }
        Ok(MixFormat { ptr })
    }

    pub fn initialize(&self, stream_flags: u32, mix_format: MixFormat) -> Result<(), ()> {
        let buffer_duration: REFERENCE_TIME = 1000000;
        let result = unsafe {
            (*self.ptr).Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags, buffer_duration, 0, mix_format.ptr, ptr::null_mut())
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->Initialize failed! {:#x}", result);
            return Err(());
        }
        Ok(())
    }

    pub fn get_buffer_size(&self) -> Result<u32, ()> {
        let mut buffer_size: u32 = 0;
        let result = unsafe {
            (*self.ptr).GetBufferSize(&mut buffer_size as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->GetBufferSize failed! {:#x}", result);
            return Err(());
        }
        Ok(buffer_size)
    }

    pub fn get_render_service(&self) -> Result<AudioRenderClient, ()> {
        let mut ptr: *mut IAudioRenderClient = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).GetService(&IID_IAudioRenderClient, &mut ptr as *mut _ as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->GetService failed! {:#x}", result);
            return Err(());
        }
        Ok(AudioRenderClient { ptr })
    }

    pub fn get_capture_service(&self) -> Result<AudioCaptureClient, ()> {
        let mut ptr: *mut IAudioCaptureClient = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).GetService(&IID_IAudioCaptureClient, &mut ptr as *mut _ as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->GetService failed! {:#x}", result);
            return Err(());
        }
        Ok(AudioCaptureClient { ptr })
    }

    pub fn start(&self) -> Result<(), ()> {
        let result = unsafe {
            (*self.ptr).Start()
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->Start failed! {:#x}", result);
            return Err(());
        }
        Ok(())
    }

    pub fn get_current_padding(&self) -> Result<u32, ()> {
        let mut num_frames_padding: u32 = 0;
        let result = unsafe {
            (*self.ptr).GetCurrentPadding(&mut num_frames_padding as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->GetCurrentPadding failed! {:#x}", result);
            return Err(());
        }
        Ok(num_frames_padding)
    }

    pub fn stop(&self) -> Result<(), ()> {
        let result = unsafe {
            (*self.ptr).Stop()
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioClient->Stop failed! {:#x}", result);
            return Err(());
        }

        Ok(())
    }
}

impl Drop for AudioClient {
    fn drop(&mut self) {
        println!("Dropping AudioClient");
        let _ = self.stop();
        unsafe { (*self.ptr).Release(); }
    }
}

#[derive(Clone)]
pub struct MixFormat {
    pub ptr: *mut WAVEFORMATEX,
}

impl Drop for MixFormat {
    fn drop(&mut self) {
        println!("Dropping MixFormat");
        unsafe { CoTaskMemFree(self.ptr as *mut _); }
    }
}

pub struct AudioRenderClient {
    ptr: *mut IAudioRenderClient,
}

impl AudioRenderClient {
    pub fn get_buffer(&self, buffer_size: u32, bytes_per_frame: u16) -> Result<&mut [u8], ()> {
        let mut data: *mut BYTE = ptr::null_mut();
        let result = unsafe {
            (*self.ptr).GetBuffer(buffer_size, &mut data)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioRenderClient->GetBuffer failed! {:#x}", result);
            return Err(());
        }

        let slice = unsafe { std::slice::from_raw_parts_mut(data, buffer_size as usize * bytes_per_frame as usize) };
        Ok(slice)
    }

    pub fn release_buffer(&self, buffer_size: u32) -> Result<(), ()> {
        let result = unsafe {
            (*self.ptr).ReleaseBuffer(buffer_size, 0)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioRenderClient->ReleaseBuffer failed! {:#x}", result);
            return Err(());
        }
        Ok(())
    }
}

impl Drop for AudioRenderClient {
    fn drop(&mut self) {
        println!("Dropping AudioRenderClient");
        unsafe { (*self.ptr).Release(); }
    }
}

pub struct AudioCaptureClient {
    ptr: *mut IAudioCaptureClient,
}

impl AudioCaptureClient {
    pub fn get_next_packet_size(&self) -> Result<u32, ()> {
        let mut packet_size: u32 = 0;
        let result = unsafe {
            (*self.ptr).GetNextPacketSize(&mut packet_size as *mut _)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioCaptureClient->GetNextPacketSize failed! {:#x}", result);
            return Err(());
        }

        Ok(packet_size)
    }

    pub fn get_buffer(&self, bytes_per_frame: u16) -> Result<(&[u8], u32), ()> {
        let mut data: *mut BYTE = ptr::null_mut();
        let mut num_frames_available: u32 = 0;
        let mut flags: DWORD = 0;
        let mut audio_position: u64 = 0;
        let result = unsafe {
            (*self.ptr).GetBuffer(&mut data, &mut num_frames_available as *mut _, &mut flags, &mut audio_position as *mut _, ptr::null_mut())
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioCaptureClient->GetBuffer failed! {:#x}", result);
            return Err(());
        }

        // 2 channel 32-bit float slice of audio
        let audio = unsafe { std::slice::from_raw_parts(data, num_frames_available as usize * bytes_per_frame as usize) };

        Ok((audio, num_frames_available))
    }

    pub fn release_buffer(&self, num_frames_available: u32) -> Result<(), ()> {
        let result = unsafe {
            (*self.ptr).ReleaseBuffer(num_frames_available)
        };

        if !SUCCEEDED(result) {
            eprintln!("IAudioCaptureClient->ReleaseBuffer failed! {:#x}", result);
            return Err(());
        }

        Ok(())
    }
}

impl Drop for AudioCaptureClient {
    fn drop(&mut self) {
        println!("Dropping AudioCaptureClient");
        unsafe { (*self.ptr).Release(); }
    }
}
