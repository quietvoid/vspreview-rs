#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::{bail, Result};
use clap::{Parser, ValueHint};
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};

mod app;
mod utils;
mod vs_handler;

use app::VSPreviewer;
use vs_handler::PreviewedScript;

#[derive(Parser, Debug)]
#[clap(name = env!("CARGO_PKG_NAME"), about = "VapourSynth script previewer", author = "quietvoid", version = env!("CARGO_PKG_VERSION"))]
struct Opt {
    #[clap(name = "input", value_hint = ValueHint::FilePath)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions::default();

    let opt = Opt::parse();

    if !opt.input.is_file() {
        bail!("Input script file does not exist!");
    }

    let previewer = VSPreviewer {
        script: Arc::new(Mutex::new(PreviewedScript::new(opt.input))),
        ..Default::default()
    };

    eframe::run_native(
        "vspreview-rs",
        options,
        Box::new(|cc| Box::new(previewer.with_cc(cc))),
    );

    Ok(())
}
