//! Zplayer 卡片系统：竖版海报卡、横版卡、续看卡、封面角标与横向滚动行。
//!
//! 渲染路径零拷贝的三条约定：
//! * 卡片从借用 `&MediaItem` 构建，渲染一行卡片不会复制条目载荷；
//! * 图片按 `item.id` 在 `MainView::images` 中查找，只克隆 `Arc<Image>`
//!   句柄（解码后的图像始终共享）；
//! * 文本一律 `SharedString::from(&str)` 借用转换；激活闭包只捕获
//!   `Arc<str>` id 与 `Copy` 值，从不捕获 `String` 或 `MediaItem`。
//!
//! 通用 Fluent 组件（按钮 / 胶囊 / 空状态 / 区块标题）由 `components.rs`
//! 提供，本文件只负责媒体卡片本身。

use super::components::{RADIUS_MD, SPACING_M, SPACING_S, SPACING_XS, fluent_section_title};
use super::state::MainView;
use crate::models::MediaItem;
use crate::ui::theme::{ThemeColors, on_accent_text};
use gpui_kit::{
    AnyElement, Context, FontWeight, Image, InteractiveElement, ObjectFit, Rgba, SharedString,
    div, img, linear_color_stop, linear_gradient, px, rgb, rgba, prelude::*,
};
use std::sync::Arc;

/// Emby/Jellyfin tick 分辨率：每秒 10,000,000 tick。
const TICKS_PER_MINUTE: i64 = 10_000_000 * 60;
/// 未看数量徽标的上限，超过后显示 `99+`。
const COUNT_BADGE_LIMIT: i64 = 99;
/// 海报卡底部渐变 scrim 的高度。
const POSTER_SCRIM_HEIGHT: f32 = 64.0;

/// 卡片系统布局令牌：每行解析一次，逐卡按值复制（`Copy`）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct CardMetrics {
    /// 竖版海报宽度。
    pub(crate) poster_width: f32,
    /// 竖版海报高度。
    pub(crate) poster_height: f32,
    /// 横版封面宽度。
    pub(crate) landscape_width: f32,
    /// 横版封面高度。
    pub(crate) landscape_height: f32,
    /// 行内两张卡片的间距（spacingHorizontalM）。
    pub(crate) gap: f32,
    /// 卡片圆角（borderRadiusLarge）。
    pub(crate) radius: f32,
    /// 封面角标（评分 / 时长 / 数量）的高度。
    pub(crate) badge_height: f32,
    /// 横版卡底部播放进度条的高度（strokeWidthThin 的倍数）。
    pub(crate) progress_height: f32,
}

impl CardMetrics {
    pub(crate) const fn standard() -> Self {
        Self {
            poster_width: 160.0,
            poster_height: 240.0,
            landscape_width: 280.0,
            landscape_height: 158.0,
            gap: SPACING_M,
            radius: RADIUS_MD,
            badge_height: 22.0,
            progress_height: 2.0,
        }
    }
}

/// 封面角标（评分 / 时长）的半透明黑底：角标叠在未知亮度的图片上，
/// 因此用固定的半透明黑而不是主题色。
fn overlay_fill() -> Rgba {
    rgba(0x0000008C)
}

/// 角标文字色：白。
fn overlay_ink() -> Rgba {
    rgb(0xFFFFFF)
}

/// 海报 scrim 上的次级文字（元信息行）：70% 白。
fn scrim_ink_secondary() -> Rgba {
    rgba(0xFFFFFFB3)
}

/// 播放进度条的轨道底色。
fn progress_track() -> Rgba {
    rgba(0x00000059)
}

/// 海报 scrim 底部的实心端颜色：85% 黑。
fn poster_scrim_ink() -> Rgba {
    rgba(0x000000D9)
}

/// 卡片激活闭包捕获的 id：`Arc<str>` 句柄（克隆引用计数），不把 `String`
/// 字段复制进 `'static` 闭包。
pub(super) fn card_ids(item: &MediaItem) -> (Arc<str>, Arc<str>) {
    (
        Arc::from(item.id.as_str()),
        Arc::from(item.server_id.as_str()),
    )
}

/// 卡片封面：命中缓存时展示图片，否则用卡片底色占位。
pub(super) fn cover(image: Option<&Arc<Image>>, theme: ThemeColors) -> AnyElement {
    match image {
        Some(image) => img(Arc::clone(image))
            .size_full()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div().size_full().bg(theme.bg_card).into_any_element(),
    }
}

