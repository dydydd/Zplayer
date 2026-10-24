//! 首页：未配置媒体服务时的空状态，以及配置好之后的滚动卡片行。
//!
//! 行顺序：继续观看 → 最新添加 → 媒体库 → 推荐电影 → 推荐剧集；空行
//! 整段跳过，保证标题下一定有卡片。行数据来自 `&HomePayload` 的借用
//! 切片，渲染不复制条目载荷；每张卡只有 `SharedString` 文本与激活闭包
//! 捕获的 `Arc<str>` id 会分配。

use super::cards::{CardMetrics, cover};
use super::components::{FluentButtonVariant, fluent_button};
use super::state::{MainView, Page};
use crate::models::MediaLibrary;
use crate::ui::theme::on_accent_text;
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, Rgba, SharedString, div,
    linear_color_stop, linear_gradient, px, rgb, rgba, prelude::*,
};
use std::sync::Arc;

/// 媒体库卡底部渐变 scrim 的高度。
const LIBRARY_SCRIM_HEIGHT: f32 = 56.0;
/// 空状态图标圆的直径。
const EMPTY_ICON_SIZE: f32 = 64.0;

impl MainView {
    /// 首页内容：加载提示 → 错误 → 空状态 → 卡片行。
    pub(crate) fn home_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        if self.loading {
            return centered_hint(SharedString::from("加载媒体库…"), theme.text_secondary);
        }
        if let Some(error) = self.error.as_deref() {
            return centered_hint(SharedString::from(error), theme.danger);
        }
        let Some(home) = self.home.as_ref() else {
            return self.empty_state(cx);
        };

        // 空行整段跳过：标题下永远有卡片。
        let mut content = div().w_full().flex().flex_col().gap_6();
        if !home.resume_items.is_empty() {
            content = content.child(self.resume_row("继续观看", &home.resume_items, cx));
        }
        if !home.latest.is_empty() {
            content = content.child(self.poster_row("最新添加", &home.latest, cx));
        }
        if !home.libraries.is_empty() {
            content = content.child(self.library_row(&home.libraries, &home.server.id, cx));
        }
        if !home.recommended_movies.is_empty() {
            content = content.child(self.poster_row("推荐电影", &home.recommended_movies, cx));
        }
        if !home.recommended_shows.is_empty() {
            content = content.child(self.poster_row("推荐剧集", &home.recommended_shows, cx));
        }
        content.into_any_element()
    }

    /// 媒体库行：每个库一张横版卡。库列表保持借用，直接从载荷切片渲染。
    fn library_row(
        &self,
        libraries: &[MediaLibrary],
        server_id: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.scroll_row(
            "媒体库",
            libraries
                .iter()
                .map(|library| self.library_card(library, server_id, cx)),
        )
    }

    /// 单个媒体库卡：横版封面 + 底部渐变 scrim 上叠库名。点击进入媒体库
    /// 浏览页；闭包只捕获 `Arc<str>` id。
    fn library_card(
        &self,
        library: &MediaLibrary,
        server_id: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let metrics = CardMetrics::standard();
        let theme = self.theme();
        let name = SharedString::from(library.name.as_str());
        let library_id: Arc<str> = Arc::from(library.id.as_str());
        let library_name: Arc<str> = Arc::from(library.name.as_str());

        let image = match self.images.get(library.id.as_str()) {
            Some(image) => Some(Arc::clone(image)),
            None => {
                if let Some(url) = library.image_url.as_deref() {
                    self.load_image(&library.id, server_id, url, cx);
                }
                None
            }
        };

        div()
            .id(SharedString::from(format!("home.library-card.{}", library.id)))
            .w(px(metrics.landscape_width))
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .child(
                div()
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
                    .child(
                        // 底部渐变 scrim：透明 → 半透明黑，库名叠在其上。
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .h(px(LIBRARY_SCRIM_HEIGHT))
                            .bg(linear_gradient(
                                180.0,
                                linear_color_stop(rgba(0x00000000), 0.0),
                                linear_color_stop(rgba(0x000000CC), 1.0),
                            ))
                            .flex()
                            .items_end()
                            .px_3()
                            .pb_2()
                            .child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xFFFFFF))
                                    .child(name),
                            ),
                    ),
            )
            .on_click(
                cx.listener(move |this: &mut MainView, _event, _window, cx| {
                    this.open_library(Arc::clone(&library_id), Arc::clone(&library_name), cx);
                }),
            )
            .into_any_element()
    }

    /// 空状态（未配置媒体服务）：accent 圆内的播放符号 + 标题 + 描述 +
    /// 「添加服务」主按钮（跳转服务器页）。
    fn empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let action = fluent_button(
            theme,
            "home.add-service",
            "＋ 添加服务",
            FluentButtonVariant::Accent,
            cx.listener(|this: &mut MainView, _event, _window, cx| {
                this.navigate(Page::Servers, cx);
            }),
        );

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                // 首页空状态的图标用 accent 圆 + 播放符号。
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
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child("没有媒体服务"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child("添加一个服务后，即可在这里浏览你的媒体库"),
            )
            .child(action)
            .into_any_element()
    }
}

/// 居中的一行状态提示（加载 / 错误）。
fn centered_hint(text: SharedString, color: Rgba) -> AnyElement {
    div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(color)
        .child(text)
        .into_any_element()
}
