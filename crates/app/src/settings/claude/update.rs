//! Tratamento das mensagens e persistência de todas as abas do lado Claude.

use super::fonts::{FontPickerState, FontTarget};
use super::{
    cli, file, files, prefs, profiles, themes, widgets, ClaudeState, Instructions, Message,
    NamePromptKind, Pref, Tab,
};
use crate::labels;
use crate::settings::{Error, Settings};
use gsh_config::schema::SoundEvent;
use gsh_harness::Json;
use iced::widget::text_editor;
use iced::Task;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(in crate::settings) fn update(s: &mut Settings, message: Message) -> Task<Message> {
    match message {
        Message::Select(tab) => {
            let task = commit_pending(s);
            s.claude.tab = tab;
            s.claude.catalog_discover = false;
            s.claude.catalog_search.clear();
            s.claude.catalog_add_menu = false;
            s.claude.selected_marketplace = None;
            let load = enter_tab(&mut s.claude, tab);
            Task::batch([task, load])
        }
        Message::Switch(which, on) => {
            set_json_key(&mut s.claude, which.spec().key, which.written(on))
        }
        Message::OutputStyle(style) => {
            let value = style.filter(|v| !v.is_empty()).map(Json::Str);
            set_json_key(&mut s.claude, file::keys::OUTPUT_STYLE, value)
        }
        Message::CrossSession(mode) => {
            let value = (!mode.is_empty()).then_some(Json::Str(mode));
            set_json_key(&mut s.claude, file::keys::CROSS_SESSION_INBOUND, value)
        }
        Message::Pref(pref) => {
            if let Pref::LaunchAtStartup(on) = &pref {
                themes::apply_launch_at_startup(&s.claude.home, *on);
            }
            apply_pref(s, &pref);
            Task::none()
        }
        Message::Theme(mode) => {
            s.select_theme(mode);
            Task::none()
        }
        Message::CodeFontInput(value) => {
            s.claude.code_font_draft = Some(value);
            Task::none()
        }
        Message::CodeFontCommit => {
            commit_code_font(s);
            Task::none()
        }
        Message::HostsInput(value) => {
            s.claude.hosts_draft = value;
            Task::none()
        }
        Message::HostsCommit => commit_hosts(&mut s.claude),
        Message::Instructions(action) => {
            let edited = action.is_edit();
            let home = s.claude.home.clone();
            let ins = s.claude.instructions.get_or_insert_with(|| {
                let saved = files::read_instructions(&home);
                Instructions {
                    content: text_editor::Content::with_text(&saved),
                    saved,
                }
            });
            ins.content.perform(action);
            if edited {
                commit_instructions(&mut s.claude);
            }
            Task::none()
        }
        Message::InstructionsCommit => {
            commit_instructions(&mut s.claude);
            Task::none()
        }
        Message::ProfileName(name) => {
            s.claude.profile_name = name;
            s.claude.profile_error = None;
            Task::none()
        }
        Message::ProfileDirectory(path) => {
            s.claude.profile_directory = path;
            s.claude.profile_error = None;
            Task::none()
        }
        Message::AddProfile => {
            if !cfg!(target_os = "linux") {
                return Task::none();
            }
            let name = s.claude.profile_name.trim().to_owned();
            let path = s.claude.profile_directory.trim();
            match profiles::register(&s.current, &name, path) {
                Ok(path) => {
                    let mut claude = s.current.claude.clone();
                    claude
                        .profiles
                        .insert(name, path.to_string_lossy().into_owned());
                    save_profiles(s, claude);
                    if s.saved {
                        s.claude.profile_name.clear();
                        s.claude.profile_directory.clear();
                    }
                }
                Err(error) => s.claude.profile_error = Some(error),
            }
            Task::none()
        }
        Message::SelectProfile(name) => {
            if let Some(name) = &name {
                if let Err(error) = profiles::directory(&s.current, name) {
                    s.claude.profile_error = Some(error);
                    return Task::none();
                }
            }
            let mut claude = s.current.claude.clone();
            claude.active_profile = name;
            save_profiles(s, claude);
            Task::none()
        }
        Message::LoginProfile(name) => {
            match profiles::directory(&s.current, &name)
                .and_then(|path| profiles::command(Some(&path), "auth login"))
            {
                Ok(command) => {
                    s.claude.profile_error = None;
                    s.claude.pending_profile_terminal = Some(command);
                }
                Err(error) => s.claude.profile_error = Some(error),
            }
            Task::none()
        }
        Message::RemoveProfile(name) => {
            let mut claude = s.current.claude.clone();
            if claude.profiles.remove(&name).is_some() {
                if claude.active_profile.as_ref() == Some(&name) {
                    claude.active_profile = None;
                }
                save_profiles(s, claude);
            }
            Task::none()
        }
        Message::RefreshAccount => {
            if s.claude.account_loading {
                Task::none()
            } else {
                load_account(&mut s.claude)
            }
        }
        Message::AccountLoaded(result) => {
            s.claude.account_loading = false;
            s.claude.account = Some(result);
            Task::none()
        }
        Message::LogoutAsk => {
            s.claude.logout_ask = true;
            Task::none()
        }
        Message::LogoutCancel => {
            s.claude.logout_ask = false;
            Task::none()
        }
        Message::LogoutConfirm => {
            s.claude.logout_ask = false;
            let home = s.claude.home.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || cli::logout(&home))
                        .await
                        .unwrap_or(Err(cli::InfoError::Failed(None)))
                },
                Message::LoggedOut,
            )
        }
        Message::LoggedOut(result) => {
            s.claude.logout_error = result.err();
            s.claude.account_loading = false;
            load_account(&mut s.claude)
        }
        Message::SwitchAccountAsk => {
            s.claude.switch_account_ask = true;
            Task::none()
        }
        Message::SwitchAccountCancel => {
            s.claude.switch_account_ask = false;
            Task::none()
        }
        Message::SwitchAccountConfirm => {
            s.claude.switch_account_ask = false;
            s.claude.account = None;
            s.claude.usage = None;
            s.claude.usage_at = None;
            s.claude.pending_terminal =
                Some(String::from("claude auth logout && claude auth login"));
            Task::none()
        }
        Message::RefreshUsage => {
            if s.claude.usage_loading {
                Task::none()
            } else {
                load_usage(&mut s.claude)
            }
        }
        Message::UsageLoaded(result) => {
            s.claude.usage_loading = false;
            s.claude.usage_at = Some(cli::now_epoch_secs());
            s.claude.usage = Some(result);
            Task::none()
        }
        Message::RefreshMcp => {
            if s.claude.mcp_loading {
                Task::none()
            } else {
                load_mcp(&mut s.claude)
            }
        }
        Message::McpLoaded(result) => {
            s.claude.mcp_loading = false;
            s.claude.mcp = Some(result);
            Task::none()
        }
        Message::SelectDevMcp(name) => {
            s.claude.dev_selected_mcp = Some(name);
            Task::none()
        }
        Message::RefreshSkills => {
            if s.claude.skills_loading {
                Task::none()
            } else {
                load_skills(&mut s.claude)
            }
        }
        Message::SkillsLoaded(result) => {
            s.claude.skills_loading = false;
            s.claude.skills = Some(result);
            Task::none()
        }
        Message::RefreshPlugins => {
            if s.claude.plugins_loading {
                Task::none()
            } else {
                load_plugins(&mut s.claude)
            }
        }
        Message::PluginsLoaded(result) => {
            s.claude.plugins_loading = false;
            s.claude.plugins = Some(result);
            Task::none()
        }
        Message::DirectoryLoaded(result) => {
            s.claude.directory_loading = false;
            s.claude.directory = Some(result);
            Task::none()
        }
        Message::CatalogDiscover(discover) => {
            s.claude.catalog_discover = discover;
            s.claude.catalog_add_menu = false;
            if discover
                && matches!(s.claude.tab, Tab::Skills | Tab::Plugins)
                && s.claude.directory.is_none()
                && !s.claude.directory_loading
            {
                load_directory(&mut s.claude)
            } else {
                Task::none()
            }
        }
        Message::CatalogSearch(query) => {
            s.claude.catalog_search = query;
            Task::none()
        }
        Message::CatalogAddMenu(open) => {
            s.claude.catalog_add_menu = open;
            Task::none()
        }
        Message::SelectMarketplace(market) => {
            s.claude.selected_marketplace = market;
            Task::none()
        }
        Message::NamePromptOpen(kind) => {
            s.claude.catalog_add_menu = false;
            s.claude.name_prompt = Some((kind, String::new()));
            Task::none()
        }
        Message::NamePromptInput(text) => {
            if let Some((_, val)) = &mut s.claude.name_prompt {
                *val = text;
            }
            Task::none()
        }
        Message::NamePromptCancel => {
            s.claude.name_prompt = None;
            Task::none()
        }
        Message::NamePromptSubmit => {
            let Some((kind, raw)) = s.claude.name_prompt.take() else {
                return Task::none();
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Task::none();
            }
            let quoted = widgets::shell_quote(trimmed);
            let cmd = match kind {
                NamePromptKind::Skill => format!("claude plugin new {quoted}"),
                NamePromptKind::Marketplace => format!("claude plugin marketplace add {quoted}"),
            };
            s.claude.pending_terminal = Some(cmd);
            Task::none()
        }
        Message::McpFormName(val) => {
            s.claude.mcp_form_name = val;
            Task::none()
        }
        Message::McpFormTarget(val) => {
            s.claude.mcp_form_target = val;
            Task::none()
        }
        Message::McpFormHttp(http) => {
            s.claude.mcp_form_http = http;
            Task::none()
        }
        Message::McpFormLocalScope(local) => {
            s.claude.mcp_form_local_scope = local;
            Task::none()
        }
        Message::DsSearch(q) => {
            s.claude.ds_search = q;
            Task::none()
        }
        Message::ImportThemePick => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title(labels::SETTINGS_DS_IMPORT_DIALOG)
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::ThemeFilePicked,
        ),
        Message::ThemeFilePicked(Some(path)) => match themes::import_theme(&s.claude.home, &path) {
            Ok(entry) => {
                s.claude.themes = themes::load_themes(&s.claude.home);
                apply_pref(s, &Pref::ThemeId(entry.id));
                show_raw_toast(&mut s.claude, labels::ds_theme_imported(&entry.name))
            }
            Err(err) => show_raw_toast(&mut s.claude, err),
        },
        Message::ThemeFilePicked(None) => Task::none(),
        Message::DeleteTheme(id) => match themes::delete_theme(&s.claude.home, &id) {
            Ok(()) => {
                s.claude.themes = themes::load_themes(&s.claude.home);
                if s.current.theme_id == id {
                    apply_pref(s, &Pref::ThemeId(String::from("gsh")));
                }
                Task::none()
            }
            Err(err) => show_raw_toast(&mut s.claude, err),
        },
        Message::KeepAwakeToggle(enable) => {
            s.claude.keep_awake.set(enable);
            if !enable {
                s.claude.stop_bag_mode();
            }
            s.claude.poll_keep_awake();
            Task::none()
        }
        Message::LidSessionDuration(duration) => {
            s.claude.keep_awake.set_lid_duration(duration);
            Task::none()
        }
        Message::LidSessionStart => {
            let duration = s.claude.keep_awake.lid_duration();
            if let Err(reason) = s.claude.keep_awake.start_lid_session(duration) {
                let msg = match reason {
                    themes::LidSafetyStop::LowBattery(pct) => {
                        labels::settings_lid_stopped_battery(pct)
                    }
                    themes::LidSafetyStop::HighTemp(temp_c) => {
                        labels::settings_lid_stopped_temp(temp_c)
                    }
                };
                s.claude.keep_awake_active = s.claude.keep_awake.is_active();
                return show_raw_toast(&mut s.claude, msg);
            }
            s.claude.poll_keep_awake();
            Task::none()
        }
        Message::LidSessionStop => {
            s.claude.keep_awake.stop_lid_session();
            s.claude.stop_bag_mode();
            s.claude.poll_keep_awake();
            Task::none()
        }
        Message::BagModeToggle(true) => apply_bag_hotspot(&mut s.claude, true),
        Message::BagModeToggle(false) => {
            s.claude.stop_bag_mode();
            s.claude.keep_awake.stop_lid_session();
            s.claude.poll_keep_awake();
            Task::none()
        }
        Message::BagHotspotInput(ssid) => {
            if s.claude.bag_mode.is_editable() {
                s.claude.bag_mode.config.hotspot_ssid = ssid;
            }
            Task::none()
        }
        Message::BagPreferredInput(raw) => {
            if s.claude.bag_mode.is_editable() {
                s.claude.bag_mode.config.preferred_wifi_ssids = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect();
                s.claude.bag_mode.preferred_draft = raw;
            }
            Task::none()
        }
        Message::BagApplyHotspot => apply_bag_hotspot(&mut s.claude, false),
        Message::BagHotspotApplied(start, result) => {
            let c = &mut s.claude;
            if !c.finish_bag_work() {
                return Task::none();
            }
            if let Err(error) = result {
                c.bag_mode.status = Some(Err(error));
                return Task::none();
            }
            if !start {
                c.bag_mode.status = Some(Ok("Perfil do hotspot configurado.".to_owned()));
                return Task::none();
            }

            if !c.keep_awake.is_lid_active() {
                let duration = c.keep_awake.lid_duration();
                if c.keep_awake.start_lid_session(duration).is_err() {
                    c.bag_mode.status = None;
                    return Task::none();
                }
            }
            c.poll_keep_awake();
            if c.keep_awake.lid_remaining_minutes().is_none() {
                c.bag_mode.status = Some(Err(
                    "Não foi possível iniciar a sessão de tampa fechada.".to_owned(),
                ));
                return Task::none();
            }
            c.bag_mode.config.enabled = true;
            c.bag_mode.status = Some(Ok("Monitorando a conexão…".to_owned()));
            c.poll_bag_mode().unwrap_or_else(Task::none)
        }
        Message::BagPollFinished(result) => {
            let c = &mut s.claude;
            let completed = c.finish_bag_work();
            c.poll_keep_awake();
            if completed && c.bag_mode.config.enabled {
                let now = Instant::now();
                c.bag_mode.poll_at = Some(now + crate::bag_mode::POLL_INTERVAL);
                c.bag_mode.status = Some(result.map(|poll| {
                    if poll.switched_to_hotspot {
                        c.bag_mode.preferred_retry_at =
                            Some(now + crate::bag_mode::HOTSPOT_RETRY_INTERVAL);
                    }
                    poll.status
                }));
            }
            Task::none()
        }
        Message::StorageUseDefault => {
            let _ = themes::set_override_root(&s.claude.home, None);
            s.claude.storage_restart_msg = Some((
                labels::SETTINGS_STORAGE_RESTART_DEFAULT_MSG.to_owned(),
                false,
            ));
            Task::none()
        }
        Message::StorageChangePick => {
            let initial = themes::effective_root(&s.claude.home);
            Task::perform(
                async move {
                    let mut dlg = rfd::AsyncFileDialog::new()
                        .set_title(labels::SETTINGS_STORAGE_CHOOSE_DIALOG);
                    if !initial.as_os_str().is_empty() && initial.is_dir() {
                        dlg = dlg.set_directory(initial);
                    }
                    dlg.pick_folder().await.map(|h| h.path().to_path_buf())
                },
                Message::StorageFolderPicked,
            )
        }
        Message::StorageFolderPicked(Some(folder)) => {
            let _ = themes::migrate_state_to(&s.claude.home, &folder);
            let _ = themes::set_override_root(&s.claude.home, Some(&folder));
            s.claude.storage_restart_msg = Some((
                labels::storage_restart_change_msg(&folder.to_string_lossy()),
                false,
            ));
            Task::none()
        }
        Message::StorageFolderPicked(None) => Task::none(),
        Message::StorageResetAsk => {
            s.claude.storage_reset_ask = true;
            Task::none()
        }
        Message::StorageResetCancel => {
            s.claude.storage_reset_ask = false;
            Task::none()
        }
        Message::StorageResetConfirm => {
            s.claude.storage_reset_ask = false;
            let _ = themes::reset_all(&s.claude.home);
            s.claude.storage_restart_msg =
                Some((labels::SETTINGS_STORAGE_RESTART_RESET_MSG.to_owned(), true));
            Task::none()
        }
        Message::StorageRestartDismiss => {
            s.claude.storage_restart_msg = None;
            Task::none()
        }
        Message::StorageRestartQuit => {
            s.claude.storage_restart_msg = None;
            s.claude.pending_quit = true;
            Task::none()
        }
        Message::FontPickerOpen(target) => {
            let current = match target {
                FontTarget::Interface => s.current.interface_font.clone(),
                FontTarget::Chat => s.current.chat.font.clone(),
            };
            s.claude.font_picker = Some(FontPickerState::open(target, current));
            Task::none()
        }
        Message::FontPickerSearch(q) => {
            if let Some(picker) = &mut s.claude.font_picker {
                picker.search = q;
            }
            Task::none()
        }
        Message::FontPickerCustomInput(v) => {
            if let Some(picker) = &mut s.claude.font_picker {
                picker.custom = v;
            }
            Task::none()
        }
        Message::FontPickerUseCustom => {
            let Some(picker) = s.claude.font_picker.take() else {
                return Task::none();
            };
            let trimmed = picker.custom.trim();
            if !trimmed.is_empty() {
                let pref = match picker.target {
                    FontTarget::Interface => Pref::InterfaceFont(Some(trimmed.to_owned())),
                    FontTarget::Chat => Pref::ChatFont(Some(trimmed.to_owned())),
                };
                apply_pref(s, &pref);
            }
            Task::none()
        }
        Message::FontPick(choice) => {
            let Some(picker) = s.claude.font_picker.take() else {
                return Task::none();
            };
            let pref = match picker.target {
                FontTarget::Interface => Pref::InterfaceFont(choice),
                FontTarget::Chat => Pref::ChatFont(choice),
            };
            apply_pref(s, &pref);
            Task::none()
        }
        Message::FontPickerClose => {
            s.claude.font_picker = None;
            Task::none()
        }
        Message::RunInTerminal(command) => {
            let task = commit_pending(s);
            s.claude.pending_terminal = Some(command);
            task
        }
        Message::OpenFile(path) => {
            let task = commit_pending(s);
            s.claude.pending_open_file = Some(path);
            task
        }
        Message::OpenUrl(url) => {
            open_url(&url);
            Task::none()
        }
        Message::NoticeClose => {
            s.claude.notice = false;
            Task::none()
        }
        Message::ToastClose(id) => {
            if s.claude.toast.as_ref().is_some_and(|(cur, _)| *cur == id) {
                s.claude.toast = None;
            }
            Task::none()
        }
    }
}

