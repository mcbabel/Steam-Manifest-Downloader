//! Steam account (QR sign-in only) and branches in the terminal UI.
//! The token never leaves smd-core: this only sees AccountInfo, QR codes and
//! branch lists.

use std::time::Duration;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use smd_core::ops::account::{self as account_ops, BranchList, QrStatus};
use smd_core::services::steam_account::{AccountInfo, Storage};
use smd_core::services::steam_branches;

use super::action::{Action, InputId};
use super::state::{DepotRow, Parsed};
use super::{apply, hint, App};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::input::TextInput;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{Ctx, Fid, ScrollView};

pub struct AccountUi {
    pub info: Option<AccountInfo>,
    pub loaded: bool,
    pub login_running: bool,
    pub qr: Option<Vec<String>>,
    pub scanned: bool,
    pub expired: bool,
    pub interval: f32,
    pub status: Option<(Tone, String)>,
    pub branches: Option<BranchList>,
    pub branches_app: Option<String>,
    pub branch: String,
    pub branch_password: TextInput,
    pub use_account: bool,
    /// The depots of the current selection were read from the account.
    pub from_account: bool,
    pub loading_depots: bool,
}

impl Default for AccountUi {
    fn default() -> Self {
        Self {
            info: None,
            loaded: false,
            login_running: false,
            qr: None,
            scanned: false,
            expired: false,
            interval: 5.0,
            status: None,
            branches: None,
            branches_app: None,
            branch: steam_branches::PUBLIC.to_string(),
            branch_password: TextInput::default().masked(),
            use_account: false,
            from_account: false,
            loading_depots: false,
        }
    }
}

impl AccountUi {
    pub fn non_public(&self) -> bool {
        !steam_branches::is_public(&self.branch)
    }

    fn selected_view(&self) -> Option<&steam_branches::BranchView> {
        self.branches
            .as_ref()?
            .branches
            .iter()
            .find(|b| b.name == self.branch)
    }

    /// The selected branch needs a password the user has not typed yet.
    pub fn needs_password(&self) -> bool {
        self.non_public()
            && self
                .selected_view()
                .is_some_and(|b| b.password_required && !b.available)
    }

    pub fn uses_account(&self) -> bool {
        self.from_account || self.non_public() || (self.info.is_some() && self.use_account)
    }
}

fn qr_lines(url: &str) -> Option<Vec<String>> {
    smd_core::services::steam_auth::qr_text(url, false).ok()
}

fn storage_text(info: &AccountInfo) -> String {
    t(match info.storage {
        Storage::Keychain => "account.storedKeychain",
        Storage::File => "account.storedFile",
        Storage::Session => "account.storedSession",
    })
}

