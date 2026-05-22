use std::sync::{Arc, RwLock};

use clap::Parser;
use replay_core::config::AppConfig;
use replay_engine::control_api::ControlApi;
use replay_engine::controller::{EngineController, StatusSnapshot};
use replay_engine::hotkeys;
use replay_engine::logging;
use replay_engine::program_output::{should_use_headless, ProgramOutputHandle, UiSpawnConfig};
use replay_engine::ui::OperatorCmd;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "replay-engine", about = "Instant Replay — Raspberry Pi 5 appliance")]
struct Args {
    /// Load config and autostart live capture on boot.
    #[arg(long)]
    appliance: bool,

    /// Use videotestsrc instead of V4L2 capture.
    #[arg(long)]
    test: bool,

    /// No native operator window (CI / headless).
    #[arg(long)]
    no_ui: bool,
}

fn build_program_handle(
    config: &AppConfig,
    args: &Args,
    status: Arc<RwLock<StatusSnapshot>>,
    op_tx: Option<std::sync::mpsc::Sender<OperatorCmd>>,
) -> ProgramOutputHandle {
    if should_use_headless(args.test) || args.no_ui {
        return ProgramOutputHandle::headless();
    }

    let show_operator = config.operator.enabled && !args.no_ui;
    if show_operator {
        ProgramOutputHandle::spawn_ui(UiSpawnConfig {
            operator: Some(config.operator.clone()),
            status,
            cmd_tx: op_tx,
        })
    } else {
        ProgramOutputHandle::spawn_program_only()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();
    let args = Args::parse();

    let mut config = AppConfig::load().unwrap_or_default();

    if args.test {
        config.input.device_id = "test".into();
        config.storage.buffer_path = std::path::PathBuf::from("/tmp/instant-replay/buffer");
        config.appliance.autostart_live = false;
    }

    if config.storage.auto_clean_on_start {
        let _ = std::fs::create_dir_all(&config.storage.buffer_path);
    }

    let status = Arc::new(RwLock::new(StatusSnapshot::default_offline()));
    let show_operator =
        config.operator.enabled && !args.no_ui && !should_use_headless(args.test);
    let (op_tx, op_rx) = if show_operator {
        let (t, r) = std::sync::mpsc::channel();
        (Some(t), Some(r))
    } else {
        (None, None)
    };

    let program = build_program_handle(&config, &args, status.clone(), op_tx);

    let (controller, event_receivers) =
        EngineController::new(config.clone(), args.test, program);
    let controller = Arc::new(controller);
    let rt_handle = tokio::runtime::Handle::current();
    EngineController::spawn_event_handlers(
        controller.clone(),
        event_receivers,
        rt_handle.clone(),
    );
    replay_engine::device_watch::spawn_device_watch(controller.clone());

    let api = ControlApi::new(controller.clone());

    hotkeys::spawn_hotkey_handler(api.clone(), config.hotkeys.clone(), rt_handle.clone());

    let status_for_ui = status.clone();
    let mut status_rx = api.subscribe_status();
    tokio::spawn(async move {
        loop {
            if let Ok(snap) = status_rx.recv().await {
                if let Ok(mut g) = status_for_ui.write() {
                    *g = snap;
                }
            }
        }
    });

    let status_api = api.clone();
    tokio::spawn(async move {
        loop {
            status_api.controller().publish_status().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    if let Some(op_rx) = op_rx {
        let api_cmds = api.clone();
        let rt = rt_handle.clone();
        std::thread::Builder::new()
            .name("operator-cmd".to_string())
            .spawn(move || {
                while let Ok(cmd) = op_rx.recv() {
                    let result = rt.block_on(async {
                        match cmd {
                            OperatorCmd::Mark => api_cmds.mark().await.map(|_| ()),
                            OperatorCmd::Replay => api_cmds.replay().await,
                            OperatorCmd::ReplayLast => api_cmds.replay_last(10).await,
                            OperatorCmd::ReturnLive => api_cmds.return_live().await,
                            OperatorCmd::ClearMark => api_cmds.clear_mark().await,
                        }
                    });
                    if let Err(e) = result {
                        tracing::warn!(error = %e, "Operator command failed");
                    }
                }
            })
            .expect("spawn operator-cmd thread");
    }

    let autostart = (args.appliance || config.appliance.enabled) && config.appliance.autostart_live;
    if autostart && should_use_headless(args.test) {
        tracing::warn!(
            "Skipping live autostart: no DISPLAY (set DISPLAY=:0 in replay-engine.service and enable desktop autologin)"
        );
    } else if autostart {
        info!("Appliance mode: autostart live");
        let (w, h) = config.parse_resolution().unwrap_or((1920, 1080));
        let device = if config.input.device_id.is_empty() {
            "test".to_string()
        } else {
            config.input.device_id.clone()
        };
        if let Err(e) = api
            .start_live(
                device,
                w,
                h,
                config.input.fps,
                config.input.pixel_format.clone(),
                config.output.display_id,
                config.output.fullscreen,
            )
            .await
        {
            tracing::error!(error = %e, "Appliance autostart failed");
        }
    }

    if !args.no_ui && config.operator.enabled && !should_use_headless(args.test) {
        info!("Native operator UI running (close window to exit)");
    } else {
        info!("Running without operator UI (keyboard hotkeys only)");
    }
    std::future::pending::<()>().await;
    Ok(())
}
