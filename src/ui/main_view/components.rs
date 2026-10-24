//! Fluent 风格可复用组件：卡片容器、按钮、胶囊选择器、图标按钮、区块
//! 标题、空状态。
//!
//! 全部为独立函数，接收 `ThemeColors`（`Copy`）与 `&str` 借用参数，不读取
//! 全局状态、不复制调用方数据。交互回调签名与 gpui-kit `Button::on_click`
//! 保持一致（`Fn(&ClickEvent, &mut Window, &mut App)`），调用方用
//! `cx.listener(...)` 生成，闭包内只捕获 `Entity` 与 `Copy` 数据。
//!
//! 尺寸约定（FluentUI v9 组件规范，值见 `docs/fluent-tokens.md`）：卡片
//! 圆角 8px（borderRadiusLarge）/ 内边距 16px（spacingHorizontalL）；按钮
//! 高 32px（v9 compact）/ 圆角 4px（borderRadiusMedium）/ 水平内边距
//! 12px（spacingHorizontalM）/ 字号 14（Base300）；胶囊全圆角 / 高 28px /
//! 水平内边距 12px；图标按钮 36×36（v9 icon button compact）。
//!
//! 媒体卡片（海报卡 / 横版卡 / 续看卡 / 横向滚动行）由 `cards.rs` 提供。
//! 旧组件（`error_banner` / `page_error_banner` / `status_note` /
//! `selectable_pill`）保留原签名供既有页面使用，其中 `selectable_pill`
//! 的视觉由 [`fluent_pill`] 提供。

use super::state::MainView;
use crate::ui::theme::{ThemeColors, on_accent_text};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Div, ElementId, FontWeight, SharedString, Window, div,
    px, prelude::*,
};

/// spacingHorizontalXS（4 逻辑 px）：紧凑间隙。
pub(crate) const SPACING_XS: f32 = 4.0;
/// spacingHorizontalS（8 逻辑 px）：卡片内元素间隙。
pub(crate) const SPACING_S: f32 = 8.0;
/// spacingHorizontalM（12 逻辑 px）：列表行内边距 / 按钮水平内边距。
pub(crate) const SPACING_M: f32 = 12.0;
/// spacingHorizontalL（16 逻辑 px）：卡片内边距。
pub(crate) const SPACING_L: f32 = 16.0;
/// spacingHorizontalXL（20 逻辑 px）：页面区块间距。
pub(crate) const SPACING_XL: f32 = 20.0;

/// borderRadiusMedium（4 逻辑 px）：按钮 / 输入 / 导航行圆角。
pub(crate) const RADIUS_SM: f32 = 4.0;
/// borderRadiusLarge（8 逻辑 px）：卡片圆角。
pub(crate) const RADIUS_MD: f32 = 8.0;
/// borderRadiusXLarge（12 逻辑 px）：大面板 / 弹层圆角。
pub(crate) const RADIUS_LG: f32 = 12.0;

/// 按钮高度（32 逻辑 px，v9 compact）。
const BUTTON_HEIGHT: f32 = 32.0;
/// 胶囊高度（28 逻辑 px）。
const PILL_HEIGHT: f32 = 28.0;
/// 图标按钮边长（36 逻辑 px，v9 icon button compact）。
const ICON_BUTTON_SIZE: f32 = 36.0;
/// 空状态图标区直径（56 逻辑 px）。
const EMPTY_ICON_SIZE: f32 = 56.0;
/// 空状态图标字号（28 逻辑 px）。
const EMPTY_ICON_GLYPH_SIZE: f32 = 28.0;
/// 空状态描述最大宽度（320 逻辑 px），避免长描述一行拉满。
const EMPTY_DESCRIPTION_MAX_WIDTH: f32 = 320.0;

/// Fluent 按钮变体。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FluentButtonVariant {
    /// 品牌绿填充 + 深色文字（页面主操作：播放、确认）。
    Accent,
    /// 控件底色 + 1px 描边（常规操作）。
    Standard,
    /// 透明底（次级 / 工具栏操作）。
    Subtle,
}

