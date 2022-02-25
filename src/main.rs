#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::Parser;

mod app;
mod utils;
mod vs_handler;

use app::VSPreviewer;
use vs_handler::PreviewedScript;

#[derive(Parser, Debug)]
#[clap(name = env!("CARGO_PKG_NAME"), about = "VapourSynth script previewer", author = "quietvoid", version = env!("CARGO_PKG_VERSION"))]
struct Opt {
    #[clap(name = "input", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let options = eframe::NativeOptions::default();
    let opt = Opt::parse();

    let previewer = VSPreviewer {
        script: Arc::new(Mutex::new(PreviewedScript::new(opt.input))),
        ..Default::default()
    };

    eframe::run_native(Box::new(previewer), options);
}
