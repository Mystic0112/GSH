//! Os painéis das abas Claude Code, Conta, Privacidade e Geral.

use super::cli::Account;
use super::code_theme;
use super::fonts::{self, FontTarget};
use super::widgets::{
    chip, dropdown, icon_choice, note, refresh_button, section, setting_row, small_button,
    text_field, value, Choice, Emphasis,
};
use super::{file, ClaudeState, Message, Pref, Switch};
use crate::settings::view::{choice, segmented, switch};
use crate::settings::Settings;
use crate::{labels, theme};
use gsh_config::schema::{AppThemeMode, TranscriptView, TranscriptWidth};
use gsh_harness::Json;
use iced::widget::{column, container, row, text, text_editor, Space};
use iced::{Element, Fill, Length};

type El<'a> = Element<'a, Message>;

/// `ClaudeCodeSettingSwitch`: o estado vem do `settings.json` lido.
fn file_switch(state: &ClaudeState, which: Switch) -> El<'static> {
    switch(
        which.is_on(&state.snapshot),
        Message::Switch(which, !which.is_on(&state.snapshot)),
    )
}

fn pref_switch(on: bool, pref: fn(bool) -> Pref) -> El<'static> {
    switch(on, Message::Pref(pref(!on)))
}

/// Aba Claude Code: preferências do chat (GSH) e chaves do `~/.claude/settings.json`.
pub fn claude_code(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let cfg = &s.current;
    let general = section(
        labels::SETTINGS_CC_SECTION_GENERAL,
        None,
        vec![
            setting_row(
                labels::SETTINGS_CC_CLASSIFY_TITLE,
                Some(labels::SETTINGS_CC_CLASSIFY_DESC),
                Some(pref_switch(
                    cfg.chat.classify_session_states,
                    Pref::ClassifySessionStates,
                )),
                None,
            ),
            setting_row(
                labels::SETTINGS_CC_SWITCH_MODELS_TITLE,
                Some(labels::SETTINGS_CC_SWITCH_MODELS_DESC),
                Some(file_switch(c, Switch::SwitchModels)),
                None,
            ),
        ],
    );
    let pickers = container(
        row![
            container(code_theme::picker(false, cfg.code.theme_light.as_deref())).width(Fill),
            Space::new().width(theme::SETTINGS_PICKER_GAP),
            container(code_theme::picker(true, cfg.code.theme_dark.as_deref())).width(Fill),
        ]
        .align_y(iced::Alignment::Start),
    )
    .padding(iced::Padding {
        bottom: f32::from(theme::SETTINGS_PICKER_BOTTOM),
        ..Default::default()
    });
    let code_font = c
        .code_font_draft
        .as_deref()
        .or(cfg.code_font.as_deref())
        .unwrap_or_default();
    let code_appearance = section(
        labels::SETTINGS_CC_SECTION_CODE_APPEARANCE,
        None,
        vec![
            pickers.into(),
            setting_row(
                labels::SETTINGS_CC_CODE_FONT_TITLE,
                Some(labels::SETTINGS_CC_CODE_FONT_DESC),
                Some(text_field(
                    code_font,
                    labels::SETTINGS_CC_CODE_FONT_PLACEHOLDER,
                    Message::CodeFontInput,
                    Message::CodeFontCommit,
                )),
                None,
            ),
        ],
    );
    let view = cfg.chat.transcript_view;
    let appearance = section(
        labels::SETTINGS_CC_SECTION_APPEARANCE,
        None,
        vec![setting_row(
            labels::SETTINGS_CC_TRANSCRIPT_TITLE,
            Some(labels::SETTINGS_CC_TRANSCRIPT_DESC),
            Some(segmented(row![
                choice(
                    labels::SETTINGS_CC_VIEW_NORMAL,
                    view == TranscriptView::Normal,
                    Message::Pref(Pref::TranscriptView(TranscriptView::Normal))
                ),
                choice(
                    labels::SETTINGS_CC_VIEW_THINKING,
                    view == TranscriptView::Thinking,
                    Message::Pref(Pref::TranscriptView(TranscriptView::Thinking))
                ),
                choice(
                    labels::SETTINGS_CC_VIEW_VERBOSE,
                    view == TranscriptView::Verbose,
                    Message::Pref(Pref::TranscriptView(TranscriptView::Verbose))
                ),
            ])),
            None,
        )],
    );
    let mut remote = Vec::new();
    // Motor CLI só em macOS/Linux (ver `ChatEngineRouter`).
    if !cfg!(windows) {
        remote.push(setting_row(
            labels::SETTINGS_CC_CHAT_REMOTE_CONTROL_TITLE,
            Some(labels::SETTINGS_CC_CHAT_REMOTE_CONTROL_DESC),
            Some(pref_switch(
                cfg.chat.remote_control,
                Pref::ChatRemoteControl,
            )),
            None,
        ));
    }
    remote.push(setting_row(
        labels::SETTINGS_CC_REMOTE_CONTROL_TITLE,
        Some(labels::SETTINGS_CC_REMOTE_CONTROL_DESC),
        Some(file_switch(c, Switch::RemoteControl)),
        None,
    ));
    let remote_control = section(labels::SETTINGS_CC_SECTION_REMOTE_CONTROL, None, remote);
    let sessions = section(
        labels::SETTINGS_CC_SECTION_SESSIONS,
        None,
        vec![
            setting_row(
                labels::SETTINGS_CC_PROMPT_SUGGESTIONS_TITLE,
                Some(labels::SETTINGS_CC_PROMPT_SUGGESTIONS_DESC),
                Some(file_switch(c, Switch::PromptSuggestions)),
                None,
            ),
            setting_row(
                labels::SETTINGS_CC_OUTPUT_STYLE_TITLE,
                Some(labels::SETTINGS_CC_OUTPUT_STYLE_DESC),
                Some(output_style_dropdown(c)),
                None,
            ),
            setting_row(
                labels::SETTINGS_CC_CROSS_SESSION_TITLE,
                Some(labels::SETTINGS_CC_CROSS_SESSION_DESC),
                Some(cross_session_dropdown(c)),
                None,
            ),
            setting_row(
                labels::SETTINGS_CC_SANDBOX_TITLE,
                Some(labels::SETTINGS_CC_SANDBOX_DESC),
                Some(file_switch(c, Switch::Sandbox)),
                None,
            ),
            setting_row(
                labels::SETTINGS_CC_SANDBOX_STRICT_TITLE,
                Some(labels::SETTINGS_CC_SANDBOX_STRICT_DESC),
                Some(file_switch(c, Switch::SandboxStrict)),
                None,
            ),
        ],
    );
    let security = section(
        labels::SETTINGS_CC_SECTION_SECURITY,
        None,
        vec![setting_row(
            labels::SETTINGS_CC_PROTECTED_HOSTS_TITLE,
            Some(labels::SETTINGS_CC_PROTECTED_HOSTS_DESC),
            Some(text_field(
                &c.hosts_draft,
                "prod, homolog.gorila.com",
                Message::HostsInput,
                Message::HostsCommit,
            )),
            None,
        )],
    );
    let browser = section(
        labels::SETTINGS_CC_SECTION_BROWSER,
        None,
        vec![setting_row(
            labels::SETTINGS_CC_LINKS_IN_APP_TITLE,
            Some(labels::SETTINGS_CC_LINKS_IN_APP_DESC),
            Some(pref_switch(cfg.links.open_in_app, Pref::OpenLinksInApp)),
            None,
        )],
    );
    column![
        general,
        code_appearance,
        appearance,
        remote_control,
        sessions,
        security,
        browser
    ]
    .width(Fill)
    .into()
}