/// 圆角半透明黑胶囊角标（评分 / 时长）。
fn overlay_chip(text: SharedString, metrics: &CardMetrics) -> AnyElement {
    div()
        .flex()
        .items_center()
        .h(px(metrics.badge_height))
        .rounded_full()
        .bg(overlay_fill())
        .px(px(SPACING_S))
        .text_xs()
        .text_color(overlay_ink())
        .child(text)
        .into_any_element()
}

/// 评分角标：封面右上角的社区评分（如 `8.5`），半透明黑胶囊白字。
fn rating_slot(item: &MediaItem, metrics: &CardMetrics) -> Option<AnyElement> {
    let rating = item.community_rating?;
    Some(
        div()
            .absolute()
            .top_2()
            .right_2()
            .child(overlay_chip(
                SharedString::from(format!("{rating:.1}")),
                metrics,
            ))
            .into_any_element(),
    )
}

/// 未看数量角标：封面左上角，accent 填充 + `on_accent_text` 文字。
fn count_slot(item: &MediaItem, metrics: &CardMetrics, theme: ThemeColors) -> Option<AnyElement> {
    let remaining = unwatched_count(item)?;
    Some(
        div()
            .absolute()
            .top_2()
            .left_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(metrics.badge_height))
                    .rounded_full()
                    .bg(theme.accent)
                    .px(px(SPACING_S))
                    .text_xs()
                    .text_color(on_accent_text())
                    .child(SharedString::from(count_label(remaining))),
            )
            .into_any_element(),
    )
}

/// 时长角标：封面左下角（如 `1h 52m`）。
fn duration_slot(item: &MediaItem, metrics: &CardMetrics) -> Option<AnyElement> {
    let runtime = format_runtime(item.run_time_ticks?)?;
    Some(
        div()
            .absolute()
            .bottom_2()
            .left_2()
            .child(overlay_chip(SharedString::from(runtime), metrics))
            .into_any_element(),
    )
}

/// 剧集编号文案（如 `S1E2` / `E2`）。
fn episode_label(item: &MediaItem) -> Option<String> {
    match (item.season_number, item.episode_number) {
        (Some(season), Some(episode)) => Some(format!("S{season}E{episode}")),
        (None, Some(episode)) => Some(format!("E{episode}")),
        _ => None,
    }
}

/// 条目类型的中文标签。
fn kind_label(item: &MediaItem) -> Option<&'static str> {
    match item.item_type.as_str() {
        "Movie" => Some("电影"),
        "Series" => Some("剧集"),
        "Episode" => Some("剧集"),
        "Season" => Some("季"),
        "BoxSet" => Some("合集"),
        "Person" => Some("人物"),
        _ => None,
    }
}

/// 竖版海报的元信息行：年份 + 第一个类型，缺失时退回条目类型。
fn poster_meta(item: &MediaItem) -> SharedString {
    let mut meta = String::new();
    if let Some(year) = item.year {
        meta.push_str(&year.to_string());
    }
    if let Some(genre) = item.genres.first() {
        if !meta.is_empty() {
            meta.push_str(" · ");
        }
        meta.push_str(genre);
    }
    if meta.is_empty() {
        if let Some(label) = kind_label(item) {
            meta.push_str(label);
        }
    }
    SharedString::from(meta)
}

/// 横版卡的元信息行：剧集名 + 集编号；非剧集退回海报的年份/类型信息。
pub(super) fn landscape_meta(item: &MediaItem) -> SharedString {
    let mut meta = String::new();
    if let Some(series) = item.series_name.as_deref() {
        meta.push_str(series);
    }
    if let Some(episode) = episode_label(item) {
        if !meta.is_empty() {
            meta.push_str(" · ");
        }
        meta.push_str(&episode);
    }
    if meta.is_empty() {
        return poster_meta(item);
    }
    SharedString::from(meta)
}

/// `1h 52m` / `52m`，时长缺失或非正数时返回 `None`。
fn format_runtime(ticks: i64) -> Option<String> {
    if ticks <= 0 {
        return None;
    }
    let minutes = ticks / TICKS_PER_MINUTE;
    if minutes < 60 {
        Some(format!("{minutes}m"))
    } else {
        Some(format!("{}h {}m", minutes / 60, minutes % 60))
    }
}

/// 部分观看条目的剩余时长（如 `剩余 32m`）。
fn remaining_runtime(item: &MediaItem) -> Option<String> {
    let total = item.run_time_ticks?;
    let position = item.playback_position_ticks.unwrap_or(0);
    format_runtime(total - position).map(|left| format!("剩余 {left}"))
}

