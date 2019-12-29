use crate::media::InterfaceTrait;
use crate::network::accept_clients;
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

        let mut stream = TcpStream::connect("192.168.0.37:42795").expect("Could not connect to TCP stream");

        app_src.set_callbacks(
            gstreamer_app::AppSrcCallbacks::new()
                .need_data(move |app_src, _| {
                    let buffer_size = 1920;
                    let mut buffer = gstreamer::Buffer::with_size(buffer_size).unwrap();
                    {
                        let buffer = buffer.get_mut().unwrap();
                        let mut data = buffer.map_writable().unwrap();

                        let mut input = vec![0; buffer_size];
                        stream.read_exact(&mut input).unwrap();

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
