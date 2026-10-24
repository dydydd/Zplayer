//! 播放器页。
//!
//! 播放面占满内容区：纯黑背景 + 居中的视频区域占位 + 顶部渐变叠加条 +
//! 底部半透明控制栏（rgba(0,0,0,0.8)）。当前 libmpv 只做解码播放，未把
//! 画面渲染进 GPUI，因此画面位置显示居中的媒体名与播放状态。
//!
//! 所有播放控制逻辑仍由 `playback.rs` 提供（`toggle_pause` / `seek_by` /
//! `adjust_volume` / `stop_playback` / seek 滑条事件订阅），本文件只负责
//! 呈现。事件闭包只捕获 `Entity<MainView>` 与 `Copy` 值，从不捕获
//! `PlayerState` 或 `String`；状态字段（f64 / bool）全部按值直接读取。

use super::state::{MainView, Page};
use crate::ui::theme::on_accent_text;
use gpui_kit::component::slider::Slider;
use gpui_kit::{AnyElement, ClickEvent, Context, FontWeight, Rgba, SharedString, Window};
use gpui_kit::{div, linear_color_stop, linear_gradient, prelude::*, px, rgb, rgba};

/// 播放面背景：纯黑。
const SURFACE_BG: u32 = 0x000000;
/// 顶部叠加条高度（逻辑像素）。
const TOP_BAR_HEIGHT: f32 = 64.;
/// 播放/暂停主控按钮尺寸。
const PRIMARY_BUTTON_SIZE: f32 = 48.;
/// 次级控制按钮（跳转、音量、停止）高度。
const CONTROL_BUTTON_HEIGHT: f32 = 36.;
/// 空状态图标圆直径。
const EMPTY_ICON_SIZE: f32 = 64.;
/// 音量百分比文本宽度，保证数字变化时控制行不抖动。
const VOLUME_LABEL_WIDTH: f32 = 44.;
/// 当前时间的等宽字体族（Windows 自带 Consolas）。
const MONO_FAMILY: &str = "Consolas";

impl MainView {
    /// 播放器页根节点：没有播放会话时退回空状态。
    pub(crate) fn player_content(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.player.is_none() {
            return self.player_empty_state(cx);
        }

        div()
            .id("page.player")
            .size_full()
            .overflow_hidden()
            .bg(rgb(SURFACE_BG))
            .child(self.player_stage())
            .child(self.player_top_bar(cx))
            .child(self.player_control_bar(cx))
            .into_any_element()
    }

