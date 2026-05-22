use std::sync::Arc;

use clap::Parser;
use replay_core::config::AppConfig;
use replay_engine::control_api::ControlApi;
use replay_engine::controller::EngineController;
use replay_engine::hotkeys;
use replay_engine::http;
use replay_engine::logging;
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

    /// Disable embedded touch HTTP server.
    #[arg(long)]
    no_http: bool,
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

    let (controller, event_receivers) = EngineController::new(config.clone(), args.test);
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

    let status_api = api.clone();
    tokio::spawn(async move {
        loop {
            status_api.controller().publish_status().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let autostart = (args.appliance || config.appliance.enabled) && config.appliance.autostart_live;
    if autostart {
        info!("Appliance mode: autostart live");
        {
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
    }

    if !args.no_http && config.http.enabled {
        let bind = config.http.bind_addr.clone();
        http::serve(&bind, api).await?;
    } else {
        info!("HTTP disabled; use keyboard hotkeys only");
        std::future::pending::<()>().await;
    }

    Ok(())
}
