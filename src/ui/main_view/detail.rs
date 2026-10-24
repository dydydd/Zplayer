//! 媒体详情页。
//!
//! 自上而下分区：
//! 1. 背景区（高 320 逻辑 px）：backdrop 图（无图用 `bg_card`）+ 底部渐变
//!    遮罩融入页面背景 + 左下 120×180 竖版海报与元数据列（标题 + 副标题行
//!    「年份 · 类型 · 评分 · 时长」）；
//! 2. 操作行：「播放」（accent 主按钮，触发 `resolve_playback`）+「收藏」
//!    「已看」（次按钮，乐观更新，见 `data.rs`）；
//! 3. 简介卡：「简介」+ `item.overview`（折叠时最多 [`SYNOPSIS_CLAMP_LINES`]
//!    行，溢出显示「展开」/「收起」）；
//! 4. 演员区（有 `people` 时）：96px 圆头像横向滚动行（点击进入人物页）；
//! 5. 季/集区（剧集且 `seasons` 非空）：季 pill 行 + 集列表行卡（缩略图 +
//!    「第 N 集 · 标题」+ 时长/简介 + 播放图标，点击整行播放该集）；
//! 6. 空态：`detail` 为 `None` 时显示「加载中...」或「加载失败：<原因>」。
//!
//! 渲染路径只借用 `&ItemDetailPayload`，不复制数据；点击闭包只捕获 `Entity`、
//! 条目 id 与 server id。图片统一走 `self.images` 缓存：详情主图复用 `item.id`
//! 键（首页卡片通常已缓存），backdrop 用 `{id}#backdrop`，人物用
//! `person:{id|name}`，剧集缩略图用剧集 id；未命中时触发 `load_image` 并以
//! 控件底色占位。

use super::components::{
    FluentButtonVariant, RADIUS_MD, RADIUS_SM, SPACING_L, SPACING_M, SPACING_S, SPACING_XL,
    SPACING_XS, fluent_button, fluent_pill, fluent_section_title,
};
use super::state::MainView;
use crate::models::{ItemDetailPayload, MediaItem, MediaPerson};
use crate::ui::theme::on_accent_text;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    AnyElement, Context, FontWeight, ObjectFit, SharedString, div, img, linear_color_stop,
    linear_gradient, px, rgba, prelude::*,
};
use std::sync::Arc;

/// 背景区高度（逻辑 px）。
const HERO_HEIGHT: f32 = 320.0;
/// 背景区底部渐变占背景区高度的比例（40%），保证叠加文字可读。
const HERO_FADE_RATIO: f32 = 0.4;
/// 竖版海报尺寸。
const POSTER_WIDTH: f32 = 120.0;
const POSTER_HEIGHT: f32 = 180.0;
/// 卡片圆角（borderRadiusLarge）。
const CARD_RADIUS: f32 = RADIUS_MD;
/// 操作行按钮高度（与 Fluent 按钮一致：32 逻辑 px，v9 compact）。
const ACTION_BUTTON_HEIGHT: f32 = 32.0;
/// 演员圆头像直径。
const AVATAR_SIZE: f32 = 96.0;
/// 演员横排最多展示数量，避免一次性触发过多图片请求。
const MAX_PEOPLE: usize = 30;
/// 剧集缩略图尺寸（16:9）。
const EPISODE_THUMB_WIDTH: f32 = 160.0;
const EPISODE_THUMB_HEIGHT: f32 = 90.0;
/// 集行右侧播放圆钮的直径。
const EPISODE_PLAY_SIZE: f32 = 40.0;
/// 简介折叠时显示的最大行数。
const SYNOPSIS_CLAMP_LINES: usize = 4;
/// 简介「展开」判定阈值（以半角字符宽为单位，见 [`synopsis_needs_expand`]）。
///
/// 按最窄的实用内容宽度标定：约 360 逻辑 px（≈700 px 窗口减去侧栏与内边距）时
/// `text_sm` 一行约 52 个单位，4 行约 208 个单位；取 180 更保守一些，宁可文字刚好
/// 放得下时多显示一个「展开」，也不要在窄窗口下把文字裁掉却没有展开入口。
const SYNOPSIS_OVERFLOW_UNITS: usize = 180;

