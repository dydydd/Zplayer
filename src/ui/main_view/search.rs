//! 搜索页：搜索结果按服务器分组，每组是一条 `cards.rs` 的横向滚动海报
//! 卡行，卡片尺寸、间距、角标与首页完全一致。
//!
//! 数据全部来自 `&SearchPayload` 的借用切片：每张卡片只借用它的
//! `MediaItem`（`poster_card` 的签名就是 `&MediaItem`），分组只保存下标，
//! 因此渲染结果页不会拷贝任何条目载荷。

use super::components::{fluent_empty_state, fluent_section_title};
use super::state::MainView;
use crate::models::MediaItem;
use gpui_kit::{AnyElement, Context, SharedString, div, prelude::*};

impl MainView {
    /// 搜索页内容：加载中 → 错误 → 空查询提示 → 无结果 → 分组结果行。
    ///
    /// 未按过回车时 `self.search` 是 `None`（`perform_search` 才写入），
    /// 此时展示居中的「按回车搜索所有服务器」引导，而不是一片空白。
    pub(crate) fn search_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();

        if self.loading {
            return self.status_note("搜索中...");
        }
        // 与详情页/媒体库页一致：`self.error` 有值时显示页面级错误条。
        if let Some(banner) = self.page_error_banner() {
            return banner;
        }
        let Some(search) = self.search.as_ref() else {
            return fluent_empty_state(
                theme,
                "🔍",
                "搜索媒体",
                "按回车搜索所有服务器",
                None,
            );
        };

        if search.items.is_empty() {
            // `value()` 返回 `SharedString`，这里直接借用它的 `Display`
            // 拼一次说明行，不额外克隆查询串。
            let query = self.search_input.read(cx).value();
            return fluent_empty_state(
                theme,
                "🔍",
                "未找到结果",
                &format!("在已连接的服务器中没有匹配 \"{query}\" 的内容"),
                None,
            );
        }

        let groups = group_search_items(&search.items);

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_6()
            .child(fluent_section_title(
                theme,
                &format!("搜索结果 ({})", search.items.len()),
            ))
            .children(groups.into_iter().map(|(server_name, indices)| {
                self.search_group(&search.items, &server_name, &indices, cx)
            }))
            .into_any_element()
    }

    /// 一个服务器分组：服务器名作为小节标题，条目放进横向滚动行。
    ///
    /// 卡片由 `cards.rs` 的 [`MainView::poster_card`] 渲染，行容器复用
    /// [`MainView::scroll_row`]。`server_name` 按引用传入，标题只在
    /// `scroll_row` 内部做一次 `&str` → `SharedString` 的转换。
    ///
    /// `indices` 是 `items` 中的下标（分组逻辑见 [`group_search_items`]），
    /// 所以这里始终借用条目本身，不构造新的 `MediaItem`。
    fn search_group(
        &self,
        items: &[MediaItem],
        server_name: &SharedString,
        indices: &[usize],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cards = indices
            .iter()
            .map(|index| self.poster_card(&items[*index], cx));
        self.scroll_row(server_name.as_ref(), cards)
    }
}

/// 按服务器名把搜索结果切成若干组，组内保持服务器返回的原始顺序。
///
/// 返回 `(服务器名, 条目下标)`：只记下标而不复制条目，调用方继续借用
/// `&[MediaItem]`。服务器名相同的条目会并入同一组（首次出现的服务器决定
/// 组的先后顺序），因此同一台服务器的结果不会被拆成多行。
fn group_search_items(items: &[MediaItem]) -> Vec<(SharedString, Vec<usize>)> {
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if let Some((_, indices)) = groups
            .iter_mut()
            .find(|(name, _)| name.as_str() == item.server_name.as_str())
        {
            indices.push(index);
        } else {
            groups.push((SharedString::from(item.server_name.as_str()), vec![index]));
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只带分组所需字段的测试条目，其余字段留空。
    fn item(id: &str, server: &str) -> MediaItem {
        MediaItem {
            id: id.to_string(),
            server_id: format!("{server}-id"),
            server_name: server.to_string(),
            name: id.to_string(),
            item_type: "Movie".to_string(),
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
            played: false,
            favorite: false,
            last_played_date: None,
            primary_image_url: None,
            backdrop_url: None,
            logo_url: None,
        }
    }

    #[test]
    fn groups_keep_first_seen_server_order() {
        let items = vec![
            item("a", "甲服务器"),
            item("b", "乙服务器"),
            item("c", "甲服务器"),
        ];
        let groups = group_search_items(&items);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0.as_ref(), "甲服务器");
        assert_eq!(groups[0].1, vec![0, 2]);
        assert_eq!(groups[1].0.as_ref(), "乙服务器");
        assert_eq!(groups[1].1, vec![1]);
    }

    #[test]
    fn every_item_lands_in_exactly_one_group() {
        let items = vec![
            item("a", "甲"),
            item("b", "乙"),
            item("c", "丙"),
            item("d", "乙"),
        ];
        let groups = group_search_items(&items);
        let mut seen: Vec<usize> = groups
            .iter()
            .flat_map(|(_, idx)| idx.iter().copied())
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2, 3]);
    }
}