fn apply_bag_hotspot(c: &mut ClaudeState, start: bool) -> Task<Message> {
    if !cfg!(target_os = "linux") || !c.bag_mode.is_editable() {
        return Task::none();
    }
    let config = c.bag_mode.config.clone();
    if let Err(error) = config.validate() {
        c.bag_mode.status = Some(Err(error));
        return Task::none();
    }
    c.bag_mode.status = Some(Ok("Configurando o hotspot…".to_owned()));
    let cancelled = c.start_bag_work();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                crate::bag_mode::configure_hotspot_blocking(&config, &cancelled)
            })
            .await
            .map_err(|err| format!("Falha ao configurar o hotspot: {err}"))?
        },
        move |result| Message::BagHotspotApplied(start, result),
    )
}

pub(in crate::settings) fn commit_pending(s: &mut Settings) -> Task<Message> {
    commit_code_font(s);
    commit_instructions(&mut s.claude);
    commit_hosts(&mut s.claude)
}

fn enter_tab(c: &mut ClaudeState, tab: Tab) -> Task<Message> {
    match tab {
        Tab::General
        | Tab::Privacy
        | Tab::Resources
        | Tab::Cowork
        | Tab::Extensions
        | Tab::System => {
            c.refresh_snapshot();
            Task::none()
        }
        Tab::ClaudeCode | Tab::DesignSystems | Tab::Memory => {
            c.reload_files();
            Task::none()
        }
        Tab::Account => enter_account(c),
        Tab::Usage => {
            let mut tasks = Vec::new();
            if c.account.is_none() && !c.account_loading {
                tasks.push(load_account(c));
            }
            if c.usage.is_none() && !c.usage_loading {
                tasks.push(load_usage(c));
            }
            Task::batch(tasks)
        }
        Tab::Developer | Tab::Connectors => {
            if c.mcp.is_none() && !c.mcp_loading {
                load_mcp(c)
            } else {
                Task::none()
            }
        }
        Tab::Skills => {
            if c.skills.is_none() && !c.skills_loading {
                load_skills(c)
            } else {
                Task::none()
            }
        }
        Tab::Plugins => {
            c.project_plugins = files::project_plugins(c.workspace.as_deref());
            if c.plugins.is_none() && !c.plugins_loading {
                load_plugins(c)
            } else {
                Task::none()
            }
        }
    }
}