/// Fluent 卡片容器：卡片底色、8px 圆角（borderRadiusLarge）、1px 描边
/// （strokeWidthThin）、16px 内边距（spacingHorizontalL），hover 提亮为
/// `bg_card_hover`。返回 [`Div`] 供调用方继续追加内容。
pub(crate) fn fluent_card(theme: ThemeColors) -> Div {
    div()
        .bg(theme.bg_card)
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(theme.border)
        .p(px(SPACING_L))
        .hover(move |this| this.bg(theme.bg_card_hover))
}

/// Fluent 按钮：高 32px（v9 compact）、圆角 4px（borderRadiusMedium）、
/// 水平内边距 12px（spacingHorizontalM）、字号 14（Base300）。
///
/// - [`FluentButtonVariant::Accent`]：accent 填充 + 深色文字，hover 用
///   `accent_hover`，按下用 `accent_pressed`；
/// - [`FluentButtonVariant::Standard`]：`bg_control` 填充 + 1px 描边，hover
///   用 `bg_control_hover`；
/// - [`FluentButtonVariant::Subtle`]：透明底，hover 用 `bg_card_hover`。
///
/// `on_click` 与 gpui-kit `Button::on_click` 签名一致，调用方用
/// `cx.listener(...)` 生成。
pub(crate) fn fluent_button(
    theme: ThemeColors,
    id: impl Into<ElementId>,
    label: &str,
    variant: FluentButtonVariant,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let button = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .h(px(BUTTON_HEIGHT))
        .px(px(SPACING_M))
        .rounded(px(RADIUS_SM))
        .cursor_pointer()
        .text_sm()
        .on_click(on_click);

    let button = match variant {
        FluentButtonVariant::Accent => button
            .bg(theme.accent)
            .text_color(on_accent_text())
            .font_weight(FontWeight::SEMIBOLD)
            .hover(move |this| this.bg(theme.accent_hover))
            .active(move |this| this.bg(theme.accent_pressed)),
        FluentButtonVariant::Standard => button
            .bg(theme.bg_control)
            .text_color(theme.text_primary)
            .border_1()
            .border_color(theme.border)
            .hover(move |this| this.bg(theme.bg_control_hover)),
        FluentButtonVariant::Subtle => button
            .text_color(theme.text_primary)
            .hover(move |this| this.bg(theme.bg_card_hover)),
    };

    button.child(SharedString::from(label)).into_any_element()
}

/// Fluent 胶囊选择器（设置模块条 / 季选择 / 排序条共用）：高 28px、水平
/// 内边距 12px（spacingHorizontalM）、全圆角。选中态 accent 填充 + 深色文字
/// （hover 提亮为 `accent_hover`），未选中态 `bg_control` + 主文字色（hover
/// 提亮为 `bg_control_hover`）。
pub(crate) fn fluent_pill(
    theme: ThemeColors,
    id: impl Into<ElementId>,
    label: &str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let pill = div()
        .id(id)
        .flex()
        .items_center()
        .h(px(PILL_HEIGHT))
        .px(px(SPACING_M))
        .rounded_full()
        .cursor_pointer()
        .text_sm()
        .on_click(on_click);

    let pill = if selected {
        pill.bg(theme.accent)
            .text_color(on_accent_text())
            .font_weight(FontWeight::SEMIBOLD)
            .hover(move |this| this.bg(theme.accent_hover))
    } else {
        pill.bg(theme.bg_control)
            .text_color(theme.text_primary)
            .hover(move |this| this.bg(theme.bg_control_hover))
    };

    pill.child(SharedString::from(label)).into_any_element()
}