impl App {
    pub(super) fn account_refresh(&mut self) {
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let info = account_ops::status(&core, &dir).await;
            apply(move |app| {
                app.acct.info = info;
                app.acct.loaded = true;
            })
        });
    }

    pub(super) fn dispatch_account(&mut self, a: &Action) -> bool {
        match a {
            Action::AccountLogin => self.account_login(),
            Action::AccountLoginCancel => {
                self.acct.login_running = false;
                self.acct.qr = None;
                self.acct.status = None;
                let core = self.core.clone();
                self.spawn(async move {
                    account_ops::cancel_qr(&core).await;
                    apply(|_| {})
                });
                self.emit(
                    "account_login",
                    Some(serde_json::json!({ "step": "cancelled" })),
                );
            }
            Action::AccountLogout => self.account_logout(),
            Action::BranchCycle(step) => self.branch_cycle(*step),
            Action::ToggleUseAccount => {
                if self.acct.info.is_some() && !self.acct.non_public() && !self.acct.from_account {
                    self.acct.use_account = !self.acct.use_account;
                }
            }
            Action::LoadFromAccount => self.load_from_account(),
            _ => return false,
        }
        true
    }

    fn account_login(&mut self) {
        self.acct.login_running = true;
        self.acct.qr = None;
        self.acct.scanned = false;
        self.acct.expired = false;
        self.acct.status = Some((Tone::Info, t("account.qrLoading")));
        self.emit(
            "account_login",
            Some(serde_json::json!({ "step": "start" })),
        );
        let core = self.core.clone();
        self.spawn(async move {
            let r = account_ops::start_qr(&core).await;
            apply(move |app| {
                if !app.acct.login_running {
                    return;
                }
                match r {
                    Ok(qr) => {
                        app.acct.qr = qr_lines(&qr.challenge_url);
                        app.acct.interval = qr.interval_secs.max(1.0);
                        app.acct.status = Some((Tone::Info, t("account.qrWaiting")));
                        app.account_poll_later();
                    }
                    Err(e) => {
                        app.acct.login_running = false;
                        app.acct.status = Some((Tone::Error, crate::i18n::localize_error(&e)));
                        app.emit(
                            "account_login",
                            Some(serde_json::json!({ "step": "failed", "reason": "start" })),
                        );
                    }
                }
            })
        });
    }

    fn account_poll_later(&mut self) {
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        let wait = Duration::from_secs_f32(self.acct.interval);
        self.spawn(async move {
            tokio::time::sleep(wait).await;
            // The terminal UI is interactive like the desktop app: without a
            // keychain the sign-in lasts for the session (smd login
            // --store-file is the explicit file option).
            let r = account_ops::poll_qr(&core, &dir, false).await;
            apply(move |app| {
                if !app.acct.login_running {
                    return;
                }
                match r {
                    Ok(QrStatus::Waiting { scanned, qr }) => {
                        if let Some(q) = qr {
                            app.acct.qr = qr_lines(&q.challenge_url);
                            app.acct.interval = q.interval_secs.max(1.0);
                        }
                        app.acct.scanned = scanned;
                        let key = if scanned {
                            "account.qrScanned"
                        } else {
                            "account.qrWaiting"
                        };
                        app.acct.status = Some((Tone::Info, t(key)));
                        app.account_poll_later();
                    }
                    Ok(QrStatus::Done { account }) => {
                        app.acct.login_running = false;
                        app.acct.qr = None;
                        app.acct.status = None;
                        app.toast(
                            Tone::Success,
                            tf("account.signedInToast", &[("name", &account.account_name)]),
                        );
                        let storage = serde_json::to_value(account.storage).unwrap_or_default();
                        app.emit(
                            "account_login",
                            Some(serde_json::json!({ "step": "done", "storage": storage })),
                        );
                        app.acct.info = Some(account);
                        app.acct.branches_app = None;
                        app.branch_load();
                    }
                    Ok(QrStatus::Expired) => {
                        app.acct.login_running = false;
                        app.acct.qr = None;
                        app.acct.expired = true;
                        app.acct.status = Some((Tone::Warning, t("account.qrExpired")));
                        app.emit(
                            "account_login",
                            Some(serde_json::json!({ "step": "expired" })),
                        );
                    }
                    Ok(QrStatus::Idle) => app.acct.login_running = false,
                    Err(e) => {
                        app.acct.login_running = false;
                        app.acct.qr = None;
                        app.acct.status = Some((Tone::Error, crate::i18n::localize_error(&e)));
                        app.emit(
                            "account_login",
                            Some(serde_json::json!({ "step": "failed", "reason": "poll" })),
                        );
                    }
                }
            })
        });
    }

    fn account_logout(&mut self) {
        let name = self
            .acct
            .info
            .as_ref()
            .map(|i| i.account_name.clone())
            .unwrap_or_default();
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            account_ops::logout(&core, &dir).await;
            apply(move |app| {
                app.acct.info = None;
                app.acct.use_account = false;
                app.acct.branches_app = None;
                app.branch_load();
                app.toast(
                    Tone::Success,
                    tf("account.signedOutToast", &[("name", &name)]),
                );
                app.emit("account_logout", None);
            })
        });
    }

    /// Loads the branches of the app in the selection (again after a
    /// sign-in or sign-out).
    pub(super) fn branch_load(&mut self) {
        let Some(app_id) = self.wiz.parsed.as_ref().map(|p| p.main_app_id.clone()) else {
            return;
        };
        if self.acct.branches_app.as_deref() == Some(app_id.as_str()) {
            return;
        }
        self.acct.branches_app = Some(app_id.clone());
        let Ok(n) = app_id.parse::<u32>() else {
            return;
        };
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = account_ops::branches(&core, &dir, n).await;
            apply(move |app| {
                if app.acct.branches_app.as_deref() != Some(app_id.as_str()) {
                    return;
                }
                if let Ok(list) = r {
                    if !list.branches.iter().any(|b| b.name == app.acct.branch) {
                        app.acct.branch = steam_branches::PUBLIC.to_string();
                    }
                    app.acct.branches = Some(list);
                }
            })
        });
    }

    /// Resets the branch choice for a new selection.
    pub(super) fn branch_reset(&mut self, from_account: bool) {
        self.acct.branch = steam_branches::PUBLIC.to_string();
        self.acct.branch_password.clear();
        self.acct.branches = None;
        self.acct.branches_app = None;
        self.acct.use_account = from_account;
        self.acct.from_account = from_account;
        if !self.acct.loaded {
            self.account_refresh();
        }
        self.branch_load();
    }

    fn branch_cycle(&mut self, step: i32) {
        let names: Vec<String> = match &self.acct.branches {
            Some(l) if !l.branches.is_empty() => {
                l.branches.iter().map(|b| b.name.clone()).collect()
            }
            _ => return,
        };
        let cur = names
            .iter()
            .position(|n| *n == self.acct.branch)
            .unwrap_or(0) as i32;
        let next = (cur + step).rem_euclid(names.len() as i32) as usize;
        self.acct.branch = names[next].clone();
        self.acct.branch_password.clear();
    }

    /// Message key when the branch choice blocks the start.
    pub(super) fn branch_problem(&self) -> Option<&'static str> {
        if !self.acct.non_public() {
            return None;
        }
        if self.acct.info.is_none() {
            return Some("branch.needsLogin");
        }
        if self.acct.needs_password() && self.acct.branch_password.trimmed().is_empty() {
            return Some("branch.needsPassword");
        }
        None
    }

    /// Branch, password and account flag for the download config.
    pub(super) fn branch_fields(&self) -> (Option<String>, Option<String>, bool) {
        let branch = self.acct.non_public().then(|| self.acct.branch.clone());
        let password = branch
            .as_ref()
            .map(|_| self.acct.branch_password.trimmed())
            .filter(|p| !p.is_empty());
        (branch, password, self.acct.uses_account())
    }

    fn load_from_account(&mut self) {
        if self.acct.info.is_none() {
            self.toast(Tone::Error, t("branch.needsLogin"));
            return;
        }
        let Some(app_id) = self.wiz.search_app_id.clone() else {
            return;
        };
        let Ok(n) = app_id.parse::<u32>() else {
            return;
        };
        self.acct.loading_depots = true;
        self.wiz.search_error = None;
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = account_ops::app_depots(&core, &dir, n, None, None).await;
            apply(move |app| {
                app.acct.loading_depots = false;
                match r {
                    Ok(depots) if !depots.is_empty() => {
                        app.emit(
                            "account_depots",
                            Some(serde_json::json!({ "count": depots.len() })),
                        );
                        app.wiz.search_repo = None;
                        app.wiz.search_key_vdf = None;
                        app.wiz.source_type = Some("account".into());
                        app.wiz.parsed = Some(Parsed {
                            main_app_id: app_id,
                            depots: depots
                                .into_iter()
                                .map(|d| DepotRow::new(d.depot_id, Some(d.manifest_id), None, None))
                                .collect(),
                            all_app_ids: Vec::new(),
                        });
                        app.wiz.from_search = true;
                        app.show_selection_step();
                    }
                    Ok(_) => app.wiz.search_error = Some(t("account.noDepots")),
                    Err(e) => app.wiz.search_error = Some(crate::i18n::localize_error(&e)),
                }
            })
        });
    }

    // ---- rendering ----------------------------------------------------------

    pub(super) fn render_account_tab(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("account.title"));
        let status = match &self.acct.info {
            Some(info) => format!(
                "{} {}",
                tf("account.signedInAs", &[("name", &info.account_name)]),
                storage_text(info)
            ),
            None => t("account.signedOut"),
        };
        let h = widgets::wrap_height(&status, w);
        let r = sv.next(h);
        widgets::paragraph(&mut sv.buf, r, &status, theme::text());
        let note = t("account.note");
        let h = widgets::wrap_height(&note, w);
        let r = sv.next(h);
        hint(&mut sv.buf, r, &note);
        sv.gap(1);

        let row = sv.next(1);
        let mut specs = Vec::new();
        if self.acct.login_running {
            specs.push(ButtonSpec::new(
                t("settings.cancel"),
                Fid::new("account.cancel"),
                Action::AccountLoginCancel,
                Btn::Secondary,
            ));
        } else if self.acct.info.is_some() {
            specs.push(ButtonSpec::new(
                t("account.signOut"),
                Fid::new("account.logout"),
                Action::AccountLogout,
                Btn::Danger,
            ));
        } else {
            let label = if self.acct.expired {
                t("account.qrRetry")
            } else {
                t("account.signIn")
            };
            specs.push(ButtonSpec::new(
                label,
                Fid::new("account.login"),
                Action::AccountLogin,
                Btn::Primary,
            ));
        }
        widgets::buttons(&mut sv.buf, ctx, row, &specs, false);
        sv.gap(1);

        if let Some((tone, msg)) = &self.acct.status {
            let h = widgets::status_height(msg, w);
            let r = sv.next(h);
            widgets::status(&mut sv.buf, r, *tone, msg, self.tick);
        }
        if let Some(lines) = &self.acct.qr {
            let intro = t("account.qrIntro");
            let h = widgets::wrap_height(&intro, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &intro);
            sv.gap(1);
            // Black on white whatever the terminal theme, so phones can read it.
            let style = Style::default().fg(Color::Black).bg(Color::White);
            for l in lines {
                let r = sv.next(1);
                widgets::line(&mut sv.buf, r, Line::from(Span::styled(l.clone(), style)));
            }
            sv.gap(1);
            let note = t("account.qrNote");
            let h = widgets::wrap_height(&note, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &note);
        }
    }

    /// Branch row, password and the account toggle in the selection panel.
    pub(super) fn render_branch_options(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let others = self
            .acct
            .branches
            .as_ref()
            .map_or(0, |l| l.branches.len().saturating_sub(1));
        if self.acct.info.is_none() {
            if others > 0 {
                let text = tf("branch.moreBranches", &[("count", &others)]);
                let h = widgets::wrap_height(&text, w);
                let r = sv.next(h);
                hint(&mut sv.buf, r, &text);
                sv.gap(1);
            }
            return;
        }
        if others > 0 || self.acct.non_public() {
            self.render_branch_picker(sv, ctx);
        }
        self.render_branch_password(sv, ctx);
        let r = sv.next(1);
        let locked = self.acct.non_public() || self.acct.from_account;
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r,
            self.acct.uses_account(),
            &t("branch.useAccount"),
            Fid::new("branch.useAccount"),
            Action::ToggleUseAccount,
            !locked,
        );
        if self.acct.non_public() {
            let text = t("branch.hintBranch");
            let h = widgets::wrap_height(&text, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &text);
        }
        sv.gap(1);
    }

    fn render_branch_picker(&mut self, sv: &mut ScrollView, ctx: &mut Ctx) {
        let r = sv.next(1);
        let label = format!("{}: ", t("branch.label"));
        widgets::text(&mut sv.buf, r, &label, theme::text());
        let mut x = r.x + widgets::width(&label);
        let view = self.acct.selected_view().cloned();
        let mut name = self.acct.branch.clone();
        if let Some(v) = &view {
            if v.password_required {
                name.push_str(&format!(" ({})", t("branch.passwordTag")));
            } else if !v.available {
                name.push_str(&format!(" ({})", t("branch.lockedTag")));
            }
        }
        let many = self
            .acct
            .branches
            .as_ref()
            .is_some_and(|l| l.branches.len() > 1);
        for (label, fid, step) in [("◂", "branch.prev", -1), ("▸", "branch.next", 1)] {
            if step == 1 {
                widgets::text(
                    &mut sv.buf,
                    ratatui::layout::Rect::new(x, r.y, r.right().saturating_sub(x), 1),
                    &name,
                    theme::accent(),
                );
                x += widgets::width(&name) + 1;
            }
            let room = r.right().saturating_sub(x);
            x += widgets::button(
                &mut sv.buf,
                ctx,
                x,
                r.y,
                room,
                &ButtonSpec::new(
                    label.to_string(),
                    Fid::new(fid),
                    Action::BranchCycle(step),
                    Btn::Secondary,
                )
                .enabled(many),
            ) + 1;
        }
    }

    fn render_branch_password(&mut self, sv: &mut ScrollView, ctx: &mut Ctx) {
        if self.acct.needs_password() {
            let mask_label = if self.acct.branch_password.masked {
                t("tui.show")
            } else {
                t("tui.hide")
            };
            let mask = ButtonSpec::new(
                mask_label,
                Fid::new("branch.mask"),
                Action::ToggleMask(InputId::BranchPassword),
                Btn::Secondary,
            );
            let row = sv.next(3);
            super::source::input_with_button(
                &mut sv.buf,
                ctx,
                row,
                &mut self.acct.branch_password,
                InputSpec {
                    label: &t("branch.passwordPlaceholder"),
                    placeholder: &t("branch.passwordPlaceholder"),
                    fid: Fid::new("branch.password"),
                    id: InputId::BranchPassword,
                    error: false,
                },
                &mask,
            );
        }
    }
}
