use super::state::{FavoriteKind, LibrarySort, MainView, Page};
use crate::models::{EpisodesInput, ItemInput, SearchInput};
use crate::services::{AppPaths, LibraryService};
use gpui_kit::Context;
use gpui_kit::prelude::*;
use std::sync::Arc;

/// 媒体库浏览页每页条数。
const LIBRARY_PAGE_SIZE: u64 = 60;
/// 播放历史页每页条数。
const HISTORY_PAGE_SIZE: u64 = 40;

impl MainView {
    pub(crate) fn load_home(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Home;
        self.loading = true;
        self.error = None;
        self.home = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx.background_spawn(async move { service.home() }).await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(home) => {
                        this.home = Some(home);
                        this.loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn perform_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string();
        self.page = Page::Search;
        self.loading = true;
        self.error = None;
        self.search = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.search(SearchInput { query }) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(search) => {
                        this.search = Some(search);
                        this.loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn open_detail(
        &mut self,
        item_id: Arc<str>,
        server_id: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        self.page = Page::Detail;
        self.loading = true;
        self.error = None;
        self.detail = None;
        self.detail_is_favorite = None;
        self.detail_is_played = None;
        // 简介展开状态不跨条目保留（`detail.rs` 的「展开/收起」）。
        self.detail_synopsis_expanded = false;
        self.resolved_session = None;
        self.player = None;
        self.playback_state = None;
        // 状态字段存 `Arc<str>`（引用计数克隆），不复制 id 字符串。
        self.selected_item_id = Some(Arc::clone(&item_id));
        self.selected_season_id = None;
        self.selected_episode_id = None;
        self.season_episodes.clear();
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let input = ItemInput {
            // `ItemInput` 仍是 `String`（进入服务/API 层）：点击路径低频，各转一次。
            item_id: item_id.to_string(),
            server_id: Some(server_id.to_string()),
            media_source_id: None,
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitle_stream_position: None,
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.detail(input) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(mut detail) => {
                        this.detail_is_favorite = Some(detail.item.favorite);
                        this.detail_is_played = Some(detail.item.played);
                        this.selected_season_id =
                            detail.seasons.first().map(|season| Arc::from(season.id.as_str()));
                        this.selected_episode_id =
                            detail.episodes.first().map(|episode| Arc::from(episode.id.as_str()));
                        this.season_episodes = std::mem::take(&mut detail.episodes);
                        this.detail = Some(detail);
                        this.loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn load_season_episodes(
        &mut self,
        season_id: Arc<str>,
        server_id: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        self.load_episodes(None, season_id, server_id, cx);
    }

    pub(crate) fn load_more_episodes(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = self.detail.as_ref() else {
            return;
        };
        // `selected_season_id` 是 `Arc<str>`：克隆引用计数（廉价）。
        let Some(season_id) = self.selected_season_id.clone() else {
            return;
        };
        let start_index = self.season_episodes.len();
        let server_id = Arc::from(detail.item.server_id.as_str());
        self.load_episodes(Some(start_index), season_id, server_id, cx);
    }

    fn load_episodes(
        &mut self,
        start_index: Option<usize>,
        season_id: Arc<str>,
        server_id: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        let selected_season_id = Arc::clone(&season_id);
        self.season_loading = true;
        self.error = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.season_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.season_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let series_id = self
            .selected_item_id
            .clone()
            .unwrap_or_else(|| Arc::clone(&season_id));
        // `EpisodesInput` 仍是 `String`（进入服务/API 层）：低频路径，各转一次。
        let input = EpisodesInput {
            series_id: series_id.to_string(),
            season_id: Some(season_id.to_string()),
            server_id: Some(server_id.to_string()),
            start_index,
            limit: Some(50),
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.episodes(input) })
                .await;

            view.update(cx, move |this, cx| {
                match result {
                    Ok(episodes) => {
                        this.selected_season_id = Some(selected_season_id);
                        if start_index.unwrap_or_default() > 0 {
                            this.season_episodes.extend(episodes.items);
                        } else {
                            this.season_episodes = episodes.items;
                            this.selected_episode_id = this
                                .season_episodes
                                .first()
                                .map(|episode| Arc::from(episode.id.as_str()));
                        }
                        this.season_loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.season_loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 详情页操作行：收藏 / 已看
    // ------------------------------------------------------------------

    /// 切换详情页当前条目的收藏状态（乐观更新 + 失败回滚）。
    pub(crate) fn toggle_detail_favorite(&mut self, cx: &mut Context<Self>) {
        let Some((server_id, item_id, current)) = self.detail_flag_target(ItemFlag::Favorite) else {
            return;
        };
        let next = !current;
        self.apply_detail_flag(ItemFlag::Favorite, next);
        self.error = None;
        cx.notify();
        self.dispatch_detail_flag(ItemFlag::Favorite, server_id, item_id, current, next, cx);
    }

    /// 切换详情页当前条目的已看状态（乐观更新 + 失败回滚）。
    pub(crate) fn toggle_detail_played(&mut self, cx: &mut Context<Self>) {
        let Some((server_id, item_id, current)) = self.detail_flag_target(ItemFlag::Played) else {
            return;
        };
        let next = !current;
        self.apply_detail_flag(ItemFlag::Played, next);
        self.error = None;
        cx.notify();
        self.dispatch_detail_flag(ItemFlag::Played, server_id, item_id, current, next, cx);
    }

    /// 读取详情页当前条目的 (server_id, item_id, 当前值)。
    fn detail_flag_target(&self, flag: ItemFlag) -> Option<(String, String, bool)> {
        let detail = self.detail.as_ref()?;
        let current = match flag {
            ItemFlag::Favorite => self.detail_is_favorite.unwrap_or(detail.item.favorite),
            ItemFlag::Played => self.detail_is_played.unwrap_or(detail.item.played),
        };
        Some((
            detail.item.server_id.clone(),
            detail.item.id.clone(),
            current,
        ))
    }

    /// 写入乐观值（同时同步 `detail.item`，让按钮文案立即更新）。
    fn apply_detail_flag(&mut self, flag: ItemFlag, value: bool) {
        match flag {
            ItemFlag::Favorite => {
                self.detail_is_favorite = Some(value);
                if let Some(detail) = self.detail.as_mut() {
                    detail.item.favorite = value;
                }
            }
            ItemFlag::Played => {
                self.detail_is_played = Some(value);
                if let Some(detail) = self.detail.as_mut() {
                    detail.item.played = value;
                }
            }
        }
    }

    /// 后台调用 `set_favorite` / `set_played`；失败时回滚到 `current`。
    fn dispatch_detail_flag(
        &mut self,
        flag: ItemFlag,
        server_id: String,
        item_id: String,
        current: bool,
        next: bool,
        cx: &mut Context<Self>,
    ) {
        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.rollback_detail_flag(flag, current, &err, cx);
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.rollback_detail_flag(flag, current, &err, cx);
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    match flag {
                        ItemFlag::Favorite => service.set_favorite(&server_id, &item_id, next),
                        ItemFlag::Played => service.set_played(&server_id, &item_id, next),
                    }
                })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(()) => this.error = None,
                    Err(err) => {
                        this.apply_detail_flag(flag, current);
                        this.error = Some(format!("{}操作失败：{err}", flag.label()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 本地失败（路径/服务不可用）时的回滚与报错。
    fn rollback_detail_flag(
        &mut self,
        flag: ItemFlag,
        current: bool,
        err: &str,
        cx: &mut Context<Self>,
    ) {
        self.apply_detail_flag(flag, current);
        self.error = Some(format!("{}操作失败：{err}", flag.label()));
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 人物页
    // ------------------------------------------------------------------

    /// 进入人物页并装载数据。
    pub(crate) fn open_person(&mut self, person_id: Arc<str>, cx: &mut Context<Self>) {
        // 记录来源页，供顶栏返回按钮使用（与 `navigate` 一致的历史语义）。
        if self.page != Page::Person {
            self.nav_history.push(self.page);
        }
        self.page = Page::Person;
        self.selected_person_id = Some(person_id.clone());
        self.load_person(person_id, cx);
    }

    /// 并行拉取人物资料与出演作品（`person_detail` + `filmography`）。
    pub(crate) fn load_person(&mut self, person_id: Arc<str>, cx: &mut Context<Self>) {
        self.person = None;
        self.person_loading = true;
        self.error = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.person_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.person_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.person_page(&person_id) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(person) => {
                        this.person = Some(person);
                        this.person_loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.person_loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 媒体库浏览页
    // ------------------------------------------------------------------

    /// 进入媒体库浏览页（首页库卡 / 侧栏库入口）。
    pub(crate) fn open_library(
        &mut self,
        library_id: Arc<str>,
        library_name: Arc<str>,
        cx: &mut Context<Self>,
    ) {
        if self.page != Page::Library {
            self.nav_history.push(self.page);
        }
        self.page = Page::Library;
        self.selected_library_id = Some(library_id);
        self.selected_library_name = Some(library_name);
        self.load_library_page(true, cx);
    }

    /// 按首页 `home.libraries` 的下标进入库浏览页。闭包只捕获下标，id/名称
    /// 的查找发生在 update 内部。
    pub(crate) fn open_library_at(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(library) = self
            .home
            .as_ref()
            .and_then(|home| home.libraries.get(index))
        else {
            return;
        };
        let library_id: Arc<str> = Arc::from(library.id.as_str());
        let library_name: Arc<str> = Arc::from(library.name.as_str());
        self.open_library(library_id, library_name, cx);
    }

    /// 切换排序方式并重新拉取第一页。
    pub(crate) fn set_library_sort(&mut self, sort: LibrarySort, cx: &mut Context<Self>) {
        if self.library_sort == sort {
            return;
        }
        self.library_sort = sort;
        self.load_library_page(true, cx);
    }

    /// 装载库浏览页：`reset = true` 重新拉第一页，否则从已加载条数继续追加。
    pub(crate) fn load_library_page(&mut self, reset: bool, cx: &mut Context<Self>) {
        if !reset && self.library_loading {
            return;
        }
        let Some(library_id) = self.selected_library_id.clone() else {
            return;
        };
        let library_name = self
            .selected_library_name
            .clone()
            .unwrap_or_else(|| Arc::from("媒体库"));
        let sort_by = self.library_sort.param();
        let start_index = if reset {
            0
        } else {
            self.library
                .as_ref()
                .map(|library| library.items.len() as u64)
                .unwrap_or(0)
        };
        if reset {
            self.library = None;
        }
        self.library_loading = true;
        self.error = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.library_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.library_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    service.library_page(
                        &library_id,
                        &library_name,
                        start_index,
                        LIBRARY_PAGE_SIZE,
                        sort_by,
                    )
                })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    // 用户可能在请求返回前切换了媒体库：丢弃过期结果。
                    Ok(page)
                        if this.selected_library_id.as_deref()
                            != Some(page.library_id.as_str()) => {}
                    Ok(page) => {
                        match this.library.as_mut() {
                            Some(current) if !reset => {
                                current.items.extend(page.items);
                                current.total_count = page.total_count;
                                current.start_index = page.start_index;
                                current.has_more = page.has_more;
                            }
                            _ => this.library = Some(page),
                        }
                        this.library_loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.library_loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 播放历史页
    // ------------------------------------------------------------------

    /// 装载播放历史：`reset = true` 重新拉第一页，否则追加下一页。
    pub(crate) fn load_history(&mut self, reset: bool, cx: &mut Context<Self>) {
        self.page = Page::History;
        if !reset && self.history_loading {
            return;
        }
        let start_index = if reset {
            0
        } else {
            self.history
                .as_ref()
                .map(|history| history.items.len() as u64)
                .unwrap_or(0)
        };
        if reset {
            self.history = None;
        }
        self.history_loading = true;
        self.error = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.history_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.history_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    service.played_history_page(start_index, HISTORY_PAGE_SIZE)
                })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(page) => {
                        match this.history.as_mut() {
                            Some(current) if !reset => {
                                current.items.extend(page.items);
                                current.total_count = page.total_count;
                                current.start_index = page.start_index;
                                current.has_more = page.has_more;
                            }
                            _ => this.history = Some(page),
                        }
                        this.history_loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.history_loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // 收藏页
    // ------------------------------------------------------------------

    /// 装载收藏列表（按当前分类 pill 的 `IncludeItemTypes`）。
    pub(crate) fn load_favorites(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Favorites;
        let item_types = self.favorite_kind.param();
        self.favorites = None;
        self.favorites_loading = true;
        self.error = None;
        cx.notify();

        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.favorites_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.favorites_loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.favorites(item_types) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(favorites) => {
                        this.favorites = Some(favorites);
                        this.favorites_loading = false;
                        this.error = None;
                    }
                    Err(err) => {
                        this.favorites_loading = false;
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 切换收藏页分类 pill 并重新拉取。
    pub(crate) fn set_favorite_kind(&mut self, kind: FavoriteKind, cx: &mut Context<Self>) {
        if self.favorite_kind == kind {
            return;
        }
        self.favorite_kind = kind;
        self.load_favorites(cx);
    }
}

/// 详情页可切换的用户标记（收藏 / 已看）。
#[derive(Clone, Copy, Debug)]
enum ItemFlag {
    Favorite,
    Played,
}

impl ItemFlag {
    fn label(self) -> &'static str {
        match self {
            Self::Favorite => "收藏",
            Self::Played => "已看",
        }
    }
}
