//! Fluent 风格可复用组件：卡片容器、按钮、胶囊选择器、图标按钮、区块
//! 标题、空状态。
//!
//! 全部为独立函数，接收 `ThemeColors`（`Copy`）与 `&str` 借用参数，不读取
//! 全局状态、不复制调用方数据。交互回调签名与 gpui-kit `Button::on_click`
//! 保持一致（`Fn(&ClickEvent, &mut Window, &mut App)`），调用方用
//! `cx.listener(...)` 生成，闭包内只捕获 `Entity` 与 `Copy` 数据。
//!
//! 尺寸约定（Windows 11 Fluent）：卡片圆角 8px / 内边距 16px；按钮圆角
//! 6px / 高 36px / 水平内边距 16px；胶囊全圆角 / 高 32px / 水平内边距
//! 12px；图标按钮 40×40。
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

/// 按钮高度（36 逻辑 px）。
const BUTTON_HEIGHT: f32 = 36.0;
/// 按钮水平内边距（16 逻辑 px）。
const BUTTON_PADDING_X: f32 = 16.0;
/// 胶囊高度（32 逻辑 px）。
const PILL_HEIGHT: f32 = 32.0;
/// 胶囊水平内边距（12 逻辑 px）。
const PILL_PADDING_X: f32 = 12.0;
/// 图标按钮边长（40 逻辑 px）。
const ICON_BUTTON_SIZE: f32 = 40.0;
/// 空状态图标区直径（64 逻辑 px）。
const EMPTY_ICON_SIZE: f32 = 64.0;
/// 空状态图标字号（32 逻辑 px）。
const EMPTY_ICON_GLYPH_SIZE: f32 = 32.0;
/// 空状态描述最大宽度（360 逻辑 px），避免长描述一行拉满。
const EMPTY_DESCRIPTION_MAX_WIDTH: f32 = 360.0;

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

/// Fluent 卡片容器：卡片底色、8px 圆角、1px 描边、16px 内边距，hover 提亮
/// 为 `bg_card_hover`。返回 [`Div`] 供调用方继续追加内容。
pub(crate) fn fluent_card(theme: ThemeColors) -> Div {
    div()
        .bg(theme.bg_card)
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .p_4()
        .hover(move |this| this.bg(theme.bg_card_hover))
}

/// Fluent 按钮：高 36px、圆角 6px、水平内边距 16px。
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
        .px(px(BUTTON_PADDING_X))
        .rounded_md()
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

/// Fluent 胶囊选择器（设置模块条 / 季选择 / 排序条共用）：高 32px、水平
/// 内边距 12px、全圆角。选中态 accent 填充 + 深色文字（hover 提亮为
/// `accent_hover`），未选中态 `bg_control` + 主文字色（hover 提亮为
/// `bg_control_hover`）。
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
        .px(px(PILL_PADDING_X))
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

/// Fluent 图标按钮（40×40，Subtle 风格）：文字符号作 glyph，hover 提亮为
/// `bg_card_hover`。`tooltip` 悬停时以 GPUI 原生 tooltip 浮层显示，同时
/// 作为无障碍名称暴露。
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
        .rounded_md()
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

/// Fluent 空状态：居中的大图标区（64px 圆形控件底 + 32px 符号）+ 标题 +
/// 描述 + 可选动作按钮。`action` 由调用方用 [`fluent_button`] 构建后传入
/// （传 `None` 则不显示动作区）。
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
        .gap_3()
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
        .when_some(action, |this, action| this.mt_2().child(action))
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
            .rounded_lg()
            .bg(theme.bg_card)
            .border_1()
            .border_color(theme.danger)
            .px_4()
            .py_2()
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

    /// 居中的一行状态提示（加载中 / 空态 / 无更多数据）。
    pub(crate) fn status_note(&self, text: &str) -> AnyElement {
        let theme = self.theme();
        div()
            .w_full()
            .py_8()
            .flex()
            .items_center()
            .justify_center()
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
