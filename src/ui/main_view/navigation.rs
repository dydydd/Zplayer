//! Navigation pane and page header for the Zplayer application shell.
//!
//! The pane owns the brand area, the primary navigation entries, the media
//! library group, the server group, and the bottom command list; the header
//! owns back navigation and the page title. Event closures only capture
//! `Entity<MainView>` plus `Copy` data (`Page`, list indices) — never
//! `String` or `Vec`.

use super::components::{RADIUS_SM, SPACING_L, SPACING_M, SPACING_S, SPACING_XS};
use super::state::{MainView, Page};
use crate::models::{MediaLibrary, SavedServerSummary};
use crate::ui::theme::ThemeColors;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    AnyElement, Context, Div, ElementId, Entity, FontWeight, SharedString, Stateful,
};
use gpui_kit::{div, px, prelude::*};
use std::sync::Arc;

/// Expanded pane width in logical pixels.
const NAV_PANE_WIDTH: f32 = 260.;
/// Collapsed icon-column width in logical pixels.
const NAV_PANE_COLLAPSED_WIDTH: f32 = 48.;
/// Page header height in logical pixels (44, v9 density).
const HEADER_HEIGHT: f32 = 44.;
/// Header icon button size (36, v9 icon button compact).
const HEADER_BUTTON_SIZE: f32 = 36.;
/// Fixed icon slot width so entry labels align in one column.
const ICON_SLOT_WIDTH: f32 = 20.;
/// Height of a single navigation entry (36, v9 nav density).
const NAV_ENTRY_HEIGHT: f32 = 36.;
/// Width of the selection indicator pill drawn at a selected entry's left
/// edge (2, strokeWidthThin double).
const NAV_INDICATOR_WIDTH: f32 = 2.;
/// Height of the selection indicator pill.
const NAV_INDICATOR_HEIGHT: f32 = 16.;

impl MainView {
    /// Navigation pane: brand area, navigation entries, and the bottom
    /// command list, over the sidebar background.
    pub(crate) fn nav_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let view = cx.entity();
        let collapsed = self.nav_collapsed;
        let width = if collapsed {
            NAV_PANE_COLLAPSED_WIDTH
        } else {
            NAV_PANE_WIDTH
        };

