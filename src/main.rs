#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::{bail, Result};
use clap::{Parser, ValueHint};
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::Receiver;

mod app;
mod utils;
mod vs_handler;

use app::{IccProfile, PreviewerResponse, ReloadType, VSCommand, VSCommandMsg, VSPreviewer};
use vs_handler::PreviewedScript;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), about = "VapourSynth script previewer", author = "quietvoid", version = env!("CARGO_PKG_VERSION"))]
struct Opt {
    #[arg(id = "input", value_hint = ValueHint::FilePath)]
    input: PathBuf,

    #[arg(
        id = "variable",
        visible_alias = "arg",
        short = 'v',
        visible_short_alias = 'a',
        help = "Variables to set in the script environment. Example: `-v key=value`",
        value_delimiter = ','
    )]
    variables: Vec<String>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let opt = Opt::parse();

    if !opt.input.is_file() {
        bail!("Input script file does not exist!");
    }

    let script = Arc::new(Mutex::new(PreviewedScript::new(opt.input, opt.variables)));
    let (cmd_sender, cmd_receiver) = tokio::sync::mpsc::channel(1);

    {
        let script = script.clone();
        tokio::spawn(async move {
            init_vs_command_loop(script, cmd_receiver).await;
        });
    }

    let previewer = VSPreviewer::new(script, cmd_sender);
    let res = eframe::run_native(
        "vspreview-rs",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(previewer.with_cc(cc)))),
    );

    if let Err(e) = res {
        bail!("Failed starting egui window: {}", e);
    }

    Ok(())
}

pub async fn init_vs_command_loop(
    script: Arc<Mutex<PreviewedScript>>,
    mut cmd_receiver: Receiver<VSCommandMsg>,
) {
    while let Some(msg) = cmd_receiver.recv().await {
        let VSCommandMsg {
            res_sender,
            cmd,
            egui_ctx,
        } = msg;
        let script = script.clone();

        match cmd {
            VSCommand::Reload => {
                let mut script_mutex = script.lock();
                let res = script_mutex.reload();
                script_mutex.add_vs_error(&res);

                let ret = if res.is_ok() {
                    let outputs_res = script_mutex.get_outputs();
                    script_mutex.add_vs_error(&outputs_res);

                    let outputs = if let Ok(outputs) = outputs_res {
                        // No output case handled by vapoursynth-rs
                        assert!(!outputs.is_empty());
                        Some(outputs)
                    } else {
                        None
                    };

                    // Not ready but we need to get the checker going
                    egui_ctx.request_repaint();

                    outputs
                } else {
                    None
                };

                res_sender.send(PreviewerResponse::Reload(ret))
            }
            VSCommand::Frame(fetch_image_state) => {
                let ret = match VSPreviewer::get_preview_image(egui_ctx, script, fetch_image_state)
                {
                    Ok(preview_frame) => preview_frame,
                    Err(e) => {
                        // Errors here are not recoverable
                        panic!("{}", e)
                    }
                };

                res_sender.send(PreviewerResponse::Frame(ret));
            }
            VSCommand::FrameProps(fetch_image_state) => {
                let ret = if let Some(mut script_mutex) = script.try_lock() {
                    let cur_output = fetch_image_state.state.cur_output;
                    let cur_frame_no = fetch_image_state.state.cur_frame_no;

                    let _lock = fetch_image_state.frame_mutex.lock();

                    let props_res = script_mutex.get_original_props(cur_output, cur_frame_no);
                    script_mutex.add_vs_error(&props_res);

                    if let Ok(props) = props_res {
                        egui_ctx.request_repaint();

                        Some(props)
                    } else {
                        None
                    }
                } else {
                    None
                };

                res_sender.send(PreviewerResponse::Props(ret));
            }
            VSCommand::ChangeScript => {
                let path = std::env::current_dir().unwrap();

                let new_file = rfd::FileDialog::new()
                    .set_title("Select a VapourSynth script file")
                    .add_filter("VapourSynth", &["vpy"])
                    .set_directory(path)
                    .pick_file();

                let ret = if let Some(new_file) = new_file {
                    let mut script_mutex = script.lock();

                    script_mutex.change_script_path(new_file);
                    egui_ctx.request_repaint();

                    ReloadType::Reload
                } else {
                    ReloadType::None
                };

                res_sender.send(PreviewerResponse::Misc(ret));
            }
            VSCommand::ChangeIcc(transforms) => {
                let new_file = rfd::FileDialog::new()
                    .set_title("Select a ICC profile file")
                    .add_filter("ICC", &["icc", "icm"])
                    .pick_file();

                let ret = if let Some(new_file) = new_file {
                    let mut transforms = transforms.lock();

                    let mut profile = IccProfile::srgb(new_file);
                    profile.setup();

                    transforms.icc = Some(profile);

                    egui_ctx.request_repaint();

                    ReloadType::Reprocess
                } else {
                    ReloadType::None
                };

                res_sender.send(PreviewerResponse::Misc(ret));
            }
            VSCommand::Exit => {
                let script_mutex = script.lock();
                script_mutex.exit();

                res_sender.send(PreviewerResponse::Close);

                break;
            }
        }
    }

    cmd_receiver.close();
}