fn narrow_dropdown<'a>(
    options: Vec<Choice>,
    selected: Choice,
    on_pick: impl Fn(Choice) -> Message + 'a,
) -> El<'a> {
    dropdown(
        options,
        selected,
        Length::Fixed(theme::SETTINGS_DROPDOWN_WIDTH),
        theme::SETTINGS_DROPDOWN_PADDING,
        theme::SETTINGS_DROPDOWN_TEXT,
        theme::SETTINGS_DROPDOWN_ICON,
        on_pick,
    )
}

fn string_at(state: &ClaudeState, key: &str) -> Option<String> {
    match file::value(&state.snapshot, key) {
        Some(Json::Str(text)) => Some(text.clone()),
        _ => None,
    }
}

/// "Estilo de saída" (`outputStyle`): Padrão (chave ausente), os embutidos e os de
/// `~/.claude/output-styles`.
fn output_style_dropdown(state: &ClaudeState) -> El<'static> {
    let current = string_at(state, file::keys::OUTPUT_STYLE);
    let standard = Choice {
        value: None,
        label: labels::SETTINGS_CC_OUTPUT_STYLE_DEFAULT.to_owned(),
    };
    let named = |name: &String| Choice {
        value: Some(name.clone()),
        label: name.clone(),
    };
    let mut options = vec![standard.clone()];
    options.extend(state.output_styles.iter().map(named));
    let selected = match &current {
        Some(name) => named(name),
        None => standard,
    };
    if !options.contains(&selected) {
        options.push(selected.clone());
    }
    narrow_dropdown(options, selected, |picked| {
        Message::OutputStyle(picked.value)
    })
}

