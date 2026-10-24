//! 人物页（演员详情）。
//!
//! 布局（自上而下）：
//! 1. 人物头卡：120px 圆头像 + 名字（`text_2xl` 粗体）+ 简介（`person.overview`，
//!    有则显示）；
//! 2. 「出演作品」：`poster_card` 横向滚动行（复用 `cards.rs`，点击进入作品
//!    详情）；
//! 3. 空态/加载态：`person` 为 `None` 时显示「加载人物资料…」或错误提示。
//!
//! 数据由 `data.rs::load_person` 并行拉取（`person_detail` + `filmography`）后
//! 一次性写入 `MainView::person`；渲染只借用 `&PersonPayload`，头像复用详情页
//! 演员卡使用的 `person:{id}` 图片缓存键（通常已经命中）。

use super::components::{RADIUS_MD, SPACING_S, SPACING_XL};
use super::state::MainView;
use crate::models::PersonPayload;
use gpui_kit::{
    AnyElement, Context, FontWeight, ObjectFit, SharedString, div, img, px, prelude::*,
};

/// 人物头像直径（逻辑 px）。
const AVATAR_SIZE: f32 = 120.0;
/// 人物头卡圆角（borderRadiusLarge）。
const CARD_RADIUS: f32 = RADIUS_MD;
/// 简介最大行宽，避免超长简介铺满整屏。
const OVERVIEW_MAX_WIDTH: f32 = 720.0;

impl MainView {
    /// 渲染人物页。
    pub(crate) fn person_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut root = div().w_full().flex().flex_col().gap_6();
        if let Some(banner) = self.page_error_banner() {
            root = root.child(banner);
        }

        let Some(person) = self.person.as_ref() else {
            let message = if self.person_loading {
                "加载人物资料…"
            } else {
                "暂无人物资料"
            };
            return root.child(self.status_note(message)).into_any_element();
        };

        root = root.child(self.person_header(person, cx));
        if person.filmography.is_empty() {
            root = root.child(self.status_note("暂无出演作品"));
        } else {
            root = root.child(self.poster_row("出演作品", &person.filmography, cx));
        }
        root.into_any_element()
    }

    /// 人物头卡：圆头像（无图用控件底色圆占位）+ 名字 + 简介，整体放在
    /// `bg_card` 行卡内。
    fn person_header(&self, payload: &PersonPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let person = &payload.person;

        // 与详情页演员卡共用缓存键，进入人物页时通常已命中。
        let image_key = format!("person:{}", person.id);
        let image = self.images.get(&image_key).cloned();
        if image.is_none() {
            if let Some(url) = person.primary_image_url.as_deref() {
                self.load_image(&image_key, &person.server_id, url, cx);
            }
        }

        let avatar = div()
            .w(px(AVATAR_SIZE))
            .h(px(AVATAR_SIZE))
            .flex_shrink_0()
            .rounded_full()
            .overflow_hidden()
            .bg(theme.bg_control)
            .child(match image {
                Some(image) => img(image)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                None => div().size_full().bg(theme.bg_control).into_any_element(),
            });

        let mut details = div()
            .flex()
            .flex_col()
            .gap(px(SPACING_S))
            .min_w_0()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(SharedString::from(person.name.as_str())),
            );

        if let Some(overview) = person
            .overview
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            details = details.child(
                div()
                    .w_full()
                    .max_w(px(OVERVIEW_MAX_WIDTH))
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(SharedString::from(overview)),
            );
        }

        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACING_XL))
            .bg(theme.bg_card)
            .rounded(px(CARD_RADIUS))
            .border_1()
            .border_color(theme.border)
            .p(px(SPACING_XL))
            .child(avatar)
            .child(details)
            .into_any_element()
    }
}
