//! 媒体库浏览页。
//!
//! 布局（自上而下）：
//! 1. 排序 pill 行：名称 / 添加时间 / 评分 / 首播日期（点击切换 `library_sort`
//!    并重新拉取第一页）；
//! 2. 海报网格：`poster_card`（`flex_wrap`，点击进入详情）；
//! 3. 「加载更多」：`has_more` 时按 `start_index += limit` 追加下一页。
//!
//! 顶栏标题由 `navigation.rs` 取 `selected_library_name`；内容只借用
//! `MainView::library`，点击闭包只捕获 `Copy` 的排序枚举与 `Entity`。

use super::components::{FluentButtonVariant, fluent_button, fluent_empty_state};
use super::state::{LibrarySort, MainView};
use gpui_kit::{AnyElement, Context, SharedString, div, prelude::*};

impl MainView {
    /// 渲染媒体库浏览页。
    pub(crate) fn library_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let mut root = div().w_full().flex().flex_col().gap_5();
        if let Some(banner) = self.page_error_banner() {
            root = root.child(banner);
        }
        root = root.child(self.library_sort_row(cx));

        let Some(library) = self.library.as_ref() else {
            if self.library_loading {
                return root.child(self.status_note("加载媒体库…")).into_any_element();
            }
            return root.child(library_empty_state(theme)).into_any_element();
        };

        if library.items.is_empty() {
            return root.child(library_empty_state(theme)).into_any_element();
        }

        root = root.child(
            div()
                .w_full()
                .flex()
                .flex_wrap()
                .gap_4()
                .children(library.items.iter().map(|item| self.poster_card(item, cx))),
        );

        let loaded = library.items.len() as u64;
        let total = library.total_count;
        if library.has_more {
            root = root
                .child(self.status_note(&format!("已加载 {loaded} / {total}")))
                .child(
                    div().flex().justify_center().child(fluent_button(
                        theme,
                        "library-load-more",
                        "加载更多",
                        FluentButtonVariant::Standard,
                        cx.listener(|view: &mut MainView, _event, _window, cx| {
                            view.load_library_page(false, cx);
                        }),
                    )),
                );
        } else {
            root = root.child(self.status_note(&format!("共 {total} 项")));
        }

        root.into_any_element()
    }

    /// 排序 pill 行：点击切换排序并重新拉取（闭包只捕获 `Copy` 的 `LibrarySort`）。
    fn library_sort_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut row = div().flex().flex_row().items_center().gap_2();
        for sort in LibrarySort::ALL {
            let selected = self.library_sort == sort;
            row = row.child(self.selectable_pill(
                SharedString::from(format!("library-sort-{}", sort.param())),
                sort.label(),
                selected,
                cx,
                move |view: &mut MainView, cx| view.set_library_sort(sort, cx),
            ));
        }
        row.into_any_element()
    }
}

/// 媒体库页空状态。
fn library_empty_state(theme: crate::ui::theme::ThemeColors) -> AnyElement {
    fluent_empty_state(
        theme,
        "🎞",
        "该媒体库暂无内容",
        "试试切换排序方式，或稍后再来看看",
        None,
    )
}