/// `MediaItem.item_type` 的中文名称；未知类型不展示。
fn item_type_label(item_type: &str) -> Option<&'static str> {
    match item_type {
        "Movie" => Some("电影"),
        "Series" => Some("剧集"),
        "Season" => Some("季"),
        "Episode" => Some("单集"),
        "BoxSet" => Some("合集"),
        "Video" => Some("视频"),
        _ => None,
    }
}

/// 只有「剧集」类型才展示季/集区。
fn is_series(item: &MediaItem) -> bool {
    item.item_type == "Series"
}

/// `run_time_ticks`（100ns 单位）格式化为中文时长；缺失或为 0 时返回 `None`。
fn format_duration(ticks: Option<i64>) -> Option<String> {
    let minutes = ticks.filter(|ticks| *ticks > 0)? / 600_000_000;
    if minutes <= 0 {
        return None;
    }
    Some(if minutes >= 60 {
        let hours = minutes / 60;
        let rest = minutes % 60;
        if rest == 0 {
            format!("{hours} 小时")
        } else {
            format!("{hours} 小时 {rest} 分钟")
        }
    } else {
        format!("{minutes} 分钟")
    })
}

/// 简介是否需要「展开」。
///
/// `line_clamp` 只在渲染时裁剪，渲染路径拿不到裁剪结果，因此这里按字符显示宽度粗估
/// 行数：全角字符（CJK 等）记 2 个单位、ASCII 记 1 个，显式换行按整行推进，超过
/// [`SYNOPSIS_OVERFLOW_UNITS`] 个单位即认为 `SYNOPSIS_CLAMP_LINES` 行放不下。
/// 阈值取保守值：宁可在文字刚好放得下时多显示一个「展开」，也不要把文字藏起来
/// 而无法查看。
fn synopsis_needs_expand(text: &str) -> bool {
    const UNITS_PER_LINE: usize = SYNOPSIS_OVERFLOW_UNITS / SYNOPSIS_CLAMP_LINES;

    let mut units = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            units = (units / UNITS_PER_LINE + 1) * UNITS_PER_LINE;
        } else if ch.is_ascii() {
            units += 1;
        } else {
            units += 2;
        }
    }
    units > SYNOPSIS_OVERFLOW_UNITS
}

/// 副标题行：`年份 · 类型 · 评分 · 时长`，缺失项自动省略。
fn detail_meta_line(item: &MediaItem) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(year) = item.year {
        parts.push(year.to_string());
    }
    if let Some(label) = item_type_label(&item.item_type) {
        parts.push(label.to_string());
    }
    if !item.genres.is_empty() {
        parts.push(
            item.genres
                .iter()
                .take(2)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" / "),
        );
    }
    if let Some(rating) = item.community_rating {
        parts.push(format!("{rating:.1} 分"));
    }
    if let Some(duration) = format_duration(item.run_time_ticks) {
        parts.push(duration);
    }

    parts.join(" · ")
}

impl MainView {
    /// 渲染媒体详情页。
    pub(crate) fn detail_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(detail) = self.detail.as_ref() else {
            let message = match self.error.as_deref() {
                Some(error) => format!("加载失败：{error}"),
                None => "加载中...".to_string(),
            };
            return self.status_note(&message);
        };

        let mut root = div().flex().flex_col().gap_6().w_full();
        if let Some(banner) = self.page_error_banner() {
            root = root.child(banner);
        }
        root = root.child(self.detail_hero(detail, cx));
        root = root.child(self.detail_actions(detail, cx));
        root = root.child(self.detail_synopsis(detail, cx));

        if !detail.people.is_empty() {
            root = root.child(self.detail_people(detail, cx));
        }
        if is_series(&detail.item) {
            root = root.child(self.detail_episodes(detail, cx));
        }