        // Brand area: the Zplayer wordmark and the collapse toggle. The
        // collapsed pane centers the toggle in the same slot.
        let brand = if collapsed {
            div()
                .flex()
                .items_center()
                .justify_center()
                .w_full()
                .h(gpui_kit::px(HEADER_HEIGHT))
                .child(Self::nav_collapse_button(
                    "shell.navigation.expand",
                    "»",
                    &view,
                    theme,
                ))
        } else {
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(SPACING_L))
                .w_full()
                .h(gpui_kit::px(HEADER_HEIGHT))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Zplayer"),
                )
                .child(Self::nav_collapse_button(
                    "shell.navigation.collapse",
                    "«",
                    &view,
                    theme,
                ))
        };

        // Bottom command area, pinned to the pane's bottom edge.
        let bottom = div()
            .id("shell.navigation.bottom")
            .flex()
            .flex_col()
            .gap(px(SPACING_XS))
            .px(px(SPACING_S))
            .py(px(SPACING_S))
            .mt_auto()
            .border_t_1()
            .border_color(theme.border)
            .child(self.nav_page_entry(
                "nav.settings",
                "⚙",
                "设置",
                Page::Settings,
                true,
                collapsed,
                &view,
            ));

        div()
            .id("shell.navigation.pane")
            .w(gpui_kit::px(width))
            .h_full()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .bg(theme.bg_sidebar)
            .child(brand)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .flex()
                    .flex_col()
                    .gap(px(SPACING_XS))
                    .px(px(SPACING_S))
                    .py(px(SPACING_S))
                    .child(self.nav_page_entry(
                        "nav.home",
                        "🏠",
                        "首页",
                        Page::Home,
                        true,
                        collapsed,
                        &view,
                    ))
                    .child(self.nav_page_entry(
                        "nav.search",
                        "🔍",
                        "搜索",
                        Page::Search,
                        true,
                        collapsed,
                        &view,
                    ))
                    .child(self.nav_library_group(collapsed, &view))
                    .child(self.nav_page_entry(
                        "nav.history",
                        "🕘",
                        "播放历史",
                        Page::History,
                        true,
                        collapsed,
                        &view,
                    ))
                    .child(self.nav_page_entry(
                        "nav.favorites",
                        "❤",
                        "收藏",
                        Page::Favorites,
                        true,
                        collapsed,
                        &view,
                    ))
                    .child(self.nav_page_entry(
                        "nav.servers",
                        "📡",
                        "服务列表",
                        Page::Servers,
                        true,
                        collapsed,
                        &view,
                    ))
                    .child(self.nav_server_group(collapsed, &view)),
            )
            .child(bottom)
            .into_any_element()
    }

    /// Page header: back control (when history exists) and the page title,
    /// over the header background.
    pub(crate) fn header_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let view = cx.entity();

        let mut header = div()
            .id("shell.header")
            .flex()
            .items_center()
            .w_full()
            .h(gpui_kit::px(HEADER_HEIGHT))
            .flex_shrink_0()
            .px(px(SPACING_S))
            .gap(px(SPACING_XS))
            .bg(theme.bg_header)
            .border_b_1()
            .border_color(theme.border);

        // Back control: shown when there is navigation history to return to.
        if !self.nav_history.is_empty() {
            let back_view = view.clone();
            header = header.child(
                div()
                    .id("shell.navigation.back")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(gpui_kit::px(HEADER_BUTTON_SIZE))
                    .rounded(px(RADIUS_SM))
                    .cursor_pointer()
                    .text_color(theme.text_secondary)
                    .hover(|this| this.bg(theme.bg_card_hover))
                    .on_click(move |_, _, cx| {
                        let _ = back_view.update(cx, |view, cx| view.go_back(cx));
                    })
                    .child("←"),
            );
        }

        header
            .child(
                div()
                    .px(px(SPACING_S))
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.page_title()),
            )
            .into_any_element()
    }

    /// Header title for the active page.
    fn page_title(&self) -> &'static str {
        match self.page {
            Page::Home => "Zplayer",
            Page::Search => "搜索",
            Page::Detail => "详情",
            Page::Person => "人物",
            Page::Library => "媒体库",
            Page::History => "播放历史",
            Page::Favorites => "收藏",
            Page::Player => "播放",
            Page::Servers => "服务",
            Page::Settings => "设置",
        }
    }

    /// The pane collapse/expand toggle (32×32 icon button). Both directions
    /// toggle the same collapsed flag, so the collapsed pane reuses this
    /// control as its expand button.
    fn nav_collapse_button(
        id: &'static str,
        glyph: &'static str,
        view: &Entity<Self>,
        theme: ThemeColors,
    ) -> Stateful<Div> {
        let view = view.clone();
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .size(gpui_kit::px(32.))
            .rounded(px(RADIUS_SM))
            .cursor_pointer()
            .text_color(theme.text_secondary)
            .hover(|this| this.bg(theme.bg_card_hover))
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |view, cx| view.toggle_nav_collapsed(cx));
            })
            .child(glyph)
    }

    /// One Fluent NavView entry row: a 2px selection indicator slot at the
    /// left edge, a fixed icon slot, and the label. Selected entries get the
    /// accent indicator, the card-hover surface, and accent-colored content;
    /// unselected entries use the secondary text color and highlight with
    /// the card-hover surface on hover. The returned row is interactive, so
    /// the caller attaches its own click handler.
    fn nav_entry_row(
        id: impl Into<ElementId>,
        icon: &'static str,
        label: SharedString,
        selected: bool,
        collapsed: bool,
        theme: ThemeColors,
    ) -> Stateful<Div> {
        let row = div()
            .id(id)
            .flex()
            .items_center()
            .w_full()
            .h(gpui_kit::px(NAV_ENTRY_HEIGHT))
            .rounded(px(RADIUS_SM))
            .cursor_pointer()
            .when(selected, |this| {
                this.bg(theme.bg_card_hover).text_color(theme.accent)
            })
            .when(!selected, |this| this.text_color(theme.text_secondary))
            .hover(|this| this.bg(theme.bg_card_hover));

        if collapsed {
            row.child(Self::nav_indicator(selected, theme))
                .child(div().flex_1().flex().justify_center().child(icon))
        } else {
            row.gap(px(SPACING_S))
                .pr(px(SPACING_M))
                .child(Self::nav_indicator(selected, theme))
                .child(
                    div()
                        .w(gpui_kit::px(ICON_SLOT_WIDTH))
                        .flex()
                        .justify_center()
                        .child(icon),
                )
                .child(label)
        }
    }

    /// Fluent selection indicator: a 2px accent pill flush with the entry's
    /// left edge. Unselected entries keep the slot as an invisible spacer so
    /// every label aligns in one column.
    fn nav_indicator(selected: bool, theme: ThemeColors) -> Div {
        div()
            .w(gpui_kit::px(NAV_INDICATOR_WIDTH))
            .h(gpui_kit::px(NAV_INDICATOR_HEIGHT))
            .rounded_full()
            .when(selected, |this| this.bg(theme.accent))
    }

    /// A navigation entry that switches to `page` on click. `highlight`
    /// controls whether the entry shows the selected state while `page` is
    /// active (the 添加服务 entry shares `Page::Servers` with the 服务列表
    /// entry and stays unhighlighted).
    fn nav_page_entry(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        page: Page,
        highlight: bool,
        collapsed: bool,
        view: &Entity<Self>,
    ) -> AnyElement {
        let selected = highlight && self.page == page;
        let theme = self.theme();
        let view = view.clone();

        Self::nav_entry_row(id, icon, SharedString::from(label), selected, collapsed, theme)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |view, cx| view.navigate(page, cx));
            })
            .into_any_element()
    }

    /// 媒体库分组：分组标题 + 每个库一个入口（来自首页数据）。未加载到
    /// 首页数据时该分组不渲染。
    fn nav_library_group(&self, collapsed: bool, view: &Entity<Self>) -> AnyElement {
        let theme = self.theme();
        let Some(home) = self.home.as_ref() else {
            return div().into_any_element();
        };
        if home.libraries.is_empty() {
            return div().into_any_element();
        }

        let mut group = div().flex().flex_col().gap(px(SPACING_XS));
        if !collapsed {
            group = group.child(
                div()
                    .px(px(SPACING_M))
                    .pt(px(SPACING_S))
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child("媒体库"),
            );
        }
        let entries = home
            .libraries
            .iter()
            .enumerate()
            .map(|(index, library)| self.nav_library_entry(index, library, collapsed, view))
            .collect::<Vec<_>>();

        group.children(entries).into_any_element()
    }

    /// 单个媒体库入口：点击进入该库的浏览页。闭包只捕获 `Copy` 的下标，
    /// id/名称的查找发生在 `open_library_at` 内部。
    fn nav_library_entry(
        &self,
        index: usize,
        library: &MediaLibrary,
        collapsed: bool,
        view: &Entity<Self>,
    ) -> AnyElement {
        let selected = self.page == Page::Library
            && self.selected_library_id.as_deref() == Some(library.id.as_str());
        let theme = self.theme();
        let view = view.clone();
        let name = SharedString::from(library.name.as_str());

        Self::nav_entry_row(
            format!("library-item-{index}"),
            "🎞",
            name,
            selected,
            collapsed,
            theme,
        )
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |view, cx| view.open_library_at(index, cx));
        })
        .into_any_element()
    }

    /// 服务器分组：每个已保存服务一个入口（当前服务器带选中标记），
    /// 末尾是「添加服务」入口。
    fn nav_server_group(&self, collapsed: bool, view: &Entity<Self>) -> AnyElement {
        let entries = self
            .servers
            .iter()
            .enumerate()
            .map(|(index, server)| self.nav_server_entry(index, server, collapsed, view))
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .gap(px(SPACING_XS))
            .children(entries)
            .child(self.nav_page_entry(
                "nav.add-server",
                "＋",
                "添加服务",
                Page::Servers,
                false,
                collapsed,
                view,
            ))
            .into_any_element()
    }

    /// 单个已保存服务入口：点击激活该服务。闭包只捕获 `Copy` 的下标，
    /// id 的查找发生在 `activate_server_at` 内部。服务名是借用转换
    /// （`SharedString::from(server.name.as_str())`），不是克隆。
    fn nav_server_entry(
        &self,
        index: usize,
        server: &SavedServerSummary,
        collapsed: bool,
        view: &Entity<Self>,
    ) -> AnyElement {
        let theme = self.theme();
        let view = view.clone();
        let active = server.active;
        let name = SharedString::from(server.name.as_str());

        Self::nav_entry_row(
            format!("server-item-{index}"),
            "🖥",
            name,
            active,
            collapsed,
            theme,
        )
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |view, cx| view.activate_server_at(index, cx));
        })
        .into_any_element()
    }

    /// Activates the saved server at `index`. The id lookup happens inside
    /// the update so the click closure captures nothing heap-allocated. The
    /// id is materialized once as an `Arc<str>` here (a copy out of
    /// `self.servers`, which ends the borrow before the `&mut self` call
    /// below).
    fn activate_server_at(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(server_id) = self
            .servers
            .get(index)
            .map(|server| Arc::from(server.id.as_str()))
        else {
            return;
        };
        self.select_server(server_id, cx);
    }
}
