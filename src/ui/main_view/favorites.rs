//! 收藏页。
//!
//! 数据来自 `get_favorite_items`（`Filters=IsFavorite`），分类 pill 通过
//! `IncludeItemTypes` 切换（全部 = `Movie,Series` / 电影 / 剧集），切换后重新
//! 拉取（`MainView::set_favorite_kind`）。布局：分类 pill 行 + `poster_card`
//! 网格（`flex_wrap`）。内容只借用 `MainView::favorites`。

use super::components::{SPACING_L, SPACING_S, SPACING_XL, fluent_empty_state};
use super::state::{FavoriteKind, MainView};
use gpui_kit::{AnyElement, Context, SharedString, div, px, prelude::*};

impl MainView {
    /// 渲染收藏页。
    pub(crate) fn favorites_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let mut root = div().w_full().flex().flex_col().gap(px(SPACING_XL));
        if let Some(banner) = self.page_error_banner() {
            root = root.child(banner);
        }
        root = root.child(self.favorite_kind_row(cx));

        let Some(favorites) = self.favorites.as_ref() else {
            if self.favorites_loading {
                return root.child(self.status_note("加载收藏…")).into_any_element();
            }
            return root.child(favorites_empty_state(theme)).into_any_element();
        };

        if favorites.items.is_empty() {
            return root.child(favorites_empty_state(theme)).into_any_element();
        }

        root = root
            .child(self.status_note(&format!("共 {} 项", favorites.items.len())))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(SPACING_L))
                    .children(favorites.items.iter().map(|item| self.poster_card(item, cx))),
            );

        root.into_any_element()
    }

    /// 分类 pill 行：闭包只捕获 `Copy` 的 `FavoriteKind`。
    fn favorite_kind_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut row = div().flex().flex_row().items_center().gap(px(SPACING_S));
        for kind in FavoriteKind::ALL {
            let selected = self.favorite_kind == kind;
            row = row.child(self.selectable_pill(
                SharedString::from(format!("favorites-kind-{}", kind.param())),
                kind.label(),
                selected,
                cx,
                move |view: &mut MainView, cx| view.set_favorite_kind(kind, cx),
            ));
        }
        row.into_any_element()
    }
}

/// 收藏页空状态。
fn favorites_empty_state(theme: crate::ui::theme::ThemeColors) -> AnyElement {
    fluent_empty_state(
        theme,
        "❤",
        "暂无收藏",
        "在详情页点击「收藏」的内容会出现在这里",
        None,
    )
}