        root.into_any_element()
    }

    /// 背景区：backdrop（无图则 `bg_card`）+ 底部渐变遮罩 + 左下海报与元数据。
    fn detail_hero(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let item = &detail.item;

        // backdrop 使用独立缓存键，避免与主图（`item.id`）互相覆盖。
        let backdrop_key = format!("{}#backdrop", item.id);
        let backdrop_image = self.images.get(&backdrop_key).cloned();
        if backdrop_image.is_none() {
            if let Some(url) = item.backdrop_url.as_ref() {
                self.load_image(&backdrop_key, &item.server_id, url, cx);
            }
        }

        // 竖版海报复用 `item.id` 键（首页卡片通常已经缓存过）。
        let poster_image = self.images.get(&item.id).cloned();
        if poster_image.is_none() {
            if let Some(url) = item.primary_image_url.as_ref() {
                self.load_image(&item.id, &item.server_id, url, cx);
            }
        }

        let backdrop_layer = match backdrop_image {
            Some(image) => img(image)
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => div().size_full().bg(theme.bg_card).into_any_element(),
        };

        // 底部 40% 渐变：透明 → 页面主背景色，保证叠加文字可读并与页面融合。
        let bottom_fade = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(HERO_HEIGHT * HERO_FADE_RATIO))
            .bg(linear_gradient(
                180.0,
                linear_color_stop(rgba(0x00000000), 0.0),
                linear_color_stop(theme.bg_main, 1.0),
            ));

        let poster_layer = div()
            .w(px(POSTER_WIDTH))
            .h(px(POSTER_HEIGHT))
            .flex_shrink_0()
            .rounded(px(CARD_RADIUS))
            .overflow_hidden()
            .bg(theme.bg_card)
            .border_1()
            .border_color(theme.border)
            .child(match poster_image {
                Some(image) => img(image)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                None => div().size_full().bg(theme.bg_card).into_any_element(),
            });

        let meta_line = detail_meta_line(item);
        let mut meta_column = div()
            .flex()
            .flex_col()
            .gap(px(SPACING_S))
            .min_w_0()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(SharedString::from(item.name.as_str())),
            );
        if !meta_line.is_empty() {
            meta_column = meta_column.child(
                div()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(SharedString::from(meta_line)),
            );
        }

        // 左下横排：竖版海报 + 元数据列（底部对齐）。
        let content = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .p_6()
            .flex()
            .flex_row()
            .items_end()
            .gap(px(SPACING_L))
            .child(poster_layer)
            .child(meta_column);

        div()
            .relative()
            .w_full()
            .h(px(HERO_HEIGHT))
            .rounded(px(CARD_RADIUS))
            .overflow_hidden()
            .bg(theme.bg_card)
            .child(backdrop_layer)
            .child(bottom_fade)
            .child(content)
            .into_any_element()
    }

    /// 操作行：「播放」（accent 主按钮）+「收藏」+「已看」（次按钮，点击后
    /// 乐观更新并后台调用 `set_favorite` / `set_played`）。
    fn detail_actions(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let is_favorite = self
            .detail_is_favorite
            .unwrap_or(detail.item.favorite);
        let is_played = self.detail_is_played.unwrap_or(detail.item.played);
        // 已经有解析好的播放会话时，主按钮变为「继续播放」，直接进入播放器；
        // 否则「播放」先解析播放源（解析成功后自动起播）。
        let has_session = self.resolved_session.is_some();
        let play_label = if has_session { "▶ 继续播放" } else { "▶ 播放" };

        let play_button = fluent_button(
            theme,
            "detail-play",
            play_label,
            FluentButtonVariant::Accent,
            cx.listener(|view: &mut MainView, _event, _window, cx| {
                if view.resolved_session.is_some() {
                    view.start_player(cx);
                } else {
                    view.resolve_playback(cx);
                }
            }),
        );

        // 「收藏」：次按钮形态；选中态用 accent 描边 + accent 文字。
        let favorite_label = if is_favorite { "已收藏" } else { "收藏" };
        let mut favorite_button = div()
            .id("detail-favorite")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(SPACING_S))
            .h(px(ACTION_BUTTON_HEIGHT))
            .px(px(SPACING_M))
            .rounded(px(RADIUS_SM))
            .bg(theme.bg_control)
            .border_1()
            .text_sm()
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_control_hover))
            .child(if is_favorite { "♥" } else { "♡" })
            .child(favorite_label)
            .on_click(cx.listener(|view: &mut MainView, _event, _window, cx| {
                view.toggle_detail_favorite(cx);
            }));
        favorite_button = if is_favorite {
            favorite_button
                .border_color(theme.accent)
                .text_color(theme.accent)
        } else {
            favorite_button
                .border_color(theme.border)
                .text_color(theme.text_primary)
        };

        // 「已看」：次按钮形态；已看时同样用 accent 描边强调。
        let mut played_button = div()
            .id("detail-played")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(SPACING_S))
            .h(px(ACTION_BUTTON_HEIGHT))
            .px(px(SPACING_M))
            .rounded(px(RADIUS_SM))
            .bg(theme.bg_control)
            .border_1()
            .text_sm()
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_control_hover))
            .child("✓")
            .child(if is_played { "已看" } else { "标记已看" })
            .on_click(cx.listener(|view: &mut MainView, _event, _window, cx| {
                view.toggle_detail_played(cx);
            }));
        played_button = if is_played {
            played_button
                .border_color(theme.accent)
                .text_color(theme.accent)
        } else {
            played_button
                .border_color(theme.border)
                .text_color(theme.text_primary)
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACING_M))
            .child(play_button)
            .child(favorite_button)
            .child(played_button)
            .into_any_element()
    }

    /// 简介卡：标题「简介」+ 多行简介文本（折叠时最多 [`SYNOPSIS_CLAMP_LINES`] 行）。
    fn detail_synopsis(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let expanded = self.detail_synopsis_expanded;
        let overview = detail
            .item
            .overview
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("暂无简介");

        let mut text_block = div()
            .w_full()
            .text_sm()
            .text_color(theme.text_secondary)
            .child(SharedString::from(overview));
        if !expanded {
            text_block = text_block.line_clamp(SYNOPSIS_CLAMP_LINES);
        }

        let mut card = div()
            .flex()
            .flex_col()
            .gap(px(SPACING_M))
            .w_full()
            .p(px(SPACING_XL))
            .bg(theme.bg_card)
            .rounded(px(CARD_RADIUS))
            .border_1()
            .border_color(theme.border)
            .child(fluent_section_title(theme, "简介"))
            .child(text_block);

        // 短简介不放「展开」；长简介折叠时显示「展开」，展开后显示「收起」。
        if expanded || synopsis_needs_expand(overview) {
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.accent)
                    .cursor_pointer()
                    .child(if expanded { "收起" } else { "展开" })
                    .id("detail-synopsis-toggle")
                    .on_click(cx.listener(|view: &mut MainView, _event, _window, cx| {
                        view.toggle_synopsis(cx);
                    })),
            );
        }

        card.into_any_element()
    }

    /// 演员区：标题「演员」+ 圆头像横向滚动行。
    fn detail_people(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let server_id = detail.item.server_id.as_str();
        let cards = detail
            .people
            .iter()
            .take(MAX_PEOPLE)
            .map(|person| self.person_card(person, server_id, cx))
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .gap(px(SPACING_L))
            .w_full()
            .child(fluent_section_title(self.theme, "演员"))
            .child(div().flex().flex_row().overflow_x_scrollbar().children(cards))
            .into_any_element()
    }

    /// 单个演员：圆头像（96px，无图用控件底色圆占位）+ 名字 + 角色。
    /// 有 `person.id` 时整卡可点击，进入人物页（闭包只捕获 `Arc<str>` id）。
    fn person_card(
        &self,
        person: &MediaPerson,
        server_id: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let person_id: Option<Arc<str>> = person.id.as_deref().map(Arc::from);
        let image_key = match person.id.as_deref() {
            Some(id) => format!("person:{id}"),
            None => format!("person:{}", person.name),
        };

        let image = self.images.get(&image_key).cloned();
        if image.is_none() {
            if let Some(url) = person.image_url.as_ref() {
                self.load_image(&image_key, server_id, url, cx);
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

        let mut card = div()
            .w(px(AVATAR_SIZE + 24.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(SPACING_S))
            .p(px(SPACING_S))
            .rounded(px(CARD_RADIUS))
            .mr(px(SPACING_L))
            .child(avatar)
            .child(
                div()
                    .text_sm()
                    .text_center()
                    .text_color(theme.text_primary)
                    .child(SharedString::from(person.name.as_str())),
            );

        if let Some(role) = person
            .role
            .as_deref()
            .map(str::trim)
            .filter(|role| !role.is_empty())
        {
            card = card.child(
                div()
                    .text_xs()
                    .text_center()
                    .text_color(theme.text_secondary)
                    .child(SharedString::from(role)),
            );
        }

        // 只有拿到人物 id 才可点击（否则服务器无法定位人物条目）。
        let Some(person_id) = person_id else {
            return card.into_any_element();
        };

        card.cursor_pointer()
            .hover(move |this| this.bg(theme.bg_card_hover))
            .id(SharedString::from(format!("person-card-{person_id}")))
            .on_click(cx.listener(
                move |view: &mut MainView, _event, _window, cx| {
                    view.open_person(Arc::clone(&person_id), cx);
                },
            ))
            .into_any_element()
    }

    /// 季/集区：季 pill 行 + 集列表行卡（加载更多复用 `data.rs` 的分页逻辑）。
    fn detail_episodes(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut section = div()
            .flex()
            .flex_col()
            .gap(px(SPACING_L))
            .w_full()
            .child(fluent_section_title(theme, "剧集"));

        if !detail.seasons.is_empty() {
            section = section.child(self.season_pills(detail, cx));
        }

        if self.season_loading {
            return section
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child("正在加载剧集..."),
                )
                .into_any_element();
        }

        if self.season_episodes.is_empty() {
            return section
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child("本季暂无剧集"),
                )
                .into_any_element();
        }

        let selected_episode_id = self.selected_episode_id.as_deref();
        let rows = self
            .season_episodes
            .iter()
            .enumerate()
            .map(|(index, episode)| {
                let selected = selected_episode_id == Some(episode.id.as_str());
                self.episode_row(episode, index, selected, cx)
            })
            .collect::<Vec<_>>();
        section = section.child(div().flex().flex_col().gap(px(SPACING_S)).children(rows));

        if self.season_has_more() {
            section = section.child(fluent_button(
                theme,
                "load-more-episodes",
                "加载更多",
                FluentButtonVariant::Standard,
                cx.listener(|view: &mut MainView, _event, _window, cx| {
                    view.load_more_episodes(cx);
                }),
            ));
        }

        section.into_any_element()
    }

    /// 季选择 pill 行：选中态 accent 填充 + `on_accent_text`，未选中
    /// `bg_control`；点击切换季并加载该季剧集。
    fn season_pills(&self, detail: &ItemDetailPayload, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        // 渲染路径：`Arc<str>` server id 以引用计数形式在所有 pill 间共享。
        let server_id: Arc<str> = Arc::from(detail.item.server_id.as_str());
        let mut row = div().flex().flex_row().gap(px(SPACING_S)).overflow_x_scrollbar();

        for season in &detail.seasons {
            let season_id: Arc<str> = Arc::from(season.id.as_str());
            // `server_id` 跨迭代共享，每个 pill 在 `move` 闭包捕获前克隆一次
            // 引用计数。
            let season_server_id = Arc::clone(&server_id);
            let selected = self.selected_season_id.as_deref() == Some(season.id.as_str());

            let pill = fluent_pill(
                theme,
                format!("season-pill-{}", season.id),
                season.name.as_str(),
                selected,
                cx.listener(
                    move |view: &mut MainView, _event, _window, cx| {
                        view.select_season(Arc::clone(&season_id), Arc::clone(&season_server_id), cx);
                    },
                ),
            );

            row = row.child(pill);
        }

        row.into_any_element()
    }

    /// 单集行卡：缩略图（160×90）+「第 N 集 · 标题」+ 时长/简介一行 +
    /// 右侧播放圆钮。行卡为 `bg_card` 底、圆角 8px、1px 描边、内边距 12px、
    /// 悬停 `bg_card_hover`。
    ///
    /// 整行可点击（含右侧播放圆钮区域）：鼠标事件默认向祖先冒泡，因此播放
    /// 圆钮只做展示与悬停反馈、不单独绑定点击，避免一次点击触发两次播放源
    /// 解析。
    fn episode_row(
        &self,
        episode: &MediaItem,
        index: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        // 渲染路径：捕获 `Arc<str>` id（各一次堆分配），不把 `String` 字段
        // 克隆进 `'static` 点击闭包。
        let episode_id: Arc<str> = Arc::from(episode.id.as_str());
        let server_id: Arc<str> = Arc::from(episode.server_id.as_str());

        let thumb = self.images.get(episode_id.as_ref()).cloned();
        if thumb.is_none() {
            if let Some(url) = episode.primary_image_url.as_ref() {
                self.load_image(episode_id.as_ref(), &episode.server_id, url, cx);
            }
        }

        let number = episode.episode_number.unwrap_or(index as i64 + 1);
        let title = format!("第 {number} 集 · {}", episode.name);

        // 第二行：时长 · 简介；两者都缺失时不渲染该行。
        let mut sub_line = format_duration(episode.run_time_ticks);
        if let Some(overview) = episode
            .overview
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            match sub_line.as_mut() {
                Some(line) => {
                    line.push_str(" · ");
                    line.push_str(overview);
                }
                None => sub_line = Some(overview.to_string()),
            }
        }

        let thumb_block = div()
            .w(px(EPISODE_THUMB_WIDTH))
            .h(px(EPISODE_THUMB_HEIGHT))
            .flex_shrink_0()
            .rounded(px(CARD_RADIUS))
            .overflow_hidden()
            .bg(theme.bg_control)
            .child(match thumb {
                Some(image) => img(image)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                None => div().size_full().bg(theme.bg_control).into_any_element(),
            });

        let mut meta = div()
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
                    .child(SharedString::from(title)),
            );
        if let Some(sub_line) = sub_line {
            meta = meta.child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .truncate()
                    .child(SharedString::from(sub_line)),
            );
        }

        // 播放闭包只捕获剧集 id 与 server id 的 `Arc<str>` 句柄（克隆引用
        // 计数，不复制字符串本体）；点击时才转成 `String` 传给播放解析。
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACING_L))
            .p(px(SPACING_M))
            .bg(theme.bg_card)
            .rounded(px(CARD_RADIUS))
            .border_1()
            .border_color(theme.border)
            .when(selected, |this| this.bg(theme.bg_card_hover))
            .cursor_pointer()
            .hover(move |this| this.bg(theme.bg_card_hover))
            .child(thumb_block)
            .child(meta)
            .child(
                div()
                    .size(px(EPISODE_PLAY_SIZE))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme.accent)
                    .text_color(on_accent_text())
                    .hover(move |this| this.bg(theme.accent_hover))
                    .child("▶")
                    .id(format!("episode-play-{episode_id}")),
            )
            .id(format!("episode-row-{episode_id}"))
            .on_click(cx.listener(
                move |view: &mut MainView, _event, _window, cx| {
                    view.play_episode(Arc::clone(&episode_id), Arc::clone(&server_id), cx);
                },
            ))
            .into_any_element()
    }

    /// 播放指定剧集：记录当前选中集并解析播放源（复用 `playback.rs` 的解析与起播）。
    pub(crate) fn play_episode(
        &mut self,
        episode_id: Arc<str>,
        server_id: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        // 状态字段存 `Arc<str>`（引用计数克隆），不复制 id 字符串。
        self.selected_episode_id = Some(Arc::clone(&episode_id));
        self.resolve_playback_for(episode_id.to_string(), server_id.to_string(), None, cx);
    }

    /// 切换简介的展开/折叠（折叠时最多 [`SYNOPSIS_CLAMP_LINES`] 行）。
    pub(crate) fn toggle_synopsis(&mut self, cx: &mut Context<Self>) {
        self.detail_synopsis_expanded = !self.detail_synopsis_expanded;
        cx.notify();
    }

    /// 切换到指定季：重置选中集并加载该季剧集（复用 `data.rs` 的加载逻辑）。
    pub(crate) fn select_season(
        &mut self,
        season_id: Arc<str>,
        server_id: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        if self.selected_season_id.as_deref() == Some(season_id.as_ref()) {
            return;
        }
        // 状态字段存 `Arc<str>`（引用计数克隆），不复制 id 字符串。
        self.selected_season_id = Some(Arc::clone(&season_id));
        self.selected_episode_id = None;
        self.load_season_episodes(season_id, server_id, cx);
    }

    /// 当前季是否还有未加载的剧集（`episode_total_count` 来自详情响应）。
    fn season_has_more(&self) -> bool {
        self.detail
            .as_ref()
            .and_then(|detail| detail.episode_total_count)
            .is_some_and(|total| self.season_episodes.len() < total)
    }
}