    /// 无播放会话时的居中空状态：accent 圆内的播放符号 + 标题 + 描述 +
    /// 返回按钮。
    fn player_empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let back_view = cx.entity();
        div()
            .id("page.player")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(rgb(SURFACE_BG))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(EMPTY_ICON_SIZE))
                    .rounded_full()
                    .bg(theme.accent)
                    .text_2xl()
                    .text_color(on_accent_text())
                    .child("▶"),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(surface_fg())
                    .child("暂无播放内容"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(surface_fg_muted())
                    .child("从详情页选择影片或剧集开始播放"),
            )
            .child(
                // 播放面上的次级按钮形态：白色低透明底 + 描边。
                div()
                    .id("player.empty-back")
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(CONTROL_BUTTON_HEIGHT))
                    .px_5()
                    .rounded(px(6.))
                    .bg(surface_hover())
                    .border_1()
                    .border_color(surface_border())
                    .text_sm()
                    .text_color(surface_fg())
                    .cursor_pointer()
                    .hover(move |this| this.bg(rgba(0xFFFFFF29)))
                    .child("返回")
                    .on_click(move |_, _, cx| {
                        let _ = back_view.update(cx, |view, cx| view.leave_player(cx));
                    }),
            )
            .into_any_element()
    }

    /// 画面区域占位：居中的媒体名（text_2xl）与播放状态。
    fn player_stage(&self) -> AnyElement {
        // 状态字段全部是 `Copy`，按值读取即可，不做任何 clone。
        let paused = self
            .playback_state
            .as_ref()
            .map(|state| state.paused)
            .unwrap_or(false);

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(surface_fg())
                    .child(self.player_title()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(surface_fg_muted())
                    .child(if paused { "已暂停" } else { "播放中" }),
            )
            .into_any_element()
    }

    /// 顶部叠加条：左侧返回按钮（回详情），右侧媒体标题。
    fn player_top_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let back_view = cx.entity();
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(TOP_BAR_HEIGHT))
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            // 0° 朝上、顺时针增大：180° 即自上而下，由黑渐隐到透明。
            .bg(linear_gradient(
                180.,
                linear_color_stop(rgba(0x000000E6), 0.),
                linear_color_stop(rgba(0x00000000), 1.),
            ))
            .child(control_button(
                "player.back",
                "◀",
                surface_fg(),
                move |_, _, cx| {
                    let _ = back_view.update(cx, |view, cx| view.leave_player(cx));
                },
            ))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(surface_fg())
                    .child(self.player_title()),
            )
            .into_any_element()
    }

    /// 底部控制栏（rgba(0,0,0,0.8) 半透明）：进度行 + 控制行。
    fn player_control_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let state = self.playback_state.as_ref();

        // 全部为 `Copy` 状态字段：f64 / bool，直接按值读取。
        let position = state.and_then(|state| state.time_pos).unwrap_or_default();
        let duration = state.and_then(|state| state.duration).unwrap_or_default();
        let paused = state.map(|state| state.paused).unwrap_or(false);
        let volume = state
            .and_then(|state| state.volume)
            .map(|volume| volume.round() as i32)
            .unwrap_or(100);

        // 时间与音量只是展示用的短字符串，进入元素前一次性格式化。
        let position_label = SharedString::from(format_time(position));
        let duration_label = SharedString::from(if duration > 0.0 {
            format_time(duration)
        } else {
            "--:--:--".to_owned()
        });
        let volume_label = SharedString::from(format!("{volume}%"));

        // 进度滑条：`bg`/`text_color` 被 Slider 组件用作进度条与滑钮的
        // 颜色来源，因此这里传入 accent 令牌。
        let progress_row = div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_family(MONO_FAMILY)
                    .text_color(surface_fg_muted())
                    .child(position_label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        Slider::new(&self.seek_slider)
                            .flex_1()
                            .bg(theme.accent)
                            .text_color(surface_fg()),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .font_family(MONO_FAMILY)
                    .text_color(surface_fg_muted())
                    .child(duration_label),
            );

        let toggle_view = cx.entity();
        let seek_back_view = cx.entity();
        let seek_forward_view = cx.entity();
        let volume_down_view = cx.entity();
        let volume_up_view = cx.entity();
        let stop_view = cx.entity();

        let control_row = div()
            .flex()
            .items_center()
            .gap_2()
            // 播放 / 暂停：主按钮，品牌强调色。
            .child(
                div()
                    .id("player.toggle-pause")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(PRIMARY_BUTTON_SIZE))
                    .rounded_full()
                    .bg(theme.accent)
                    .cursor_pointer()
                    .hover(move |this| this.bg(theme.accent_hover))
                    .on_click(move |_, _, cx| {
                        let _ = toggle_view.update(cx, |view, cx| view.toggle_pause(cx));
                    })
                    .child(
                        div()
                            .text_lg()
                            .text_color(on_accent_text())
                            .child(if paused { "⏸" } else { "▶" }),
                    ),
            )
            // 快退 10 秒 / 快进 30 秒。
            .child(control_button(
                "player.seek-back",
                "-10s",
                surface_fg(),
                move |_, _, cx| {
                    let _ = seek_back_view.update(cx, |view, cx| view.seek_by(-10.0, cx));
                },
            ))
            .child(control_button(
                "player.seek-forward",
                "+30s",
                surface_fg(),
                move |_, _, cx| {
                    let _ = seek_forward_view.update(cx, |view, cx| view.seek_by(30.0, cx));
                },
            ))
            // 音量：静音图标 + 百分比，配合 −/＋ 复用 `adjust_volume`。
            .child(placeholder_chip(
                "player.volume-icon",
                if volume == 0 { "🔇" } else { "🔊" },
            ))
            .child(
                div()
                    .w(px(VOLUME_LABEL_WIDTH))
                    .text_sm()
                    .font_family(MONO_FAMILY)
                    .text_color(surface_fg())
                    .child(volume_label),
            )
            .child(control_button(
                "player.volume-down",
                "−",
                surface_fg(),
                move |_, _, cx| {
                    let _ = volume_down_view.update(cx, |view, cx| view.adjust_volume(-5, cx));
                },
            ))
            .child(control_button(
                "player.volume-up",
                "＋",
                surface_fg(),
                move |_, _, cx| {
                    let _ = volume_up_view.update(cx, |view, cx| view.adjust_volume(5, cx));
                },
            ))
            .child(div().flex_1())
            // 字幕 / 音轨 / 倍速占位（暂无功能）。
            .child(placeholder_chip("player.subtitles", "CC"))
            .child(placeholder_chip("player.audio-track", "♪"))
            .child(placeholder_chip("player.speed", "1x"))
            // 停止播放：复用现有 `stop_playback`。
            .child(control_button(
                "player.stop",
                "⏹",
                surface_fg(),
                move |_, _, cx| {
                    let _ = stop_view.update(cx, |view, cx| view.stop_playback(cx));
                },
            ));

        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            // 半透明黑（80%）。
            .bg(rgba(0x000000CC))
            .child(progress_row)
            .child(control_row)
            .into_any_element()
    }

    /// 媒体标题：优先取详情页条目名，没有详情时退回通用文案。
    fn player_title(&self) -> SharedString {
        SharedString::from(
            self.detail
                .as_ref()
                .map(|detail| detail.item.name.as_str())
                .unwrap_or("正在播放"),
        )
    }

    /// 离开播放器页：优先走导航历史（`go_back`）；播放由详情页直接进入
    /// 时历史可能为空，此时回落到详情页（没有详情则回首页）。
    fn leave_player(&mut self, cx: &mut Context<Self>) {
        self.go_back(cx);
        if self.page != Page::Player {
            return;
        }
        self.page = if self.detail.is_some() {
            Page::Detail
        } else {
            Page::Home
        };
        cx.notify();
    }
}