/// "Mensagens de outras sessões" (`crossSessionInbound`): Aceitar, Segurar ou Recusar. Ausente
/// = o GSH grava "accept" no boot; mostra como Aceitar.
fn cross_session_dropdown(state: &ClaudeState) -> El<'static> {
    let known = [
        ("accept", labels::SETTINGS_CC_CROSS_SESSION_ACCEPT),
        ("hold", labels::SETTINGS_CC_CROSS_SESSION_HOLD),
        ("refuse", labels::SETTINGS_CC_CROSS_SESSION_REFUSE),
    ];
    let current = string_at(state, file::keys::CROSS_SESSION_INBOUND)
        .unwrap_or_else(|| String::from("accept"));
    let as_choice = |(value, label): (&str, &str)| Choice {
        value: Some(value.to_owned()),
        label: label.to_owned(),
    };
    let mut options: Vec<Choice> = known.iter().copied().map(as_choice).collect();
    let selected = known
        .iter()
        .find(|(value, _)| *value == current)
        .copied()
        .map_or_else(
            || Choice {
                value: Some(current.clone()),
                label: current.clone(),
            },
            as_choice,
        );
    if !options.contains(&selected) {
        options.push(selected.clone());
    }
    narrow_dropdown(options, selected, |picked| {
        Message::CrossSession(picked.value.unwrap_or_default())
    })
}

/// Aba Privacidade: o GSH não manda nada para lugar nenhum; o que dá para desligar é o envio
/// do próprio Claude Code (`DISABLE_*` no `env` do `~/.claude/settings.json`).
pub fn privacy(s: &Settings) -> El<'_> {
    let c = &s.claude;
    column![section(
        labels::SETTINGS_PRIVACY_TITLE,
        None,
        vec![
            setting_row(
                labels::SETTINGS_PRIVACY_GSH_TELEMETRY,
                Some(labels::SETTINGS_PRIVACY_GSH_TELEMETRY_DESC),
                None,
                None,
            ),
            setting_row(
                labels::SETTINGS_PRIVACY_CC_TELEMETRY,
                Some(labels::SETTINGS_PRIVACY_CC_TELEMETRY_DESC),
                Some(file_switch(c, Switch::DisableTelemetry)),
                None,
            ),
            setting_row(
                labels::SETTINGS_PRIVACY_CC_ERRORS,
                Some(labels::SETTINGS_PRIVACY_CC_ERRORS_DESC),
                Some(file_switch(c, Switch::DisableErrorReporting)),
                None,
            ),
        ],
    )]
    .width(Fill)
    .into()
}

