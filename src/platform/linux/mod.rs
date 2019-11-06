use crate::media::InterfaceTrait;
use byte_slice_cast::*;
use gstreamer::prelude::*;
use gstreamer::{Caps, FlowSuccess, Pipeline, State};
use gstreamer_app::{AppSink, AppSrc};
use gstreamer_audio::AUDIO_FORMAT_S16;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, sync_channel};
use std::net::{TcpListener, TcpStream};

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
        gstreamer::init().expect("Could not init gstreamer");

        let pipeline = Pipeline::new(None);
        let src = gstreamer::ElementFactory::make("appsrc", None).expect("Could not make audiotestsrc");
        let sink = gstreamer::ElementFactory::make("autoaudiosink", None).expect("Could not make appsink");

        pipeline.add_many(&[&src, &sink]).expect("Add elements to pipeline");
        src.link(&sink).expect("Could not link src to sink");

        let app_src = src.dynamic_cast::<AppSrc>().expect("Could not make AppSrc");
        app_src.set_caps(Some(&Caps::new_simple(
            "audio/x-raw",
            &[
                ("format", &AUDIO_FORMAT_S16.to_string()),
                ("layout", &"interleaved"),
                ("channels", &(2i32)),
                ("rate", &(48_000)),
            ],
        )));

        let mut file = File::open("audio.raw").expect("Could not open audio.raw");
        let mut file_buffer = vec![];
        file.read_to_end(&mut file_buffer).expect("Could not read to end of file");

        app_src.set_callbacks(
            gstreamer_app::AppSrcCallbacks::new()
                .need_data(move |app_src, _| {
                    let buffer_size = 1920;
                    let mut buffer = gstreamer::Buffer::with_size(buffer_size).unwrap();
                    {
                        let buffer = buffer.get_mut().unwrap();
                        let mut data = buffer.map_writable().unwrap();

                        let input: Vec<_> = file_buffer.drain(..buffer_size).collect();
                        let mut i = 0;
                        for p in data.as_mut_slice() {
                            *p = input[i];
                            i += 1;
                        }
                    }

                    let _ = app_src.push_buffer(buffer);
                }).build()
        );

        gst_main_loop(pipeline);
    }

    fn start_recording(&self) -> Result<(), ()> {
        let (pipeline, receiver) = create_pipeline();
        let serve_thread = std::thread::spawn(move || {
            accept_clients(receiver);
        });

        gst_main_loop(pipeline);
        serve_thread.join();
    }
}

fn create_pipeline() -> (Pipeline, Receiver<Vec<u8>>) {
    gstreamer::init().expect("Could not init gstreamer");

    let pipeline = Pipeline::new(None);
    let src = gstreamer::ElementFactory::make("pulsesrc", None).expect("Could not make audiotestsrc");
    let sink = gstreamer::ElementFactory::make("appsink", None).expect("Could not make appsink");

    src.set_property("device", &"alsa_output.pci-0000_00_1b.0.analog-stereo.monitor").expect("Could not set device");

    pipeline.add_many(&[&src, &sink]).expect("Add elements to pipeline");
    src.link(&sink).expect("Could not link src to sink");

    let app_sink = sink.dynamic_cast::<AppSink>().expect("Sink element is expected to be an appsink");
    app_sink.set_caps(Some(&Caps::new_simple(
        "audio/x-raw",
        &[
            ("format", &AUDIO_FORMAT_S16.to_string()),
            ("layout", &"interleaved"),
            ("channels", &(2i32)),
            ("rate", &(48_000)),
        ],
    )));

    let (sender, receiver) = sync_channel(1);
    app_sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::new()
            .new_sample(move |appsink| {
                let sample = appsink.pull_sample().expect("Could not get sample");
                let buffer = sample.get_buffer().expect("Could not get buffer");

                let map = buffer.map_readable().expect("Could not map buffer to readable memory");
                let samples = map.as_slice_of::<u8>().expect("Could not get samples");

                sender.send(samples.to_vec());

                Ok(FlowSuccess::Ok)
            })
            .build()
    );

    (pipeline, receiver)
}

fn gst_main_loop(pipeline: Pipeline) {
    pipeline.set_state(State::Playing).expect("Could not set gstreamer state to Playing");

    let bus = pipeline
        .get_bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gstreamer::CLOCK_TIME_NONE) {
        use gstreamer::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(State::Null).expect("Could not set gstreamer state to Null");
                eprintln!("gstreamer error ({:?})", err);
                return;
            }
            _ => (),
        }
    }

    pipeline.set_state(State::Null).expect("Could not set gstreamer state to Null");
}

fn accept_clients(receiver: Receiver<Vec<u8>>) {
    let mut listener = TcpListener::bind("0.0.0.0:42795").expect("Could not bind to port");
    listener.set_nonblocking(true).expect("Could not make listener non-blocking");
    let mut clients = vec![];

    'accept: loop {
        match listener.accept() {
            Ok((stream, _address)) => {
                println!("New client");
                clients.push(stream);
            }
            _ => ()
        }

        match receiver.try_recv() {
            Ok(val) => {
                for mut client in &clients {
                    client.write_all(&val).expect("Could not write to TCP stream");
                }
            },
            _ => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            },
        }
    }
}
