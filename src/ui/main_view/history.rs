//! 播放历史页。
//!
//! 数据来自 `get_played_history`（`Filters=IsPlayed` + `SortBy=DatePlayed` 倒序，
//! 见 `LibraryService::played_history_page`）。布局：
//! 1. Fluent 列表行卡（`bg_card` 行卡、圆角 8px、1px 描边、内边距 12px、
//!    悬停 `bg_card_hover`）：缩略图 + 标题/副标题 + 右侧「上次播放 x」，
//!    点击进入详情；
//! 2. `has_more` 时显示「加载更多」，按 `start_index += limit` 追加。
//!
//! 内容只借用 `MainView::history`，渲染不复制条目数据。

use super::cards::{CardMetrics, card_ids, landscape_meta};
use super::components::{
    FluentButtonVariant, SPACING_L, SPACING_M, SPACING_S, SPACING_XL, SPACING_XS, fluent_button,
    fluent_empty_state,
};
use super::state::MainView;
use crate::models::MediaItem;
use gpui_kit::{
    AnyElement, Context, Image, ObjectFit, SharedString, div, img, px, prelude::*,
};
use std::sync::Arc;

/// 历史行缩略图尺寸（16:9）。
const THUMB_WIDTH: f32 = 160.0;
const THUMB_HEIGHT: f32 = 90.0;

impl MainView {
    /// 渲染播放历史页。
    pub(crate) fn history_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let mut root = div().w_full().flex().flex_col().gap(px(SPACING_XL));
        if let Some(banner) = self.page_error_banner() {
            root = root.child(banner);
        }

        let Some(history) = self.history.as_ref() else {
            if self.history_loading {
                return root.child(self.status_note("加载播放历史…")).into_any_element();
            }
            return root.child(history_empty_state(theme)).into_any_element();
        };

        if history.items.is_empty() {
            return root.child(history_empty_state(theme)).into_any_element();
        }

        root = root.child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(SPACING_S))
                .children(history.items.iter().map(|item| self.history_row(item, cx))),
        );

        let loaded = history.items.len() as u64;
        let total = history.total_count;
        if history.has_more {
            root = root
                .child(self.status_note(&format!("已加载 {loaded} / {total}")))
                .child(
                    div().flex().justify_center().child(fluent_button(
                        theme,
                        "history-load-more",
                        "加载更多",
                        FluentButtonVariant::Standard,
                        cx.listener(|view: &mut MainView, _event, _window, cx| {
                            view.load_history(false, cx);
                        }),
                    )),
                );
        } else {
            root = root.child(self.status_note(&format!("共 {total} 项")));
        }

        root.into_any_element()
    }

    /// 单条历史行卡：缩略图（160×90）+ 标题/副标题 + 右侧「上次播放 x」，
    /// 点击进入详情。闭包只捕获 `Arc<str>` id。
    fn history_row(&self, item: &MediaItem, cx: &mut Context<Self>) -> AnyElement {
        let metrics = CardMetrics::standard();
        let theme = self.theme();
        let image = self.card_artwork(item, cx);
        let title = SharedString::from(item.name.as_str());
        let subtitle = landscape_meta(item);
        let date_label = last_played_label(item);
        let (item_id, server_id) = card_ids(item);

        let thumb_block = div()
            .w(px(THUMB_WIDTH))
            .h(px(THUMB_HEIGHT))
            .flex_shrink_0()
            .rounded(px(metrics.radius))
            .overflow_hidden()
            .bg(theme.bg_control)
            .child(cover_block(image.as_ref()));

        let mut info = div()
            .flex()
            .flex_col()
            .gap(px(SPACING_XS))
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_primary)
                    .truncate()
                    .child(title),
            );
        if !subtitle.as_str().is_empty() {
            info = info.child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .truncate()
                    .child(subtitle),
            );
        }

        div()
            .id(SharedString::from(format!("history-row.{item_id}")))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACING_L))
            .p(px(SPACING_M))
            .bg(theme.bg_card)
            .rounded(px(metrics.radius))
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_card_hover))
            .child(thumb_block)
            .child(info)
            .children(date_label.map(|label| {
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(SharedString::from(label))
                    .into_any_element()
            }))
            .on_click(
                cx.listener(move |this: &mut MainView, _event, _window, cx| {
                    this.open_detail(Arc::clone(&item_id), Arc::clone(&server_id), cx);
                }),
            )
            .into_any_element()
    }
}

/// 历史页空状态。
fn history_empty_state(theme: crate::ui::theme::ThemeColors) -> AnyElement {
    fluent_empty_state(
        theme,
        "🕘",
        "暂无播放历史",
        "播放过的影片和剧集会出现在这里",
        None,
    )
}

/// 行内缩略图：命中缓存时展示图片，否则用控件底色占位。
fn cover_block(image: Option<&Arc<Image>>) -> AnyElement {
    match image {
        Some(image) => img(Arc::clone(image))
            .size_full()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div().size_full().into_any_element(),
    }
}

/// 「上次播放 2026-10-24」：`DatePlayed` 是 ISO 8601，取日期部分；非 ISO
/// 取值原样展示；缺失或空白时返回 `None`。
fn last_played_label(item: &MediaItem) -> Option<String> {
    let raw = item.last_played_date.as_deref()?.trim();
    if raw.is_empty() {
        return None;
    }
    let date = raw.split('T').next().unwrap_or(raw);
    Some(format!("上次播放 {date}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_with_date(date: Option<&str>) -> MediaItem {
        MediaItem {
            id: "item-1".to_string(),
            server_id: "server-1".to_string(),
            server_name: "Server".to_string(),
            name: "Episode".to_string(),
            item_type: "Episode".to_string(),
            year: None,
            overview: None,
            community_rating: None,
            run_time_ticks: None,
            playback_position_ticks: None,
            played_percentage: None,
            child_count: None,
            season_number: None,
            episode_number: None,
            series_name: None,
            series_id: None,
            season_name: None,
            season_id: None,
            genres: Vec::new(),
            official_rating: None,
            studios: Vec::new(),
            tags: Vec::new(),
            played: true,
            favorite: false,
            last_played_date: date.map(str::to_string),
            primary_image_url: None,
            backdrop_url: None,
            logo_url: None,
        }
    }

    #[test]
    fn last_played_label_keeps_only_the_date_part() {
        assert_eq!(
            last_played_label(&item_with_date(Some("2026-10-24T13:05:00.0000000Z"))).as_deref(),
            Some("上次播放 2026-10-24")
        );
    }

    #[test]
    fn last_played_label_passes_through_non_iso_values() {
        assert_eq!(
            last_played_label(&item_with_date(Some("2026-10-24"))).as_deref(),
            Some("上次播放 2026-10-24")
        );
    }

    #[test]
    fn last_played_label_is_none_without_a_date() {
        assert_eq!(last_played_label(&item_with_date(None)), None);
        assert_eq!(last_played_label(&item_with_date(Some("   "))), None);
    }
}