fn enter_account(c: &mut ClaudeState) -> Task<Message> {
    if c.instructions.is_none() {
        let saved = files::read_instructions(&c.home);
        c.instructions = Some(Instructions {
            content: text_editor::Content::with_text(&saved),
            saved,
        });
    }
    if c.account.is_none() && !c.account_loading {
        load_account(c)
    } else {
        Task::none()
    }
}

fn load_account(c: &mut ClaudeState) -> Task<Message> {
    c.account_loading = true;
    let home = c.home.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || cli::account(&home))
                .await
                .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::AccountLoaded,
    )
}

fn load_usage(c: &mut ClaudeState) -> Task<Message> {
    c.usage_loading = true;
    let home = c.home.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || cli::usage(&home))
                .await
                .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::UsageLoaded,
    )
}

fn load_mcp(c: &mut ClaudeState) -> Task<Message> {
    c.mcp_loading = true;
    let home = c.home.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || cli::mcp_servers(&home))
                .await
                .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::McpLoaded,
    )
}

fn load_plugins(c: &mut ClaudeState) -> Task<Message> {
    c.plugins_loading = true;
    let home = c.home.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || cli::plugins(&home))
                .await
                .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::PluginsLoaded,
    )
}

fn load_directory(c: &mut ClaudeState) -> Task<Message> {
    c.directory_loading = true;
    let home = c.home.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || cli::directory(&home))
                .await
                .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::DirectoryLoaded,
    )
}

