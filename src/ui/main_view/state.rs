use crate::models::{
    FavoritesPayload, HomePayload, ItemDetailPayload, LibraryBrowsePayload, MediaItem, PersonPayload,
    PlayedHistoryPayload, SavedServerSummary, SearchPayload,
};
use crate::playback::ResolvedPlaybackSession;
use crate::player::{PlayerSession, PlayerState};
use crate::services::AppPaths;
use super::settings::SettingsModule;
use crate::ui::theme::{ThemeColors, ThemeMode, is_dark_appearance};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_kit::{AppContext, Context, Entity, Image, Window};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) enum Page {
    #[default]
    Home,
    Search,
    Detail,
    /// 演员/人物页（由详情页演员卡点击进入）。
    Person,
    /// 媒体库浏览页（由首页媒体库卡或侧栏库入口进入）。
    Library,
    /// 播放历史页。
    History,
    /// 收藏页。
    Favorites,
    Player,
    Servers,
    Settings,
}

/// 媒体库浏览页的排序方式（顶栏排序 pill）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) enum LibrarySort {
    #[default]
    Name,
    DateAdded,
    Rating,
    PremiereDate,
}

impl LibrarySort {
    /// 顺序即排序 pill 的展示顺序。
    pub(crate) const ALL: [LibrarySort; 4] = [
        LibrarySort::Name,
        LibrarySort::DateAdded,
        LibrarySort::Rating,
        LibrarySort::PremiereDate,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "名称",
            Self::DateAdded => "添加时间",
            Self::Rating => "评分",
            Self::PremiereDate => "首播日期",
        }
    }

    /// Emby/Jellyfin 的 `SortBy` 取值。
    pub(crate) fn param(self) -> &'static str {
        match self {
            Self::Name => "SortName",
            Self::DateAdded => "DateCreated",
            Self::Rating => "CommunityRating",
            Self::PremiereDate => "PremiereDate",
        }
    }
}

/// 收藏页的分类 pill（映射到 `IncludeItemTypes`）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) enum FavoriteKind {
    #[default]
    All,
    Movie,
    Series,
}

impl FavoriteKind {
    pub(crate) const ALL: [FavoriteKind; 3] =
        [FavoriteKind::All, FavoriteKind::Movie, FavoriteKind::Series];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Movie => "电影",
            Self::Series => "剧集",
        }
    }

    pub(crate) fn param(self) -> &'static str {
        match self {
            Self::All => "Movie,Series",
            Self::Movie => "Movie",
            Self::Series => "Series",
        }
    }
}

pub(crate) struct MainView {
    pub(crate) page: Page,
    /// Whether the navigation pane is collapsed to its icon column
    /// (`shell.navigation.collapse`).
    pub(crate) nav_collapsed: bool,
    /// Back-navigation history: `navigate` pushes the page it leaves, and
    /// `go_back` pops it. `Page` is `Copy`, so entries are plain bytes and
    /// nothing is ever cloned.
    pub(crate) nav_history: Vec<Page>,
    pub(crate) theme: ThemeColors,
    pub(crate) theme_mode: ThemeMode,
    /// Active settings module; defaults to General.
    pub(crate) settings_module: SettingsModule,
    pub(crate) search_input: Entity<InputState>,
    pub(crate) server_type_input: Entity<InputState>,
    pub(crate) server_name_input: Entity<InputState>,
    pub(crate) server_url_input: Entity<InputState>,
    pub(crate) server_username_input: Entity<InputState>,
    pub(crate) server_password_input: Entity<InputState>,
    pub(crate) server_proxy_url_input: Entity<InputState>,
    pub(crate) seek_slider: Entity<SliderState>,
    pub(crate) servers: Vec<SavedServerSummary>,
    pub(crate) loading: bool,
    pub(crate) season_loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) home: Option<HomePayload>,
    pub(crate) search: Option<SearchPayload>,
    pub(crate) detail: Option<ItemDetailPayload>,
    /// 当前选中条目/季/集的 id。`Arc<str>`：渲染比较只借用（`as_deref`），
    /// 写入状态时克隆引用计数而不复制字符串。
    pub(crate) selected_item_id: Option<Arc<str>>,
    pub(crate) selected_season_id: Option<Arc<str>>,
    pub(crate) selected_episode_id: Option<Arc<str>>,
    pub(crate) season_episodes: Vec<MediaItem>,
    /// Whether the detail page synopsis is expanded (shows full text).
    pub(crate) detail_synopsis_expanded: bool,
    /// 详情页当前条目的收藏状态。`None` = 尚未加载；点击「收藏」后先乐观
    /// 更新，后台调用失败时回滚并写入 `error`。
    pub(crate) detail_is_favorite: Option<bool>,
    /// 详情页当前条目的已看状态（「已看」按钮，同上乐观更新 + 回滚）。
    pub(crate) detail_is_played: Option<bool>,
    /// 当前选中的人物 id（人物页）。
    pub(crate) selected_person_id: Option<Arc<str>>,
    pub(crate) person: Option<PersonPayload>,
    pub(crate) person_loading: bool,
    /// 当前选中的媒体库 id / 名称（媒体库浏览页）。
    pub(crate) selected_library_id: Option<Arc<str>>,
    pub(crate) selected_library_name: Option<Arc<str>>,
    pub(crate) library_sort: LibrarySort,
    pub(crate) library: Option<LibraryBrowsePayload>,
    pub(crate) library_loading: bool,
    /// 播放历史页的一页数据（「加载更多」时原地追加 items）。
    pub(crate) history: Option<PlayedHistoryPayload>,
    pub(crate) history_loading: bool,
    /// 收藏页的分类与数据。
    pub(crate) favorite_kind: FavoriteKind,
    pub(crate) favorites: Option<FavoritesPayload>,
    pub(crate) favorites_loading: bool,
    pub(crate) resolved_session: Option<ResolvedPlaybackSession>,
    pub(crate) player: Option<PlayerSession>,
    pub(crate) playback_state: Option<PlayerState>,
    pub(crate) images: HashMap<String, Arc<Image>>,
}

