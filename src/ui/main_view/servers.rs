//! 服务器页：已保存服务器列表 + 添加服务器表单（全部卡片化）。
//!
//! 数据逻辑（读取 / 激活 / 添加 / 删除）保持不变；本文件只负责呈现。
//! 文案与颜色统一走 `self.theme()` 令牌。

use super::components::{
    FluentButtonVariant, fluent_button, fluent_card, fluent_empty_state, fluent_section_title,
};
use super::state::MainView;
use crate::models::{LoginInput, SaveServerInput, SavedServerSummary};
use crate::services::{AppPaths, ServerService};
use crate::ui::theme::ThemeColors;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{AnyElement, Context, Entity, FontWeight, SharedString, div, px, prelude::*};
use std::sync::Arc;

/// 表单卡宽度（逻辑 px）。
const FORM_CARD_WIDTH: f32 = 460.0;
/// 卡片圆角。
const CARD_RADIUS: f32 = 8.0;
/// 按钮高度（与 Fluent 按钮一致：36 逻辑 px）。
const BUTTON_HEIGHT: f32 = 36.0;

impl MainView {
    pub(crate) fn reload_servers(&mut self) {
        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.error = Some(err);
                return;
            }
        };
        let service = match ServerService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.error = Some(err);
                return;
            }
        };
        match service.list() {
            Ok(servers) => {
                self.servers = servers;
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }

    pub(crate) fn select_server(&mut self, server_id: Arc<str>, cx: &mut Context<Self>) {
        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match ServerService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        // 点击路径（低频）：`Arc<str>` → `String` 一次进入存储层。
        match service.set_active(server_id.to_string()) {
            Ok(_) => {
                self.reload_servers();
                self.load_home(cx);
            }
            Err(err) => {
                self.error = Some(err);
                cx.notify();
            }
        }
    }

    pub(crate) fn add_server(&mut self, cx: &mut Context<Self>) {
        let server_type = self.server_type_input.read(cx).value().to_string();
        let name = self.server_name_input.read(cx).value().to_string();
        let url = self.server_url_input.read(cx).value().to_string();
        let username = self.server_username_input.read(cx).value().to_string();
        let password = self.server_password_input.read(cx).value().to_string();
        let proxy_url = self.server_proxy_url_input.read(cx).value().to_string();
        let proxy_url = proxy_url.trim();
        let proxy_url = (!proxy_url.is_empty()).then(|| proxy_url.to_string());

        self.loading = true;
        self.error = None;
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
        let service = match ServerService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.loading = false;
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let view = cx.entity();
        let login = LoginInput {
            server_type,
            name,
            url,
            username,
            password,
            use_system_proxy: true,
            // move：`proxy_url` 只在这里被消费，无需复制。
            proxy_url,
        };

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    // `login_result` 恰好被消费一次来构建 `SaveServerInput`，
                    // 字段全部 move 而非 clone。
                    let login_result = service.login(login)?;
                    let saved = service.save(SaveServerInput {
                        id: login_result.id,
                        name: login_result.name,
                        url: login_result.url,
                        username: login_result.username,
                        user_id: login_result.user_id,
                        access_token: login_result.access_token,
                        use_system_proxy: login_result.use_system_proxy,
                        proxy_url: login_result.proxy_url,
                        icon_url: None,
                        icon_name: None,
                    })?;
                    Ok::<_, String>(saved)
                })
                .await;

            view.update(cx, move |this, cx| {
                match result {
                    Ok(_) => {
                        this.reload_servers();
                        this.loading = false;
                        this.load_home(cx);
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

    pub(crate) fn delete_server(&mut self, server_id: Arc<str>, cx: &mut Context<Self>) {
        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(err) => {
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        let service = match ServerService::new(&paths) {
            Ok(service) => service,
            Err(err) => {
                self.error = Some(err);
                cx.notify();
                return;
            }
        };
        // 点击路径（低频）：`Arc<str>` → `String` 一次进入存储层。
        match service.delete(server_id.to_string()) {
            Ok(_) => {
                self.home = None;
                self.detail = None;
                self.player = None;
                self.playback_state = None;
                self.resolved_session = None;
                self.reload_servers();
                cx.notify();
            }
            Err(err) => {
                self.error = Some(err);
                cx.notify();
            }
        }
    }

    /// 服务器页内容：页面标题、已保存服务器列表、添加服务器表单卡。
    pub(crate) fn servers_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let submit_view = view.clone();
        let theme = self.theme();
        let mut root = div().flex().flex_col().gap_6();

        root = root.child(fluent_section_title(theme, "媒体服务器"));

        if self.servers.is_empty() {
            root = root.child(fluent_empty_state(
                theme,
                "📡",
                "还没有保存的服务器",
                "在下方添加你的第一台 Emby 或 Jellyfin 服务器",
                None,
            ));
        } else {
            root = root
                .child(fluent_section_title(theme, "已保存的服务器"))
                .child(
                    div().flex().flex_col().gap_3().children(
                        self.servers
                            .iter()
                            .map(|server| self.server_list_item(server, view.clone())),
                    ),
                );
        }

        // 添加服务器表单卡：标签 + 输入框 + 主按钮。
        let form = fluent_card(theme)
            .w(px(FORM_CARD_WIDTH))
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .child(fluent_section_title(theme, "添加服务器"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(form_field(
                        theme,
                        "服务器类型",
                        &self.server_type_input,
                        "server-type",
                    ))
                    .child(form_field(
                        theme,
                        "服务器名称",
                        &self.server_name_input,
                        "server-name",
                    ))
                    .child(form_field(
                        theme,
                        "服务器地址",
                        &self.server_url_input,
                        "server-url",
                    ))
                    .child(form_field(
                        theme,
                        "用户名",
                        &self.server_username_input,
                        "server-username",
                    ))
                    .child(form_field(
                        theme,
                        "密码",
                        &self.server_password_input,
                        "server-password",
                    ))
                    .child(form_field(
                        theme,
                        "代理地址（可选）",
                        &self.server_proxy_url_input,
                        "server-proxy-url",
                    )),
            )
            .child(fluent_button(
                theme,
                "submit-server",
                "连接",
                FluentButtonVariant::Accent,
                move |_, _, cx| {
                    let _ = submit_view.update(cx, |view, cx| view.add_server(cx));
                },
            ))
            .when(self.loading, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child("连接中..."),
                )
            });

        root = root.child(form);
        root.into_any_element()
    }

    /// 单个已保存服务器卡：名称 / 状态 / 地址 / 数量 + 操作按钮。当前使用
    /// 的服务器用 accent 描边强调。
    fn server_list_item(&self, server: &SavedServerSummary, view: Entity<Self>) -> AnyElement {
        let theme = self.theme();
        // 渲染路径：`Arc<str>` id，闭包只克隆引用计数而非字符串本体。
        let server_id: Arc<str> = Arc::from(server.id.as_str());
        let activate_server_id = Arc::clone(&server_id);
        let delete_server_id = Arc::clone(&server_id);
        let activate_view = view.clone();
        let delete_view = view;
        let server_name = SharedString::from(server.name.as_str());
        let server_url = SharedString::from(server.url.as_str());
        let active = server.active;
        let status = if active { "当前使用" } else { "未使用" };
        // 当前使用的服务器用 accent 强调；未使用的退回次级文字色。
        let status_color = if active {
            theme.accent
        } else {
            theme.text_secondary
        };
        let counts = format!(
            "{} 部电影 / {} 部剧集 / {} 集",
            server.movie_count.unwrap_or_default(),
            server.series_count.unwrap_or_default(),
            server.episode_count.unwrap_or_default()
        );
        let activate_label = if active {
            "设为当前"
        } else {
            "切换到此服务器"
        };

        div()
            .id(format!("server-card-{server_id}"))
            .flex()
            .flex_col()
            .gap_3()
            .rounded(px(CARD_RADIUS))
            .bg(theme.bg_card)
            .border_1()
            .border_color(if active { theme.accent } else { theme.border })
            .p_4()
            .hover(move |this| this.bg(theme.bg_card_hover))
            // 名称 + 状态行
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(server_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(status),
                    ),
            )
            // 地址行
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(server_url),
            )
            // 数量行
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(SharedString::from(counts)),
            )
            // 操作行
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(fluent_button(
                        theme,
                        SharedString::from(format!("activate-server-{server_id}")),
                        activate_label,
                        FluentButtonVariant::Standard,
                        move |_, _, cx| {
                            let _ = activate_view.update(cx, |view, cx| {
                                view.select_server(Arc::clone(&activate_server_id), cx)
                            });
                        },
                    ))
                    .child(danger_button(
                        theme,
                        SharedString::from(format!("delete-server-{delete_server_id}")),
                        "删除",
                        move |_, _, cx| {
                            let _ = delete_view.update(cx, |view, cx| {
                                view.delete_server(Arc::clone(&delete_server_id), cx)
                            });
                        },
                    )),
            )
            .into_any_element()
    }
}

/// 危险次按钮（删除）：Fluent 次按钮形态 + danger 文字色。
fn danger_button(
    theme: ThemeColors,
    id: SharedString,
    label: &str,
    handler: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(BUTTON_HEIGHT))
        .px_4()
        .rounded_md()
        .bg(theme.bg_control)
        .border_1()
        .border_color(theme.border)
        .text_color(theme.danger)
        .text_sm()
        .cursor_pointer()
        .hover(move |this| this.bg(theme.bg_control_hover))
        .child(SharedString::from(label))
        .on_click(handler)
        .into_any_element()
}

/// 表单字段：小标签 + 输入框。
fn form_field(
    theme: ThemeColors,
    label: &str,
    input: &Entity<InputState>,
    id: &'static str,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(theme.text_secondary)
                .child(SharedString::from(label)),
        )
        .child(Input::new(input).id(id).w_full())
        .into_any_element()
}