fn load_skills(c: &mut ClaudeState) -> Task<Message> {
    c.skills_loading = true;
    let home = c.home.clone();
    let workspace = c.workspace.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let pls = cli::plugins(&home).unwrap_or_default();
                Ok(files::skills(&home, &pls, workspace.as_deref()))
            })
            .await
            .unwrap_or(Err(cli::InfoError::Failed(None)))
        },
        Message::SkillsLoaded,
    )
}

fn set_json_key(c: &mut ClaudeState, key: &str, value: Option<Json>) -> Task<Message> {
    let path = c.settings_path();
    let result = file::set(&path, key, value);
    c.refresh_snapshot();
    match result {
        Ok(()) => Task::none(),
        Err(detail) => show_toast(c, &detail),
    }
}

fn apply_pref(s: &mut Settings, pref: &Pref) {
    let result = match &s.path {
        Ok(path) => prefs::save(path, pref),
        Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
    };
    if let Ok(config) = &result {
        s.interface_size = config.interface_size.to_string();
        s.scale = (config.interface_size.clamp(10.0, 20.0) / 14.0) as f32;
        s.notifications = config.notifications_enabled;
        s.permission_sound = config
            .sound
            .events
            .get(&SoundEvent::ActionRequired)
            .copied()
            .unwrap_or(true);
        s.transcript_width = config.chat.transcript_width;
        s.current = config.clone();
        let variant = crate::theme::ThemeVariant::from_mode(config.theme_mode);
        let spec = themes::selected_theme_json(&s.claude.home, &config.theme_id);
        if !crate::theme::set_theme_palette(&config.theme_id, spec.as_deref(), variant) {
            crate::theme::set_active(variant);
        }
    }
    s.saved = result.is_ok();
    s.error = result.err();
}