/// Aba Geral do lado Claude: Aparência (tema, fontes da interface e do chat, tamanho, largura e
/// movimento reduzido) e Notificações (`general_panel.dart`).
pub fn general(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let mode = s.theme_mode;
    let size = s.interface_size.trim().parse::<f64>().ok();
    let width = s.current.chat.transcript_width;
    let size_choice = |label: &'static str, px: f64| {
        choice(
            label,
            size == Some(px),
            Message::Pref(Pref::InterfaceSize(px)),
        )
    };
    let width_choice = |label: &'static str, value: TranscriptWidth| {
        choice(
            label,
            width == value,
            Message::Pref(Pref::TranscriptWidth(value)),
        )
    };
    let theme_choice = |icon, tip, value: AppThemeMode| {
        icon_choice(icon, tip, mode == value, Message::Theme(value))
    };
    let appearance = section(
        labels::SETTINGS_GC_SECTION_APPEARANCE,
        None,
        vec![
            setting_row(
                labels::SETTINGS_GC_THEME,
                None,
                Some(segmented(row![
                    theme_choice(
                        labels::ICON_DESKTOP_WINDOWS,
                        labels::SETTINGS_GC_THEME_SYSTEM,
                        AppThemeMode::System
                    ),
                    theme_choice(
                        labels::ICON_LIGHT_MODE,
                        labels::SETTINGS_GC_THEME_LIGHT,
                        AppThemeMode::Light
                    ),
                    theme_choice(
                        labels::ICON_DARK_MODE,
                        labels::SETTINGS_GC_THEME_DARK,
                        AppThemeMode::Dark
                    ),
                ])),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_UI_FONT,
                Some(labels::SETTINGS_GC_UI_FONT_DESC),
                Some(fonts::font_field(
                    s.current.interface_font.as_deref(),
                    labels::SETTINGS_GC_UI_FONT_HINT,
                    FontTarget::Interface,
                )),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_CHAT_FONT,
                Some(labels::SETTINGS_GC_CHAT_FONT_DESC),
                Some(fonts::font_field(
                    s.current.chat.font.as_deref(),
                    labels::SETTINGS_GC_CHAT_FONT_DEFAULT,
                    FontTarget::Chat,
                )),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_TEXT_SIZE,
                Some(labels::SETTINGS_GC_TEXT_SIZE_DESC),
                Some(segmented(row![
                    size_choice(labels::SETTINGS_GC_SIZE_SMALL, 13.0),
                    size_choice(labels::SETTINGS_GC_SIZE_MEDIUM, 14.0),
                    size_choice(labels::SETTINGS_GC_SIZE_LARGE, 16.0),
                ])),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_TRANSCRIPT_WIDTH,
                Some(labels::SETTINGS_GC_TRANSCRIPT_WIDTH_DESC),
                Some(segmented(row![
                    width_choice(labels::SETTINGS_GC_WIDTH_NARROW, TranscriptWidth::Narrow),
                    width_choice(labels::SETTINGS_GC_WIDTH_MEDIUM, TranscriptWidth::Medium),
                    width_choice(labels::SETTINGS_GC_WIDTH_WIDE, TranscriptWidth::Wide),
                ])),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_REDUCED_MOTION,
                Some(labels::SETTINGS_GC_REDUCED_MOTION_DESC),
                Some(file_switch(c, Switch::ReducedMotion)),
                None,
            ),
        ],
    );
    let notifications = section(
        labels::SETTINGS_GC_SECTION_NOTIFICATIONS,
        None,
        vec![
            setting_row(
                labels::SETTINGS_GC_NOTIFY_TURN_DONE,
                Some(labels::SETTINGS_GC_NOTIFY_TURN_DONE_DESC),
                Some(pref_switch(s.notifications, Pref::Notifications)),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_NOTIFY_PERMISSION,
                Some(labels::SETTINGS_GC_NOTIFY_PERMISSION_DESC),
                Some(pref_switch(s.permission_sound, Pref::PermissionSound)),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_PUSH_INPUT_NEEDED,
                Some(labels::SETTINGS_GC_PUSH_INPUT_NEEDED_DESC),
                Some(file_switch(c, Switch::PushInputNeeded)),
                None,
            ),
            setting_row(
                labels::SETTINGS_GC_PUSH_AGENT,
                Some(labels::SETTINGS_GC_PUSH_AGENT_DESC),
                Some(file_switch(c, Switch::PushAgent)),
                None,
            ),
        ],
    );
    column![appearance, notifications].width(Fill).into()
}