/// 播放进度（`0.0..=1.0`）：优先用位置/总时长，缺失时退回服务器上报的
/// 已看百分比，两者都没有则视为无进度。
fn playback_progress(item: &MediaItem) -> f32 {
    if let (Some(position), Some(duration)) = (item.playback_position_ticks, item.run_time_ticks) {
        if duration > 0 && position > 0 {
            return (position as f64 / duration as f64).clamp(0.0, 1.0) as f32;
        }
    }
    item.played_percentage
        .filter(|percent| *percent > 0.0)
        .map(|percent| (percent / 100.0).clamp(0.0, 1.0) as f32)
        .unwrap_or(0.0)
}

/// 条目仍未看的子条目数量（由子条目总数与已看百分比推导）；无子条目或
/// 全部已看时返回 `None`。
fn unwatched_count(item: &MediaItem) -> Option<i64> {
    let total = item.child_count?;
    if total <= 0 {
        return None;
    }
    let watched = item
        .played_percentage
        .map(|percent| (total as f64 * percent / 100.0).round() as i64)
        .unwrap_or(0);
    let remaining = total - watched;
    (remaining > 0).then_some(remaining)
}

/// 数量角标文案：超过上限后收敛为 `99+`，避免长剧集撑宽胶囊。
fn count_label(remaining: i64) -> String {
    if remaining > COUNT_BADGE_LIMIT {
        format!("{COUNT_BADGE_LIMIT}+")
    } else {
        remaining.to_string()
    }
}

/// 横版卡底部贴边的播放进度条：accent 填充，宽度等于已看比例。
fn progress_bar(progress: f32, metrics: &CardMetrics, theme: ThemeColors) -> AnyElement {
    let width = metrics.landscape_width * progress.clamp(0.0, 1.0);
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(px(metrics.progress_height))
        .bg(progress_track())
        .child(div().h_full().bg(theme.accent).w(px(width)))
        .into_any_element()
}

/// 海报卡底部渐变 scrim：透明 → 半透明黑，标题与元信息叠在其上。
fn poster_scrim(title: SharedString, meta: Option<SharedString>) -> AnyElement {
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(px(POSTER_SCRIM_HEIGHT))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgba(0x00000000), 0.0),
            linear_color_stop(poster_scrim_ink(), 1.0),
        ))
        .flex()
        .flex_col()
        .justify_end()
        .gap(px(SPACING_XS))
        .px(px(SPACING_S))
        .pb(px(SPACING_S))
        .child(
            div()
                .w_full()
                .truncate()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(overlay_ink())
                .child(title),
        )
        .children(meta.map(|meta| {
            div()
                .w_full()
                .truncate()
                .text_xs()
                .text_color(scrim_ink_secondary())
                .child(meta)
                .into_any_element()
        }))
        .into_any_element()
}

impl MainView {
    /// 条目的缓存图片，未命中时触发后台加载。
    ///
    /// 只克隆 `Arc<Image>` 句柄；解码后的图像始终共享。未命中时封面以
    /// `theme.bg_card` 占位（见 [`cover`]），加载完成后插入缓存并通知重绘。
    pub(crate) fn card_artwork(&self, item: &MediaItem, cx: &mut Context<Self>) -> Option<Arc<Image>> {
        if let Some(image) = self.images.get(item.id.as_str()) {
            return Some(Arc::clone(image));
        }
        if let Some(url) = item.primary_image_url.as_deref() {
            self.load_image(&item.id, &item.server_id, url, cx);
        }
        None
    }

