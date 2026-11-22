use std::time::Duration;

use notify_rust::{Notification, NotificationResponse, Timeout};
use tokio::sync::watch;

use crate::labels;

pub const PERMISSION_WINDOW: Duration = Duration::from_secs(30);
pub const TURN_WINDOW: Duration = Duration::from_secs(8);
const LINUX_DESKTOP_ENTRY: &str = "work.gorila.gshdesktop";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Allow,
    Always,
    Deny,
    Terminal,
    Unavailable,
}

pub fn notification(title: &str, body: &str, can_always: Option<bool>) -> Notification {
    let mut notification = Notification::new();
    notification
        .summary(title)
        .body(body)
        .timeout(Timeout::Milliseconds(if can_always.is_some() {
            PERMISSION_WINDOW.as_millis() as u32
        } else {
            TURN_WINDOW.as_millis() as u32
        }));
    #[cfg(target_os = "linux")]
    notification
        .appname("GSH Desktop")
        .hint(notify_rust::Hint::DesktopEntry(
            LINUX_DESKTOP_ENTRY.to_owned(),
        ))
        .action("default", labels::ACTION_IN_TERMINAL);
    #[cfg(target_os = "windows")]
    notification.app_id("work.gorila.gshdesktop");
    #[cfg(target_os = "macos")]
    {
        use notify_rust::NsNotificationExt;
        notification.actions_label("Ação");
        notification.close_button("Fechar");
    }

    // macOS legacy reports a click on the dropdown title as its first action.
    notification.action("terminal", labels::ACTION_IN_TERMINAL);
    if let Some(can_always) = can_always {
        notification.action("allow", labels::ACTION_ALLOW);
        if can_always {
            notification.action("always", labels::ACTION_ALWAYS);
        }
        notification.action("deny", labels::ACTION_DENY);
    }
    notification
}

fn action_for_response(response: &NotificationResponse) -> Action {
    match response {
        NotificationResponse::Default => Action::Terminal,
        NotificationResponse::Action(action) => match action.as_str() {
            "allow" => Action::Allow,
            "always" => Action::Always,
            "deny" => Action::Deny,
            "default" | "terminal" => Action::Terminal,
            _ => Action::Unavailable,
        },
        NotificationResponse::Closed(_) | NotificationResponse::Reply(_) => Action::Unavailable,
    }
}

#[cfg(target_os = "linux")]
pub async fn show(mut updates: watch::Receiver<Notification>, window: Duration) -> Action {
    use iced::futures::StreamExt;
    use zbus::{fdo::DBusProxy, names::BusName, Connection, Proxy};

    const SERVICE_TIMEOUT: Duration = Duration::from_secs(2);
    const SERVICE: &str = "org.freedesktop.Notifications";

    let notification = updates.borrow_and_update().clone();
    let deadline = tokio::time::sleep(window);
    tokio::pin!(deadline);
    let setup = async {
        let connection = Connection::session().await?;
        let bus = DBusProxy::new(&connection).await?;
        let name = BusName::try_from(SERVICE)?;
        let owner = match bus.get_name_owner(name.clone()).await {
            Ok(owner) => owner,
            Err(zbus::fdo::Error::NameHasNoOwner(_)) => {
                bus.start_service_by_name(SERVICE.try_into()?, 0).await?;
                bus.get_name_owner(name).await?
            }
            Err(error) => return Err(error.into()),
        };
        // Keep Notify and its signals on the same unique D-Bus sender across replacements.
        let proxy = Proxy::new_owned(
            connection,
            owner.clone(),
            "/org/freedesktop/Notifications",
            SERVICE,
        )
        .await?;
        let signals = proxy.receive_all_signals().await?;
        let id = notify_linux(&proxy, &notification, 0).await?;
        Ok::<_, zbus::Error>((proxy, owner, signals, id))
    };
    let Ok(Ok((proxy, owner, mut signals, mut id))) =
        tokio::time::timeout(SERVICE_TIMEOUT, setup).await
    else {
        return Action::Unavailable;
    };

    loop {
        tokio::select! {
            biased;
            _ = &mut deadline => break,
            signal = signals.next() => {
                let Some(signal) = signal else { break };
                if let Some(action) = linux_action(&signal, &owner, id) {
                    return action;
                }
            }
            update = updates.changed() => {
                if update.is_err() {
                    break;
                }
                let notification = updates.borrow_and_update().clone();
                match tokio::time::timeout(SERVICE_TIMEOUT, notify_linux(&proxy, &notification, id)).await {
                    Ok(Ok(replacement)) => {
                        id = replacement;
                        deadline.as_mut().reset(tokio::time::Instant::now() + window);
                    }
                    _ => break,
                }
            }
        }
    }

    let _ = tokio::time::timeout(
        SERVICE_TIMEOUT,
        proxy.call::<_, _, ()>("CloseNotification", &(id,)),
    )
    .await;
    Action::Unavailable
}