fn save_profiles(s: &mut Settings, claude: gsh_config::schema::ClaudeProfiles) {
    s.claude.profile_error = None;
    apply_pref(s, &Pref::ClaudeProfiles(claude));
}

fn commit_code_font(s: &mut Settings) {
    let Some(draft) = s.claude.code_font_draft.take() else {
        return;
    };
    let trimmed = draft.trim();
    let next = (!trimmed.is_empty()).then(|| trimmed.to_owned());
    if next != s.current.code_font {
        apply_pref(s, &Pref::CodeFont(next));
    }
}

fn commit_hosts(c: &mut ClaudeState) -> Task<Message> {
    if c.hosts_draft.trim() == c.hosts {
        return Task::none();
    }
    let list: Vec<String> = c
        .hosts_draft
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    match files::write_hosts(&c.home, &list) {
        Ok(()) => {
            let formatted = files::read_hosts(&c.home).join(", ");
            c.hosts = formatted.clone();
            c.hosts_draft = formatted;
            Task::none()
        }
        Err(error) => show_toast(c, &error.to_string()),
    }
}

fn commit_instructions(c: &mut ClaudeState) {
    let Some(ins) = &mut c.instructions else {
        return;
    };
    let text = ins.content.text();
    if text == ins.saved {
        return;
    }
    match files::write_instructions(&c.home, &text) {
        Ok(()) => {
            ins.saved = text;
            c.instructions_error = None;
        }
        Err(error) => {
            c.instructions_error = Some(error);
        }
    }
}

