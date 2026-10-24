mod api;
mod models;
mod playback;
mod playback_watch;
mod player;
mod services;
mod storage;
mod ui;

pub fn run_gpui() -> gpui_kit::Result<()> {
    ui::run()
}
