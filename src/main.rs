#![allow(unused)]

mod adapters;
mod engine;
mod env;
mod error;
mod history;
mod indicators;
mod klinestore;
mod logger;
mod message_bus;
mod time;
mod tui;
mod types;
mod ws;

pub use error::Result;

use crate::env as app_env;
use adapters::binance::BinanceRest;
use engine::Engine;
use error::GlobalError;
use history::HistoryService;
use klinestore::KlineStore;
use log::info;
use logger::initialize_logger;
use message_bus::{EngineBus, HistoryBus, UiBus, WsBus};
use tokio::runtime::Builder;
use tui::run_tui;
use ws::WsClient;

fn main() -> Result<()> {
    let app_env = app_env::init_defaults();
    initialize_logger(&app_env)?;
    info!(
        "env: APP_ENV={}, RUST_LOG={}",
        app_env.app_env, app_env.rust_log
    );
    info!("grid: started");

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| GlobalError::Other(format!("runtime build error: {e}")))?;

    let binance = BinanceRest::builder().build();
    KlineStore::init(binance);

    // let ws_config = config.clone();

    let engine_bus = EngineBus::builder().build();
    let history_bus = HistoryBus::builder().build();
    let ws_bus = WsBus::builder().build();
    let ui_bus = UiBus::builder().build();

    let engine_tx_ui = engine_bus.engine_sender();
    let engine_tx_history = engine_bus.engine_sender();
    let history_tx_ui = history_bus.history_sender();
    let (engine_tx_ws, engine_rx) = engine_bus.into_engine();
    let (history_tx_engine, history_rx) = history_bus.into_engine();
    let (ws_tx_ui, ws_rx) = ws_bus.into_engine();
    let (ui_tx_engine, ui_rx) = ui_bus.into_engine();

    let handle = runtime.handle().clone();
    let ui_handle = runtime.handle().clone();

    let engine_handle = handle.spawn(async move {
        Engine::new(engine_rx, history_tx_engine, ui_tx_engine)
            .run()
            .await
    });
    let hist_handle = handle.spawn(async move {
        HistoryService::new(history_rx, engine_tx_history)
            .run()
            .await
    });
    let ws_handle = handle.spawn(async move { WsClient::new(ws_rx, engine_tx_ws).run().await });

    let watcher = {
        let handle = handle.clone();
        std::thread::spawn(move || {
            let res: Result<()> = handle.block_on(async move {
                let (engine_result, ws_result, hist_result) =
                    tokio::try_join!(engine_handle, ws_handle, hist_handle)
                        .map_err(|e| GlobalError::Other(format!("task join error: {e}")))?;
                engine_result?;
                ws_result?;
                hist_result?;
                Ok(())
            });
            if let Err(e) = res {
                panic!("background task failed: {e}");
            }
        })
    };

    run_tui(engine_tx_ui, history_tx_ui, ws_tx_ui, ui_handle, ui_rx)?;

    let _ = watcher.join();
    Ok(())
}