fn show_toast(c: &mut ClaudeState, detail: &str) -> Task<Message> {
    show_raw_toast(c, labels::settings_save_failed(detail))
}

fn show_raw_toast(c: &mut ClaudeState, message: String) -> Task<Message> {
    c.next_toast = c.next_toast.wrapping_add(1);
    let id = c.next_toast;
    c.toast = Some((id, message));
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(|| {
                std::thread::sleep(Duration::from_secs(4));
            })
            .await;
            id
        },
        Message::ToastClose,
    )
}

fn open_url(url: &str) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::claude::cli::{Account, InfoError};
    use crate::settings::claude::Switch;
    use crate::settings::Message as RootMsg;
    use gsh_config::schema::{TranscriptView, TranscriptWidth};
    use iced::widget::text_editor::{Action, Edit, Motion};
    use std::path::PathBuf;

    fn temp_fixture(name: &str) -> (PathBuf, Settings) {
        let dir =
            std::env::temp_dir().join(format!("gsh-claude-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let config_path = dir.join("config.toml");
        let mut settings = Settings {
            path: Ok(config_path),
            ..Settings::default()
        };
        settings.claude.home = dir.clone();
        settings.claude.reset();
        (dir, settings)
    }

    #[test]
    fn bag_mode_preserves_single_flight_after_settings_reload_and_cancels_on_lid_stop() {
        let (dir, mut settings) = temp_fixture("bag-mode");
        settings.claude.bag_mode.config.hotspot_ssid = "Hotspot".into();
        assert!(settings.claude.poll_bag_mode().is_none());
        let pending = settings.claude.start_bag_work();
        settings.reload();
        assert_eq!(settings.claude.bag_mode.config.hotspot_ssid, "Hotspot");
        assert!(!settings.claude.bag_mode.is_editable());
        assert!(!pending.load(std::sync::atomic::Ordering::Relaxed));
        let _ = settings.update(RootMsg::Claude(Message::LidSessionStop));
        assert!(pending.load(std::sync::atomic::Ordering::Relaxed));
        let _ = settings.update(RootMsg::Claude(Message::BagHotspotApplied(true, Ok(()))));
        assert!(settings.claude.bag_mode.is_editable());
        assert!(settings.claude.poll_bag_mode().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn switches_output_style_and_cross_session_persist_and_bad_json_shows_toast() {
        let (dir, mut s) = temp_fixture("switches");
        let json_path = dir.join(".claude/settings.json");
        std::fs::create_dir_all(json_path.parent().unwrap()).unwrap();
        std::fs::write(&json_path, r#"{"customKey":"preserve","first":1}"#).unwrap();
        s.claude.reset();

        let _ = s.update(RootMsg::Claude(Message::Switch(Switch::Sandbox, true)));
        assert!(Switch::Sandbox.is_on(&s.claude.snapshot));
        let _ = s.update(RootMsg::Claude(Message::Switch(
            Switch::SandboxStrict,
            true,
        )));
        assert!(Switch::SandboxStrict.is_on(&s.claude.snapshot));
        let _ = s.update(RootMsg::Claude(Message::OutputStyle(Some(
            "Learning".into(),
        ))));
        let _ = s.update(RootMsg::Claude(Message::CrossSession("hold".into())));
        let raw = std::fs::read_to_string(&json_path).unwrap();
        assert!(raw.contains("\"customKey\": \"preserve\""));
        assert_eq!(
            file::value(&s.claude.snapshot, "outputStyle"),
            Some(&Json::Str("Learning".into()))
        );
        assert_eq!(
            file::value(&s.claude.snapshot, "crossSessionInbound"),
            Some(&Json::Str("hold".into()))
        );

        let _ = s.update(RootMsg::Claude(Message::OutputStyle(None)));
        assert!(file::value(&s.claude.snapshot, "outputStyle").is_none());

        std::fs::write(&json_path, "[1, 2]").unwrap();
        let _ = s.update(RootMsg::Claude(Message::Switch(
            Switch::DisableTelemetry,
            true,
        )));
        assert!(s
            .claude
            .toast_text()
            .unwrap()
            .starts_with("Não consegui salvar:"));
        let id = s.claude.toast.as_ref().unwrap().0;
        let _ = s.update(RootMsg::Claude(Message::ToastClose(id)));
        assert!(s.claude.toast_text().is_none());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn prefs_fonts_themes_storage_and_instructions_round_trip_in_temp_dir() {
        let _guard = crate::theme::test_guard();
        let (dir, mut s) = temp_fixture("prefs");
        let _ = s.update(RootMsg::Claude(Message::Pref(Pref::TranscriptView(
            TranscriptView::Thinking,
        ))));
        assert_eq!(s.current.chat.transcript_view, TranscriptView::Thinking);
        let _ = s.update(RootMsg::Claude(Message::Pref(Pref::TranscriptWidth(
            TranscriptWidth::Narrow,
        ))));
        assert_eq!(s.transcript_width, TranscriptWidth::Narrow);
        let _ = s.update(RootMsg::Claude(Message::Pref(Pref::InterfaceSize(16.0))));
        assert_eq!(s.interface_size, "16");

        let _ = s.update(RootMsg::Claude(Message::FontPickerOpen(
            FontTarget::Interface,
        )));
        let _ = s.update(RootMsg::Claude(Message::FontPickerCustomInput(
            "Inter".into(),
        )));
        let _ = s.update(RootMsg::Claude(Message::FontPickerUseCustom));
        assert_eq!(s.current.interface_font.as_deref(), Some("Inter"));

        let _ = s.update(RootMsg::Claude(Message::FontPickerOpen(FontTarget::Chat)));
        let _ = s.update(RootMsg::Claude(Message::FontPick(Some("Roboto".into()))));
        assert_eq!(s.current.chat.font.as_deref(), Some("Roboto"));

        let _ = s.update(RootMsg::Claude(Message::CodeFontInput(
            "  Fira Code  ".into(),
        )));
        let _ = s.update(RootMsg::Claude(Message::CodeFontCommit));
        assert_eq!(s.current.code_font.as_deref(), Some("Fira Code"));

        let _ = s.update(RootMsg::Claude(Message::HostsInput(
            "Prod, homolog.gorila.com, prod".into(),
        )));
        let _ = s.update(RootMsg::Claude(Message::HostsCommit));
        assert_eq!(s.claude.hosts, "prod, homolog.gorila.com");
        assert_eq!(
            files::read_hosts(&dir),
            vec!["prod".to_owned(), "homolog.gorila.com".to_owned()]
        );

        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        std::fs::write(dir.join(".claude/CLAUDE.md"), "inicial").unwrap();
        let _ = s.update(RootMsg::Claude(Message::Select(Tab::Account)));
        let _ = s.update(RootMsg::Claude(Message::Instructions(Action::Move(
            Motion::DocumentEnd,
        ))));
        let _ = s.update(RootMsg::Claude(Message::Instructions(Action::Edit(
            Edit::Insert('!'),
        ))));
        let _ = s.update(RootMsg::Claude(Message::InstructionsCommit));
        assert_eq!(files::read_instructions(&dir), "inicial!");

        let custom_theme = dir.join("neon.json");
        std::fs::write(
            &custom_theme,
            r#"{"id":"acme.neon","name":"Neon","variants":{"dark":{}}}"#,
        )
        .unwrap();
        let _ = s.update(RootMsg::Claude(Message::ThemeFilePicked(Some(
            custom_theme,
        ))));
        assert_eq!(s.current.theme_id, "acme.neon");
        let _ = s.update(RootMsg::Claude(Message::DeleteTheme("acme.neon".into())));
        assert_eq!(s.current.theme_id, "gsh");

        let _ = s.update(RootMsg::Claude(Message::NamePromptOpen(
            NamePromptKind::Skill,
        )));
        let _ = s.update(RootMsg::Claude(Message::NamePromptInput(
            "minha-skill".into(),
        )));
        let _ = s.update(RootMsg::Claude(Message::NamePromptSubmit));
        assert_eq!(
            s.claude.take_terminal_command().as_deref(),
            Some("claude plugin new 'minha-skill'")
        );

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn account_logout_and_terminal_flow_update_state() {
        let (dir, mut s) = temp_fixture("account");
        let acc = Account {
            logged_in: true,
            email: Some("dev@gorila.com".into()),
            org_name: Some("Gorila".into()),
            org_id: Some("org-1".into()),
            subscription_type: Some("team".into()),
            auth_method: Some("claude.ai".into()),
            api_provider: None,
        };
        let _ = s.update(RootMsg::Claude(Message::AccountLoaded(Ok(acc.clone()))));
        assert_eq!(s.claude.account, Some(Ok(acc)));
        let _ = s.update(RootMsg::Claude(Message::LogoutAsk));
        assert!(s.claude.logout_ask);
        let _ = s.update(RootMsg::Claude(Message::LogoutCancel));
        assert!(!s.claude.logout_ask);
        let _ = s.update(RootMsg::Claude(Message::LoggedOut(Err(
            InfoError::CliMissing,
        ))));
        assert_eq!(s.claude.logout_error, Some(InfoError::CliMissing));

        let _ = s.update(RootMsg::Claude(Message::SwitchAccountAsk));
        assert!(s.claude.switch_account_ask);
        let _ = s.update(RootMsg::Claude(Message::SwitchAccountCancel));
        assert!(!s.claude.switch_account_ask);
        let _ = s.update(RootMsg::Claude(Message::SwitchAccountAsk));
        let _ = s.update(RootMsg::Claude(Message::SwitchAccountConfirm));
        assert!(!s.claude.switch_account_ask);
        assert!(s.claude.account.is_none());
        assert_eq!(
            s.claude.take_terminal_command().as_deref(),
            Some("claude auth logout && claude auth login")
        );

        let _ = s.update(RootMsg::Claude(Message::RunInTerminal(
            "claude auth login".into(),
        )));
        assert_eq!(
            s.claude.take_terminal_command().as_deref(),
            Some("claude auth login")
        );
        s.claude.show_no_workspace();
        assert!(s.claude.notice);
        let _ = s.update(RootMsg::Claude(Message::NoticeClose));
        assert!(!s.claude.notice);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn profiles_persist_selection_and_login_targets_the_requested_directory() {
        let (dir, mut s) = temp_fixture("profiles");
        let profile = dir.join("Conta Trabalho");
        let _ = s.update(RootMsg::Claude(Message::ProfileName("Trabalho".into())));
        let _ = s.update(RootMsg::Claude(Message::ProfileDirectory(
            profile.to_string_lossy().into_owned(),
        )));
        let _ = s.update(RootMsg::Claude(Message::AddProfile));
        assert!(s.saved);
        assert_eq!(s.current.claude.active_profile, None);
        let _ = s.update(RootMsg::Claude(Message::SelectProfile(Some(
            "Trabalho".into(),
        ))));
        let config = gsh_config::load(s.path.as_ref().unwrap()).unwrap();
        assert_eq!(profiles::selected(&config), Ok(Some(profile.clone())));

        let _ = s.update(RootMsg::Claude(Message::LoginProfile("Trabalho".into())));
        assert_eq!(
            s.claude.take_profile_terminal_command(),
            Some(profiles::command(Some(&profile), "auth login").unwrap())
        );
        let _ = s.update(RootMsg::Claude(Message::SwitchAccountConfirm));
        assert_eq!(
            s.claude.take_terminal_command().as_deref(),
            Some("claude auth logout && claude auth login")
        );
        let _ = s.update(RootMsg::Claude(Message::RemoveProfile("Trabalho".into())));
        assert!(s.current.claude.profiles.is_empty());
        assert_eq!(s.current.claude.active_profile, None);
        assert!(profile.is_dir());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