#[cfg(target_os = "linux")]
async fn notify_linux(
    proxy: &zbus::Proxy<'_>,
    notification: &Notification,
    id: u32,
) -> zbus::Result<u32> {
    let hints = std::collections::HashMap::from([(
        "desktop-entry",
        zbus::Variant::from(LINUX_DESKTOP_ENTRY),
    )]);
    let body = notification
        .body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    proxy
        .call(
            "Notify",
            &(
                notification.appname.as_str(),
                id,
                "",
                notification.summary.as_str(),
                body,
                &notification.actions,
                hints,
                i32::from(notification.timeout),
            ),
        )
        .await
}

#[cfg(target_os = "linux")]
fn linux_action(
    message: &zbus::Message,
    owner: &zbus::names::UniqueName<'_>,
    id: u32,
) -> Option<Action> {
    let header = message.header();
    if header.sender() != Some(owner) {
        return None;
    }
    match header.member()?.as_str() {
        "ActionInvoked" => {
            let (notification, action) = message.body().deserialize::<(u32, String)>().ok()?;
            (notification == id).then(|| action_for_response(&NotificationResponse::Action(action)))
        }
        "NotificationClosed" => {
            let (notification, _) = message.body().deserialize::<(u32, u32)>().ok()?;
            (notification == id).then_some(Action::Unavailable)
        }
        _ => None,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn show(mut updates: watch::Receiver<Notification>, window: Duration) -> Action {
    let notification = updates.borrow_and_update().clone();
    let (reply, response) = tokio::sync::oneshot::channel();
    // shortcut: these notify-rust backends cannot cancel a native wait, use cancellable platform APIs if needed.
    let started = std::thread::Builder::new().spawn(move || {
        #[cfg(target_os = "macos")]
        let _ = notify_rust::set_application("work.gorila.gshdesktop");
        let Ok(handle) = notification.show() else {
            return;
        };
        let mut action = Action::Unavailable;
        let _ = handle.wait_for_response(|response: &NotificationResponse| {
            action = action_for_response(response);
        });
        let _ = reply.send(action);
    });

    if started.is_err() {
        return Action::Unavailable;
    }
    tokio::select! {
        action = response => action.unwrap_or(Action::Unavailable),
        _ = tokio::time::sleep(window) => Action::Unavailable,
        _ = updates.changed() => Action::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_is_the_safe_first_visible_action() {
        for (can_always, expected) in [
            (None, vec!["terminal"]),
            (Some(false), vec!["terminal", "allow", "deny"]),
            (Some(true), vec!["terminal", "allow", "always", "deny"]),
        ] {
            let notification = notification("Título", "Corpo", can_always);
            let (actions, remainder) = notification.actions.as_chunks::<2>();
            assert!(remainder.is_empty());
            let ids = actions
                .iter()
                .filter_map(|action| (action[0] != "default").then_some(action[0].as_str()))
                .collect::<Vec<_>>();
            assert_eq!(ids, expected);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_notification_uses_the_installed_desktop_entry_identity() {
        let notification = notification("Title", "Body", None);
        assert_eq!(notification.appname, "GSH Desktop");
        assert!(notification
            .hints
            .contains(&notify_rust::Hint::DesktopEntry(
                LINUX_DESKTOP_ENTRY.to_owned()
            )));
    }

    #[test]
    fn only_named_button_responses_map_to_permission_decisions() {
        use notify_rust::CloseReason;

        assert_eq!(
            action_for_response(&NotificationResponse::Default),
            Action::Terminal
        );
        assert_eq!(
            action_for_response(&NotificationResponse::Closed(CloseReason::Dismissed)),
            Action::Unavailable
        );
        assert_eq!(
            action_for_response(&NotificationResponse::Action("unrecognised".into())),
            Action::Unavailable
        );
        assert_eq!(
            action_for_response(&NotificationResponse::Action("allow".into())),
            Action::Allow
        );
    }

    use crate::{
        claude_permission::permission_key,
        island::{IslandMsg, PermissionDecision},
        tests::test_shell,
        Msg, PaneTab, Shell,
    };
    use serde_json::{json, Value};
    use std::{sync::mpsc, time::Instant};

    fn ask(shell: &mut Shell, args: Value) -> (String, mpsc::Receiver<Value>) {
        let (reply, response) = mpsc::sync_channel(1);
        let request = json!({"cmd": "permission-ask", "tabId": "t1", "args": args});
        let _ = shell.permission_ask(gsh_ipc::PendingCommand { request, reply });
        let id = shell
            .status_runtime
            .pane_record("t1")
            .unwrap()
            .pending_ask()
            .unwrap()
            .id
            .clone();
        (id, response)
    }

    fn decision(response: &mpsc::Receiver<Value>) -> Value {
        response.try_recv().unwrap()["data"]["output"]["hookSpecificOutput"]["decision"].clone()
    }

    #[test]
    fn two_asks_in_one_tab_keep_separate_decisions_and_cards() {
        let mut shell = test_shell();
        shell.window_focused = false;
        shell.settings.current.notifications_enabled = true;
        let (old, old_response) = ask(&mut shell, json!({"tool_name": "Bash"}));
        let (new, new_response) = ask(&mut shell, json!({"tool_name": "Read"}));
        assert_ne!(old, new);
        assert!(shell
            .notification_updates
            .contains_key(&permission_key(&old)));
        assert!(shell
            .notification_updates
            .contains_key(&permission_key(&new)));

        let gesture = Msg::Island(IslandMsg::DecidePermission {
            ask_id: old.clone(),
            decision: PermissionDecision::Allow,
        });
        let _ = shell.update(gesture.clone());
        let _ = shell.update(gesture);
        assert_eq!(decision(&old_response)["behavior"], "allow");
        assert_eq!(new_response.try_recv(), Err(mpsc::TryRecvError::Empty));
        assert_eq!(
            shell
                .status_runtime
                .pane_record("t1")
                .unwrap()
                .pending_ask()
                .unwrap()
                .id,
            new
        );
        let _ = shell.handle_native_notification_action(&permission_key(&new), Action::Deny);
        assert_eq!(decision(&new_response)["behavior"], "deny");
        assert!(shell.pending_permissions.is_empty());
        assert!(shell.notification_updates.is_empty());
    }

    #[test]
    fn late_actions_and_passive_fallback_keep_the_active_tab() {
        for (expired, disabled, action) in [
            (true, false, Action::Allow),
            (true, false, Action::Always),
            (true, false, Action::Deny),
            (true, false, Action::Terminal),
            (false, true, Action::Terminal),
            (false, false, Action::Unavailable),
            (false, false, Action::Terminal),
        ] {
            let mut shell = test_shell();
            shell.window_focused = false;
            shell.settings.current.notifications_enabled = true;
            shell.panes.focused_model_mut().open_terminal(2);
            let (id, response) = ask(&mut shell, json!({"tool_name": "Bash"}));
            if expired {
                shell.pending_permissions.get_mut(&id).unwrap().deadline = Instant::now();
            }
            if disabled {
                shell.settings.current.notifications_enabled = false;
            }
            let _ = shell.handle_native_notification_action(&permission_key(&id), action);
            assert_eq!(
                response.try_recv().unwrap(),
                json!({"ok": true, "data": {}})
            );
            let expected = if !expired && !disabled && action == Action::Terminal {
                1
            } else {
                2
            };
            assert_eq!(
                shell.panes.focused_model().active_tab(),
                Some(&PaneTab::Terminal(expected))
            );
            assert!(shell.pending_permissions.is_empty());
            assert!(shell.notification_updates.is_empty());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_signals_match_both_the_daemon_and_the_notification() {
        use zbus::{names::UniqueName, Message};

        let owner = UniqueName::try_from(":1.4").unwrap();
        for (sender, id, expected) in [
            (":1.5", 42, None),
            (":1.4", 43, None),
            (":1.4", 42, Some(Action::Allow)),
        ] {
            let signal = Message::signal(
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
            )
            .sender(sender)
            .build(&(id as u32, "allow"))
            .unwrap();
            assert_eq!(linux_action(&signal, &owner, 42), expected);
        }
    }

    #[test]
    fn expiring_or_closing_a_tab_releases_its_pending_hook() {
        for close in [false, true] {
            let mut shell = test_shell();
            shell.window_focused = false;
            shell.settings.current.notifications_enabled = true;
            let (id, response) = ask(&mut shell, json!({"tool_name": "Bash"}));
            if close {
                shell.cancel_pending_permissions("t1");
            } else {
                shell.pending_permissions.get_mut(&id).unwrap().deadline = Instant::now();
                assert!(shell.expire_pending_permissions());
                assert!(shell
                    .status_runtime
                    .pane_record("t1")
                    .unwrap()
                    .pending_ask()
                    .is_none());
            }
            let _ = shell.handle_native_notification_action(&permission_key(&id), Action::Allow);
            assert_eq!(
                response.try_recv().unwrap(),
                json!({"ok": true, "data": {}})
            );
            assert!(shell.pending_permissions.is_empty());
            assert!(shell.notification_updates.is_empty());
        }
    }

    #[test]
    fn resolving_the_visible_card_restores_the_previous_pending_card() {
        let mut shell = test_shell();
        shell.window_focused = true;
        shell.settings.current.notifications_enabled = true;
        shell.panes.focused_model_mut().open_terminal(2);
        let (older, older_reply) = ask(&mut shell, json!({"tool_name": "Bash"}));
        let (newer, newer_reply) = ask(&mut shell, json!({"tool_name": "Read"}));
        assert!(shell.notification_updates.is_empty());

        shell.answer_pending_permission(&newer, PermissionDecision::Deny);
        assert_eq!(decision(&newer_reply)["behavior"], "deny");
        assert_eq!(older_reply.try_recv(), Err(mpsc::TryRecvError::Empty));
        assert_eq!(
            shell
                .status_runtime
                .pane_record("t1")
                .unwrap()
                .pending_ask()
                .unwrap()
                .id,
            older
        );
    }

    #[test]
    fn notificacoes_deduplicam_abas_e_permissoes_por_chave() {
        let mut shell = test_shell();
        let _first =
            shell.native_notification_task("tab:t1".into(), "first".into(), String::new(), None);
        let first = shell.notification_updates["tab:t1"].clone();
        let _replacement =
            shell.native_notification_task("tab:t1".into(), "updated".into(), String::new(), None);
        let _different = shell.native_notification_task(
            "ask:t1".into(),
            "permission".into(),
            String::new(),
            Some(false),
        );
        assert_eq!(shell.notification_updates.len(), 2);
        assert!(first.same_channel(&shell.notification_updates["tab:t1"]));
        assert!(!first.same_channel(&shell.notification_updates["ask:t1"]));
        #[cfg(target_os = "linux")]
        assert_eq!(first.borrow().summary, "updated");
        #[cfg(not(target_os = "linux"))]
        assert_eq!(first.borrow().summary, "first");
    }
}