    /// 竖版海报卡（160×240）：封面 + 角标 + 底部渐变 scrim 上叠标题与
    /// 元信息；1px 描边，悬停时描边转 accent。
    pub(crate) fn poster_card(&self, item: &MediaItem, cx: &mut Context<Self>) -> AnyElement {
        let metrics = CardMetrics::standard();
        let theme = self.theme();
        let image = self.card_artwork(item, cx);
        let title = SharedString::from(item.name.as_str());
        let meta = poster_meta(item);
        let meta = (!meta.as_str().is_empty()).then_some(meta);
        let (item_id, server_id) = card_ids(item);

        div()
            .id(SharedString::from(format!("home.poster-card.{item_id}")))
            .relative()
            .w(px(metrics.poster_width))
            .h(px(metrics.poster_height))
            .bg(theme.bg_card)
            .rounded(px(metrics.radius))
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_card_hover).border_color(theme.accent))
            .child(cover(image.as_ref(), theme))
            .children(count_slot(item, &metrics, theme))
            .children(rating_slot(item, &metrics))
            .children(duration_slot(item, &metrics))
            .child(poster_scrim(title, meta))
            .on_click(
                cx.listener(move |this: &mut MainView, _event, _window, cx| {
                    this.open_detail(Arc::clone(&item_id), Arc::clone(&server_id), cx);
                }),
            )
            .into_any_element()
    }

    /// 续看卡（横版卡 280×158）：封面 + 角标 + accent 进度条（2px，
    /// strokeWidthThin 的倍数）+ 剩余时长副标题，标题与元信息在封面下方。
    pub(crate) fn resume_card(&self, item: &MediaItem, cx: &mut Context<Self>) -> AnyElement {
        let metrics = CardMetrics::standard();
        let theme = self.theme();
        let image = self.card_artwork(item, cx);
        let title = SharedString::from(item.name.as_str());
        let subtitle =
            remaining_runtime(item).map_or_else(|| landscape_meta(item), SharedString::from);
        let progress = playback_progress(item);
        let (item_id, server_id) = card_ids(item);

        let cover_block = div()
            .id(SharedString::from(format!(
                "home.landscape-cover.{item_id}"
            )))
            .w(px(metrics.landscape_width))
            .h(px(metrics.landscape_height))
            .bg(theme.bg_card)
            .rounded(px(metrics.radius))
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .relative()
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_card_hover).border_color(theme.accent))
            .child(cover(image.as_ref(), theme))
            .children(count_slot(item, &metrics, theme))
            .children(rating_slot(item, &metrics))
            .children(duration_slot(item, &metrics))
            .child(progress_bar(progress, &metrics, theme));

        div()
            .id(SharedString::from(format!("home.resume-card.{item_id}")))
            .flex()
            .flex_col()
            .gap(px(SPACING_XS))
            .w(px(metrics.landscape_width))
            .cursor_pointer()
            .child(cover_block)
            .child(
                div()
                    .w_full()
                    .truncate()
                    .text_sm()
                    .text_color(theme.text_primary)
                    .child(title),
            )
            .child(
                div()
                    .w_full()
                    .truncate()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(subtitle),
            )
            .on_click(
                cx.listener(move |this: &mut MainView, _event, _window, cx| {
                    this.open_detail(Arc::clone(&item_id), Arc::clone(&server_id), cx);
                }),
            )
            .into_any_element()
    }

    /// 横向卡片行：`text_xl` 半粗体区块标题 + 不换行的横向滚动行。卡片
    /// 保持自身宽度（`flex_shrink_0`），间距为 [`CardMetrics::gap`]。
    pub(crate) fn scroll_row(
        &self,
        title: &str,
        cards: impl IntoIterator<Item = AnyElement>,
    ) -> AnyElement {
        let metrics = CardMetrics::standard();
        let theme = self.theme();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(SPACING_M))
            .child(fluent_section_title(theme, title))
            .child(
                div()
                    .id(SharedString::from(format!("home.row.{title}")))
                    .w_full()
                    .flex()
                    .flex_row()
                    .overflow_x_scroll()
                    .children(
                        cards
                            .into_iter()
                            .map(|card| div().flex_shrink_0().mr(px(metrics.gap)).child(card)),
                    ),
            )
            .into_any_element()
    }

    /// 用 `render` 把 `items` 渲染成一条横向滚动行。条目保持借用：迭代器
    /// 逐个把 `&MediaItem` 交给 `render`。
    fn card_row(
        &self,
        title: &str,
        items: &[MediaItem],
        cx: &mut Context<Self>,
        mut render: impl FnMut(&Self, &MediaItem, &mut Context<Self>) -> AnyElement,
    ) -> AnyElement {
        let cards = items.iter().map(|item| render(self, item, cx));
        self.scroll_row(title, cards)
    }

    /// 竖版海报卡行。
    pub(crate) fn poster_row(
        &self,
        title: &str,
        items: &[MediaItem],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.card_row(title, items, cx, |view, item, cx| {
            view.poster_card(item, cx)
        })
    }

    /// 续看卡行。
    pub(crate) fn resume_row(
        &self,
        title: &str,
        items: &[MediaItem],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.card_row(title, items, cx, |view, item, cx| view.resume_card(item, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::{count_label, format_runtime};

    /// 每分钟对应的 Emby/Jellyfin tick 数（每秒 10,000,000）。
    fn minutes(value: i64) -> i64 {
        value * 60 * 10_000_000
    }

    #[test]
    fn runtime_chip_formats_hours_and_minutes() {
        assert_eq!(format_runtime(minutes(112)).as_deref(), Some("1h 52m"));
        assert_eq!(format_runtime(minutes(60)).as_deref(), Some("1h 0m"));
        assert_eq!(format_runtime(minutes(52)).as_deref(), Some("52m"));
        assert_eq!(format_runtime(minutes(0)), None);
        assert_eq!(format_runtime(-1), None);
    }

    #[test]
    fn count_badge_caps_long_series() {
        assert_eq!(count_label(1), "1");
        assert_eq!(count_label(99), "99");
        assert_eq!(count_label(100), "99+");
    }
}
