//! Application shell render: page header on top, navigation pane and page
//! content below. The pane and header themselves live in `navigation.rs`;
//! page switching state and history live in `state.rs`.

use super::components::SPACING_L;
use super::state::{MainView, Page};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{AnyElement, Context, IntoElement, Render, Window};
use gpui_kit::{div, px, prelude::*};

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        // 播放器页的播放面要占满内容区（无内边距、不滚动），由播放器页自己
        // 绘制叠加控制层；其余页面沿用带内边距的滚动内容区。
        // `overflow_y_scrollbar()` 返回 `Scrollable<Div>`，与 `Div` 不是同一
        // 类型，因此两条分支各自构建后再统一为 `AnyElement`。
        let content = if self.page == Page::Player {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(self.page_content(cx))
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_y_scrollbar()
                .p_6()
                .child(self.page_content(cx))
                .into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg_main)
            .text_color(theme.text_primary)
            .child(self.header_bar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.nav_pane(cx))
                    .child(content),
            )
    }
}

impl MainView {
    /// Dispatches the active page into its content element.
    pub(crate) fn page_content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.page {
            Page::Home => self.home_content(cx),
            // The search field moved out of the old title bar into the
            // search page itself; Enter still triggers `perform_search`
            // through the input subscription in `state.rs`.
            Page::Search => div()
                .flex()
                .flex_col()
                .gap(px(SPACING_L))
                .child(
                    Input::new(&self.search_input)
                        .id("media-search")
                        .w(px(420.)),
                )
                .child(self.search_content(cx))
                .into_any_element(),
            Page::Detail => self.detail_content(cx),
            Page::Person => self.person_content(cx),
            Page::Library => self.library_content(cx),
            Page::History => self.history_content(cx),
            Page::Favorites => self.favorites_content(cx),
            Page::Player => self.player_content(cx),
            Page::Servers => self.servers_content(cx),
            Page::Settings => self.settings_content(cx),
        }
    }
}