/// 播放面上的主文字颜色（播放面恒为黑色，故不随主题变化）。
fn surface_fg() -> Rgba {
    rgb(0xFFFFFF)
}

/// 播放面上的次级文字颜色。
fn surface_fg_muted() -> Rgba {
    rgb(0xB6BDC6)
}

/// 播放面上的悬停底色（白色 10%）。
fn surface_hover() -> Rgba {
    rgba(0xFFFFFF1A)
}

/// 播放面上的描边色（白色 16%）。
fn surface_border() -> Rgba {
    rgba(0xFFFFFF29)
}

/// 次级控制按钮：图标或短文本，悬停显示浅色底。
fn control_button(
    id: &'static str,
    label: &'static str,
    fg: Rgba,
    handler: impl Fn(&ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .h(px(CONTROL_BUTTON_HEIGHT))
        .px_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .text_color(fg)
        .hover(move |this| this.bg(surface_hover()))
        .on_click(handler)
        .child(label)
        .into_any_element()
}

/// 占位控件（字幕 / 音轨 / 倍速）：只呈现，不可点击。
fn placeholder_chip(id: &'static str, label: &'static str) -> AnyElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .h(px(CONTROL_BUTTON_HEIGHT))
        .px_2()
        .rounded_md()
        .text_sm()
        .text_color(surface_fg_muted())
        .child(label)
        .into_any_element()
}

/// 播放时间格式化：HH:MM:SS。
fn format_time(value: f64) -> String {
    let seconds = value.max(0.0) as u64;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}
