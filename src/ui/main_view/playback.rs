use super::state::{MainView, Page};
use crate::models::ItemInput;
use crate::playback_watch::{WatchPlaybackInput, watch};
use crate::player::{PlayerSession, PlayerSessionConfig};
use crate::services::{AppPaths, PlaybackService};
use gpui_kit::Context;
use gpui_kit::prelude::*;
use std::time::Duration;

const PLAYBACK_UI_POLL_INTERVAL: Duration = Duration::from_millis(500);

impl MainView {
    pub(crate) fn resolve_playback(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = self.detail.as_ref() else {
            return;
        };
        let media_source_id = detail
            .media_sources
            .first()
            .map(|source| source.id.clone());
        self.resolve_playback_for(
            detail.item.id.clone(),
            detail.item.server_id.clone(),
            media_source_id,
            cx,
        );
    }

    /// Resolve playback for an arbitrary item (used by the detail page's main
    /// "播放" button and by episode rows).
    pub(crate) fn resolve_playback_for(
        &mut self,
        item_id: String,
        server_id: String,
        media_source_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.loading = true;
        self.error = None;
        self.resolved_session = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match PlaybackService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let input = ItemInput {
            // The id is needed twice: once for the resolve request and once
            // when the resolved session auto-starts playback below.
            item_id: item_id.clone(),
            server_id: Some(server_id),
            media_source_id,
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitle_stream_position: None,
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.resolve(input) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(session) => {
                        this.resolved_session = Some(session);
                        this.loading = false;
                        this.error = None;
                        // Resolve 成功后自动开始播放，供详情页"播放"按钮与
                        // 集列表行点击复用。
                        let _ = this.start_player_with(item_id, cx);
                    }
                    Err(err) => {
                        this.loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn start_player(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = self.detail.as_ref() else {
            return;
        };
        self.start_player_with(detail.item.id.clone(), cx);
    }

    /// Start the player for a resolved session, playing `item_id` (used for the
    /// detail page's main "播放" button and for episode rows).
    pub(crate) fn start_player_with(&mut self, item_id: String, cx: &mut Context<Self>) {
        let Some(session) = self.resolved_session.clone() else {
            return;
        };
        let media_source_id = session.media_source_id.clone();
        let play_session_id = session.play_session_id.clone().unwrap_or_default();
        let play_method = session.play_method();
        let start_position_ticks = session.start_position_ticks;

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        self.loading = true;
        self.error = None;
        cx.notify();
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let player_paths = paths.player();
            let service = PlaybackService::new(&paths);
            let default_volume = paths
                .store()
                .and_then(|store| store.settings())
                .map(|settings| settings.default_volume)
                .unwrap_or(100);

            let result = cx
                .background_spawn(async move {
                    let player = PlayerSession::start(PlayerSessionConfig {
                        paths: player_paths,
                        playback: session,
                        default_volume,
                        audio_device: None,
                    })?;
                    if let Ok(service) = service {
                        let watch_player = player.clone();
                        std::thread::spawn(move || {
                            let _stopped = watch(WatchPlaybackInput {
                                service,
                                player: watch_player,
                                item_id,
                                media_source_id,
                                play_session_id,
                                play_method,
                                start_position_ticks,
                            });
                        });
                    }
                    Ok(player)
                })
                .await;

            view.update(cx, move |this, cx| {
                match result {
                    Ok(player) => {
                        this.player = Some(player);
                        this.playback_state = None;
                        this.error = None;
                        this.loading = false;
                        this.page = Page::Player;
                        this.poll_playback_state(cx);
                    }
                    Err(err) => {
                        this.player = None;
                        this.error = Some(err);
                        this.loading = false;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn poll_playback_state(&mut self, cx: &mut Context<Self>) {
        let Some(player) = self.player.clone() else {
            return;
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            cx.background_executor()
                .timer(PLAYBACK_UI_POLL_INTERVAL)
                .await;
            let state = player.state();

            view.update(cx, move |this, cx| {
                let mut keep_polling = false;
                match state {
                    Ok(state) => {
                        this.playback_state = Some(state.clone());
                        if let Some(duration) = state.duration.filter(|duration| *duration > 0.0) {
                            let position = state.time_pos.unwrap_or_default();
                            let ratio = ((position / duration) as f32).clamp(0.0, 1.0);
                            let slider = this.seek_slider.clone();
                            if let Some(window_handle) = cx.active_window() {
                                let _ = cx.update_window(window_handle, |_, window, cx| {
                                    slider.update(cx, |slider, cx| {
                                        slider.set_value(ratio, window, cx)
                                    });
                                });
                            }
                        }
                        if state.stopped {
                            this.player = None;
                            this.playback_state = None;
                            this.page = if this.detail.is_some() {
                                Page::Detail
                            } else {
                                Page::Home
                            };
                        } else {
                            keep_polling = true;
                        }
                    }
                    Err(err) => {
                        this.error = Some(err);
                        keep_polling = this.player.is_some();
                    }
                }
                cx.notify();
                if keep_polling {
                    this.poll_playback_state(cx);
                }
            });
        })
        .detach();
    }

    pub(crate) fn toggle_pause(&self, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_ref() {
            let _ = player.toggle_pause();
        }
        cx.notify();
    }

    pub(crate) fn seek_by(&self, seconds: f64, cx: &mut Context<Self>) {
        let position = self
            .playback_state
            .as_ref()
            .and_then(|state| state.time_pos)
            .unwrap_or_default();
        if let Some(player) = self.player.as_ref() {
            let _ = player.seek((position + seconds).max(0.0));
        }
        cx.notify();
    }

    pub(crate) fn adjust_volume(&self, delta: i32, cx: &mut Context<Self>) {
        let volume = self
            .playback_state
            .as_ref()
            .and_then(|state| state.volume)
            .map(|volume| volume.round() as i32)
            .unwrap_or(100);
        if let Some(player) = self.player.as_ref() {
            let _ = player.set_volume((volume + delta).clamp(0, 100));
        }
        cx.notify();
    }

    pub(crate) fn seek_to_ratio(&self, ratio: f32, cx: &mut Context<Self>) {
        let duration = self
            .playback_state
            .as_ref()
            .and_then(|state| state.duration)
            .unwrap_or_default();
        if duration <= 0.0 {
            return;
        }
        if let Some(player) = self.player.as_ref() {
            let _ = player.seek(duration * ratio.clamp(0.0, 1.0) as f64);
        }
        cx.notify();
    }

    pub(crate) fn stop_playback(&mut self, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_ref() {
            let _ = player.stop();
        }
        self.player = None;
        self.playback_state = None;
        self.page = if self.detail.is_some() {
            Page::Detail
        } else {
            Page::Home
        };
        cx.notify();
    }
}