/// Fluent 图标按钮（36×36，v9 icon button compact，Subtle 风格）：文字符号
/// 作 glyph，hover 提亮为 `bg_card_hover`。`tooltip` 悬停时以 GPUI 原生
/// tooltip 浮层显示，同时作为无障碍名称暴露。
pub(crate) fn fluent_icon_button(
    theme: ThemeColors,
    id: impl Into<ElementId>,
    glyph: &str,
    tooltip: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let tooltip_text = SharedString::from(tooltip);
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(ICON_BUTTON_SIZE))
        .rounded(px(RADIUS_SM))
        .cursor_pointer()
        .text_lg()
        .text_color(theme.text_primary)
        .hover(move |this| this.bg(theme.bg_card_hover))
        .aria_label(tooltip_text.clone())
        .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
        .on_click(on_click)
        .child(SharedString::from(glyph))
        .into_any_element()
}

/// Fluent 区块标题：`text_xl` 半粗体 + 主文字色。
pub(crate) fn fluent_section_title(theme: ThemeColors, text: &str) -> Div {
    div()
        .text_xl()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_primary)
        .child(SharedString::from(text))
}

/// Fluent 空状态：居中的大图标区（56px 圆形控件底 + 28px 符号）+ 标题
/// （20px，Base500）+ 描述（14px，Base300，最宽 320px）+ 可选动作按钮。
/// `action` 由调用方用 [`fluent_button`] 构建后传入（传 `None` 则不显示
/// 动作区）。
pub(crate) fn fluent_empty_state(
    theme: ThemeColors,
    icon_glyph: &str,
    title: &str,
    description: &str,
    action: Option<AnyElement>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACING_M))
        .w_full()
        .py_12()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(EMPTY_ICON_SIZE))
                .rounded_full()
                .bg(theme.bg_control)
                .child(
                    div()
                        .text_size(px(EMPTY_ICON_GLYPH_SIZE))
                        .text_color(theme.text_secondary)
                        .child(SharedString::from(icon_glyph)),
                ),
        )
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(SharedString::from(title)),
        )
        .child(
            div()
                .text_sm()
                .text_center()
                .max_w(px(EMPTY_DESCRIPTION_MAX_WIDTH))
                .text_color(theme.text_secondary)
                .child(SharedString::from(description)),
        )
        .when_some(action, |this, action| {
            this.mt(px(SPACING_S)).child(action)
        })
        .into_any_element()
}

impl MainView {
    /// 操作失败提示条：详情页收藏/已看等后台调用失败时显示（`self.error`），
    /// 成功后由调用方清空 `error` 自动消失。渲染只借用错误文本。
    pub(crate) fn error_banner(&self, message: &str) -> AnyElement {
        let theme = self.theme();
        div()
            .id("error-banner")
            .w_full()
            .rounded(px(RADIUS_MD))
            .bg(theme.bg_card)
            .border_1()
            .border_color(theme.danger)
            .px(px(SPACING_L))
            .py(px(SPACING_S))
            .text_sm()
            .text_color(theme.danger)
            .child(SharedString::from(message))
            .into_any_element()
    }

    /// 页面级错误条：`self.error` 有值时返回提示条，否则返回 `None`。
    pub(crate) fn page_error_banner(&self) -> Option<AnyElement> {
        self.error
            .as_deref()
            .map(|message| self.error_banner(message))
    }

    /// 居中的一行状态提示（加载中 / 空态 / 无更多数据）：字号 14
    /// （Base300）的次级文字。
    pub(crate) fn status_note(&self, text: &str) -> AnyElement {
        let theme = self.theme();
        div()
            .w_full()
            .py_8()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.text_secondary)
            .child(SharedString::from(text))
            .into_any_element()
    }

    /// 可选中的胶囊按钮（排序 pill / 分类 pill 共用）：视觉由 [`fluent_pill`]
    /// 提供。`on_click` 只接收 `MainView` 与 `Context`，调用方通常传入捕获
    /// `Copy` 枚举的闭包。
    pub(crate) fn selectable_pill(
        &self,
        id: SharedString,
        label: &str,
        selected: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut MainView, &mut Context<MainView>) + 'static,
    ) -> AnyElement {
        fluent_pill(
            self.theme(),
            id,
            label,
            selected,
            cx.listener(move |view: &mut MainView, _event, _window, cx| on_click(view, cx)),
        )
    }
}