/// Aba Conta: Perfil (as instruções do `~/.claude/CLAUDE.md`) e o login do Claude Code nesta
/// máquina (`claude auth status`), com Entrar (terminal com `claude auth login`) e Sair.
pub fn account(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let mut rows: Vec<El<'_>> = match &c.account {
        Some(Err(error)) => vec![note(labels::claude_info_error(error), true)],
        None => vec![note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false)],
        Some(Ok(account)) => account_rows(account),
    };
    if let Some(error) = &c.logout_error {
        rows.push(note(labels::claude_info_error(error), true));
    }
    column![
        if cfg!(target_os = "linux") {
            profile_list(s)
        } else {
            Space::new().into()
        },
        section(
            labels::SETTINGS_ACCOUNT_PROFILE,
            None,
            vec![instructions(c)]
        ),
        section(
            labels::SETTINGS_ACCOUNT_TITLE,
            Some(refresh_button(c.account_loading, Message::RefreshAccount)),
            rows,
        ),
    ]
    .width(Fill)
    .into()
}

fn profile_list(s: &Settings) -> El<'_> {
    let profiles = &s.current.claude;
    let c = &s.claude;
    let choose = |name: Option<String>| {
        let active = profiles.active_profile == name;
        small_button(
            if active { "Em uso" } else { "Usar" },
            None,
            Emphasis::Outline,
            (!active).then_some(Message::SelectProfile(name)),
        )
    };
    let mut rows = vec![setting_row(
        "Conta padrão",
        Some("Selecione um perfil para os próximos chats e agentes Claude abertos pelo GSH."),
        Some(choose(None)),
        None,
    )];
    for (name, directory) in &profiles.profiles {
        rows.push(
            row![
                column![
                    text(name)
                        .font(theme::FONT_BODY)
                        .size(theme::SETTINGS_NAV_TEXT_SIZE),
                    note(directory.clone(), false),
                ]
                .width(Fill),
                choose(Some(name.clone())),
                small_button(
                    labels::SETTINGS_ACCOUNT_LOGIN,
                    None,
                    Emphasis::Outline,
                    Some(Message::LoginProfile(name.clone())),
                ),
                small_button(
                    "Desvincular",
                    None,
                    Emphasis::Outline,
                    Some(Message::RemoveProfile(name.clone())),
                ),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([theme::SETTINGS_ROW_VERTICAL, 0])
            .into(),
        );
    }
    rows.push(
        column![
            text_field(
                &c.profile_name,
                "Nome do perfil",
                Message::ProfileName,
                Message::AddProfile
            ),
            text_field(
                &c.profile_directory,
                "/caminho/da/pasta",
                Message::ProfileDirectory,
                Message::AddProfile,
            ),
            small_button(
                "Adicionar perfil",
                None,
                Emphasis::Primary,
                (!c.profile_name.trim().is_empty() && !c.profile_directory.trim().is_empty())
                    .then_some(Message::AddProfile),
            ),
        ]
        .spacing(8)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0])
        .into(),
    );
    if let Some(error) = &c.profile_error {
        rows.push(note(error.clone(), true));
    }
    section("Perfis do Claude Code", None, rows)
}

