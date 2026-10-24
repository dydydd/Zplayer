//! 设置页：7 模块 pill 条 + 每模块内容（常规 / 播放器 / 媒体库 / 外部服务 /
//! AI / 快捷键 / 关于）。

use super::components::{fluent_card, fluent_pill};
use super::state::MainView;
use crate::ui::theme::{ThemeMode, is_dark_appearance};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{AnyElement, Context, FontWeight, SharedString, div, prelude::*};

/// 设置模块（pill 条中的 7 个模块）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum SettingsModule {
    #[default]
    General,
    Player,
    MediaLibrary,
    ExternalServices,
    Ai,
    KeyboardShortcuts,
    About,
}

impl SettingsModule {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            SettingsModule::General => "常规",
            SettingsModule::Player => "播放器",
            SettingsModule::MediaLibrary => "媒体库",
            SettingsModule::ExternalServices => "外部服务",
            SettingsModule::Ai => "AI",
            SettingsModule::KeyboardShortcuts => "快捷键",
            SettingsModule::About => "关于",
        }
    }

    pub(crate) fn all() -> [SettingsModule; 7] {
        [
            SettingsModule::General,
            SettingsModule::Player,
            SettingsModule::MediaLibrary,
            SettingsModule::ExternalServices,
            SettingsModule::Ai,
            SettingsModule::KeyboardShortcuts,
            SettingsModule::About,
        ]
    }
}

impl MainView {
    /// 渲染设置页。
    pub(crate) fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.settings_module;

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_6()
            .child(self.settings_module_strip(cx))
            .child(match selected {
                SettingsModule::General => self.general_settings(cx),
                SettingsModule::Player => self.settings_placeholder("播放器"),
                SettingsModule::MediaLibrary => self.settings_placeholder("媒体库"),
                SettingsModule::ExternalServices => self.settings_placeholder("外部服务"),
                SettingsModule::Ai => self.settings_placeholder("AI"),
                SettingsModule::KeyboardShortcuts => self.settings_placeholder("快捷键"),
                SettingsModule::About => self.about_settings(),
            })
            .into_any_element()
    }

    /// 7 模块 pill 条（横向滚动）：点击切换模块。
    fn settings_module_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let selected = self.settings_module;

        let mut strip = div().flex().gap_2().overflow_x_scrollbar();
        for module in SettingsModule::all() {
            let active = module == selected;
            let pill = fluent_pill(
                theme,
                SharedString::from(format!("settings-pill-{:?}", module)),
                module.label(),
                active,
                cx.listener(move |view: &mut MainView, _event, _window, cx| {
                    view.apply_settings_module(module, cx);
                }),
            );
            strip = strip.child(pill);
        }

        strip.into_any_element()
    }

    /// 切换当前设置模块。
    pub(crate) fn apply_settings_module(&mut self, module: SettingsModule, cx: &mut Context<Self>) {
        if self.settings_module != module {
            self.settings_module = module;
            cx.notify();
        }
    }

    /// 常规模块：主题三 pill + 占位卡。
    fn general_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.settings_card(
                "主题",
                "选择应用的配色主题",
                self.theme_picker(cx),
            ))
            .child(self.settings_card(
                "应用字体",
                "尚未开放",
                self.placeholder_value(),
            ))
            .child(self.settings_card(
                "备份与恢复",
                "导入/导出配置（尚未开放）",
                self.placeholder_value(),
            ))
            .into_any_element()
    }

    /// 主题三 pill（浅色 / 深色 / 跟随系统）：选中 accent 填充，点击立即
    /// 应用并持久化。
    fn theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let mut row = div().flex().flex_row().gap_2();
        for (mode, label) in [
            (ThemeMode::Light, "浅色"),
            (ThemeMode::Dark, "深色"),
            (ThemeMode::FollowSystem, "跟随系统"),
        ] {
            let selected = self.theme_mode == mode;
            let pill = fluent_pill(
                theme,
                SharedString::from(format!("settings-theme-{:?}", mode)),
                label,
                selected,
                cx.listener(move |view: &mut MainView, _event, window, cx| {
                    let system_dark = is_dark_appearance(window.appearance());
                    view.apply_theme_mode(mode, system_dark, cx);
                }),
            );
            row = row.child(pill);
        }
        row.into_any_element()
    }

    /// 关于模块：产品卡（名称 + 版本 + 版权）+ 开源声明占位。
    fn about_settings(&self) -> AnyElement {
        let theme = self.theme();
        let version = env!("CARGO_PKG_VERSION");
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.settings_card(
                "Zplayer",
                "原生 Rust 媒体客户端",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_primary)
                            .child(format!("版本 {version}")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child("© 2026 Zplayer"),
                    )
                    .into_any_element(),
            ))
            .child(self.settings_card(
                "开源声明",
                "第三方开源组件许可（尚未开放）",
                self.placeholder_value(),
            ))
            .into_any_element()
    }

    /// 尚未迁移模块的占位卡。
    fn settings_placeholder(&self, name: &str) -> AnyElement {
        self.settings_card(
            &format!("{name}：迁移中"),
            "该模块正在迁移中，敬请期待",
            self.placeholder_value(),
        )
    }

    /// 占位卡的右侧内容（一个「—」）。
    fn placeholder_value(&self) -> AnyElement {
        let theme = self.theme();
        div()
            .text_sm()
            .text_color(theme.text_secondary)
            .child("—")
            .into_any_element()
    }

    /// 横向设置行卡：左侧标题 + 描述，右侧内容。基于 `fluent_card`
    /// （`bg_card` 底、圆角 8px、1px 描边、16px 内边距、hover 提亮）。
    fn settings_card(
        &self,
        title: &str,
        desc: &str,
        content: AnyElement,
    ) -> AnyElement {
        let theme = self.theme();
        fluent_card(theme)
            .flex()
            .gap_4()
            .child(
                div()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(SharedString::from(title)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(SharedString::from(desc)),
                    ),
            )
            .child(div().flex_1())
            .child(content)
            .into_any_element()
    }
}