impl MainView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索所有媒体服务器"));
        let server_type_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Emby 或 Jellyfin"));
        let server_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("服务器名称"));
        let server_url_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("http://192.168.1.10:8096"));
        let server_username_input = cx.new(|cx| InputState::new(window, cx).placeholder("用户名"));
        let server_password_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("密码").masked(true));
        let server_proxy_url_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("代理地址（可选）"));
        let seek_slider = cx.new(|_| SliderState::new().min(0.0).max(1.0).step(0.01));

        // Theme: read the persisted preference from AppSettings ("Light" /
        // "Dark" / "Default", case-insensitive, ThemeMapping semantics).
        // "Default"/unrecognized follows the OS via the window appearance.
        let theme_mode = AppPaths::from_environment()
            .and_then(|paths| paths.store())
            .and_then(|store| store.settings())
            .map(|settings| ThemeMode::from_persisted(&settings.theme))
            .unwrap_or_default();
        let theme = theme_mode.resolve(is_dark_appearance(window.appearance()));

        let mut this = Self {
            page: Page::Home,
            nav_collapsed: false,
            nav_history: Vec::new(),
            theme,
            theme_mode,
            settings_module: SettingsModule::default(),
            search_input,
            server_type_input,
            server_name_input,
            server_url_input,
            server_username_input,
            server_password_input,
            server_proxy_url_input,
            seek_slider,
            servers: Vec::new(),
            loading: false,
            season_loading: false,
            error: None,
            home: None,
            search: None,
            detail: None,
            selected_item_id: None,
            selected_season_id: None,
            selected_episode_id: None,
            season_episodes: Vec::new(),
            detail_synopsis_expanded: false,
            detail_is_favorite: None,
            detail_is_played: None,
            selected_person_id: None,
            person: None,
            person_loading: false,
            selected_library_id: None,
            selected_library_name: None,
            library_sort: LibrarySort::default(),
            library: None,
            library_loading: false,
            history: None,
            history_loading: false,
            favorite_kind: FavoriteKind::default(),
            favorites: None,
            favorites_loading: false,
            resolved_session: None,
            player: None,
            playback_state: None,
            images: HashMap::new(),
        };

        cx.subscribe(
            &this.search_input,
            |this: &mut MainView, _, event: &InputEvent, cx: &mut Context<MainView>| {
                if matches!(
                    event,
                    InputEvent::PressEnter {
                        secondary: false,
                        ..
                    }
                ) {
                    this.perform_search(cx);
                }
            },
        )
        .detach();

        cx.subscribe(
            &this.seek_slider,
            |this: &mut MainView, _, event: &SliderEvent, cx: &mut Context<MainView>| {
                if let SliderEvent::Release(SliderValue::Single(ratio)) = event {
                    this.seek_to_ratio(*ratio, cx);
                }
            },
        )
        .detach();

        // FollowSystem: re-resolve the palette whenever the OS appearance
        // changes so the UI tracks the system light/dark setting live.
        cx.observe_window_appearance(
            window,
            |this: &mut MainView, window: &mut Window, cx: &mut Context<MainView>| {
                if this.theme_mode == ThemeMode::FollowSystem {
                    this.theme = this
                        .theme_mode
                        .resolve(is_dark_appearance(window.appearance()));
                    cx.notify();
                }
            },
        )
        .detach();

        this.reload_servers();
        if !this.servers.is_empty() {
            this.load_home(cx);
        }
        this
    }

    /// Returns the resolved theme colors. `ThemeColors` is `Copy`, so this is
    /// a cheap value copy that render code can use without borrowing `self`.
    pub(crate) fn theme(&self) -> ThemeColors {
        self.theme
    }

    /// Applies a new theme mode immediately (settings page theme picker).
    /// `system_dark` comes from the current window appearance so
    /// `FollowSystem` resolves against the live OS setting. The preference is
    /// also persisted to `AppSettings.theme` (`"Light"` / `"Dark"` /
    /// `"Default"`) so it survives a restart.
    pub(crate) fn apply_theme_mode(
        &mut self,
        mode: ThemeMode,
        system_dark: bool,
        cx: &mut Context<Self>,
    ) {
        self.theme_mode = mode;
        self.theme = mode.resolve(system_dark);
        cx.notify();
        self.persist_theme(mode);
    }

    /// Writes the theme preference through the existing `AppStore::save_settings`
    /// (`storage.rs`), mutating only the `theme` field of the loaded settings.
    fn persist_theme(&self, mode: ThemeMode) {
        let Ok(paths) = AppPaths::from_environment() else {
            return;
        };
        let Ok(store) = paths.store() else {
            return;
        };
        let Ok(mut settings) = store.settings() else {
            return;
        };
        settings.theme = mode.persisted().to_string();
        let _ = store.save_settings(settings);
    }

    /// Switches to `page`, recording the page being left in the navigation
    /// history so the header back button (`shell.navigation.back`) can return
    /// to it. `Page` is `Copy`; nothing is cloned.
    pub(crate) fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        if page != self.page {
            self.nav_history.push(self.page);
        }
        self.apply_page(page, cx);
    }

    /// Returns to the most recent page in the navigation history, if any.
    pub(crate) fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.nav_history.pop() {
            self.apply_page(page, cx);
        }
    }

    /// Toggles the navigation pane between the full pane and the collapsed
    /// icon column.
    pub(crate) fn toggle_nav_collapsed(&mut self, cx: &mut Context<Self>) {
        self.nav_collapsed = !self.nav_collapsed;
        cx.notify();
    }

    /// Applies a page switch without touching the navigation history.
    fn apply_page(&mut self, page: Page, cx: &mut Context<Self>) {
        match page {
            Page::Home => self.load_home(cx),
            // 播放历史与收藏每次进入都重新拉取，避免展示过期数据。
            Page::History => self.load_history(true, cx),
            Page::Favorites => self.load_favorites(cx),
            // 人物页与库浏览页的数据由 `open_person` / `open_library` 装载；
            // 从历史返回时直接复用已加载的状态。
            Page::Person | Page::Library => {
                self.page = page;
                cx.notify();
            }
            Page::Search | Page::Detail | Page::Player | Page::Servers | Page::Settings => {
                self.page = page;
                cx.notify();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_sort_maps_labels_to_server_sort_by_values() {
        assert_eq!(LibrarySort::Name.param(), "SortName");
        assert_eq!(LibrarySort::DateAdded.param(), "DateCreated");
        assert_eq!(LibrarySort::Rating.param(), "CommunityRating");
        assert_eq!(LibrarySort::PremiereDate.param(), "PremiereDate");
        assert_eq!(LibrarySort::ALL.len(), 4);
        assert_eq!(LibrarySort::default(), LibrarySort::Name);
        // 每个 pill 的 id 与文案都由 param/label 派生，必须唯一。
        let ids = LibrarySort::ALL.map(|sort| sort.param());
        assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(), 4);
        let labels = LibrarySort::ALL.map(|sort| sort.label());
        assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), 4);
    }

    #[test]
    fn favorite_kind_maps_to_include_item_types() {
        assert_eq!(FavoriteKind::All.param(), "Movie,Series");
        assert_eq!(FavoriteKind::Movie.param(), "Movie");
        assert_eq!(FavoriteKind::Series.param(), "Series");
        assert_eq!(FavoriteKind::ALL.len(), 3);
        assert_eq!(FavoriteKind::default(), FavoriteKind::All);
    }

    #[test]
    fn page_variants_are_distinct() {
        let pages = [
            Page::Home,
            Page::Search,
            Page::Detail,
            Page::Person,
            Page::Library,
            Page::History,
            Page::Favorites,
            Page::Player,
            Page::Servers,
            Page::Settings,
        ];
        assert_eq!(pages.iter().collect::<std::collections::HashSet<_>>().len(), pages.len());
    }
}