fn account_rows(account: &Account) -> Vec<El<'static>> {
    let p = theme::palette();
    let (title, desc, icon, color) = if account.logged_in {
        (
            labels::SETTINGS_ACCOUNT_LOGGED_IN,
            labels::SETTINGS_ACCOUNT_LOGGED_IN_DESC,
            labels::ICON_CHECK_CIRCLE,
            p.ok,
        )
    } else {
        (
            labels::SETTINGS_ACCOUNT_LOGGED_OUT,
            labels::SETTINGS_ACCOUNT_LOGGED_OUT_DESC,
            labels::ICON_CANCEL,
            p.text3,
        )
    };
    let action: El<'static> = if account.logged_in {
        row![
            small_button(
                labels::SETTINGS_ACCOUNT_SWITCH,
                None,
                Emphasis::Outline,
                Some(Message::SwitchAccountAsk),
            ),
            small_button(
                labels::SETTINGS_ACCOUNT_LOGOUT,
                None,
                Emphasis::Outline,
                Some(Message::LogoutAsk),
            ),
        ]
        .spacing(8.0)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        small_button(
            labels::SETTINGS_ACCOUNT_LOGIN,
            None,
            Emphasis::Primary,
            Some(Message::RunInTerminal(String::from("claude auth login"))),
        )
    };
    let mut rows = vec![setting_row(
        title,
        Some(desc),
        Some(action),
        Some(
            text(icon)
                .font(theme::FONT_ICON)
                .size(theme::SETTINGS_ROW_LEADING_ICON)
                .color(color)
                .into(),
        ),
    )];
    if account.logged_in {
        let mut detail = |title: &'static str, trailing: El<'static>| {
            rows.push(setting_row(title, None, Some(trailing), None));
        };
        if let Some(v) = &account.email {
            detail(labels::SETTINGS_ACCOUNT_EMAIL, value(v.clone(), false));
        }
        if let Some(v) = &account.org_name {
            detail(
                labels::SETTINGS_ACCOUNT_ORGANIZATION,
                value(v.clone(), false),
            );
        }
        if let Some(v) = &account.org_id {
            detail(labels::SETTINGS_ACCOUNT_ORG_ID, value(v.clone(), true));
        }
        if let Some(v) = &account.subscription_type {
            detail(labels::SETTINGS_ACCOUNT_PLAN, chip(v.clone()));
        }
        if let Some(v) = account
            .auth_method
            .as_ref()
            .or(account.api_provider.as_ref())
        {
            detail(labels::SETTINGS_ACCOUNT_METHOD, value(v.clone(), false));
        }
    }
    rows
}

/// "Instruções para o Claude": o `~/.claude/CLAUDE.md` num campo de 4 a 12 linhas.
fn instructions(c: &ClaudeState) -> El<'_> {
    let p = theme::palette();
    let mut block = column![
        text(labels::SETTINGS_ACCOUNT_INSTRUCTIONS)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_NAV_TEXT_SIZE)
            .line_height(iced::widget::text::LineHeight::Relative(
                theme::LINE_HEIGHT_BODY
            ))
            .color(p.text),
        Space::new().height(theme::SETTINGS_ROW_TITLE_GAP),
        text(labels::SETTINGS_ACCOUNT_INSTRUCTIONS_DESC)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DESC_SIZE)
            .line_height(iced::widget::text::LineHeight::Relative(
                theme::LINE_HEIGHT_BODY
            ))
            .color(p.text3),
        Space::new().height(theme::SETTINGS_TEXTAREA_GAP),
    ];
    if let Some(ins) = &c.instructions {
        let lines = ins.content.line_count().clamp(
            theme::SETTINGS_TEXTAREA_MIN_LINES,
            theme::SETTINGS_TEXTAREA_MAX_LINES,
        );
        let line_height = theme::SIZE_MONO * theme::LINE_HEIGHT_MONO;
        let [v_pad, _] = theme::SETTINGS_TEXT_FIELD_PADDING;
        let padding = f32::from(v_pad);
        block = block.push(
            text_editor(&ins.content)
                .placeholder(labels::SETTINGS_ACCOUNT_INSTRUCTIONS_HINT)
                .on_action(Message::Instructions)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_MONO)
                .line_height(iced::widget::text::LineHeight::Relative(
                    theme::LINE_HEIGHT_MONO,
                ))
                .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
                .height(Length::Fixed(
                    lines as f32 * line_height + 2.0 * padding + 2.0,
                ))
                .style(editor_style),
        );
    }
    if let Some(error) = &c.instructions_error {
        block = block
            .push(Space::new().height(theme::SETTINGS_TEXTAREA_ERROR_GAP))
            .push(note(labels::claude_info_error(error), true));
    }
    container(block)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0])
        .width(Fill)
        .into()
}

fn editor_style(_: &iced::Theme, status: text_editor::Status) -> text_editor::Style {
    let p = theme::palette();
    text_editor::Style {
        background: iced::Background::Color(iced::Color {
            a: theme::SETTINGS_TEXT_FIELD_FILL_ALPHA,
            ..p.border
        }),
        border: iced::Border {
            color: if matches!(status, text_editor::Status::Focused { .. }) {
                p.accent
            } else {
                p.border
            },
            width: 1.0,
            radius: theme::SETTINGS_TEXTAREA_RADIUS.into(),
        },
        placeholder: p.text3,
        value: p.text,
        selection: p.accent_soft,
    }
}
