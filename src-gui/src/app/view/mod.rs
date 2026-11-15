#![allow(clippy::all)]

mod modals;
mod settings;
mod shell;
mod step_emulator;
mod step_progress;
mod step_select;
mod step_shortcut;
mod step_steam_library;
mod step_upload;

use crate::app::ui_state::UiState;

pub const MODALS: &[(&str, fn(&mut crate::app::App, &mut crate::dom::Dom<'_>), bool)] = &[
    ("cancel-modal", crate::app::App::view_cancel_modal, true),
    ("shutdown-modal", crate::app::App::view_shutdown_modal, true),
    ("followup-modal", crate::app::App::view_followup_modal, true),
    ("close-modal", crate::app::App::view_close_modal, true),
    ("remove-source-modal", crate::app::App::view_remove_source_modal, true),
    ("history-remove-modal", crate::app::App::view_history_remove_modal, true),
    ("emu-revert-modal", crate::app::App::view_emu_revert_modal, true),
    ("emu-download-modal", crate::app::App::view_emu_download_modal, true),
    ("history-clear-modal", crate::app::App::view_history_clear_modal, true),
    ("telemetry-modal", crate::app::App::view_telemetry_modal, true),
    ("behavior-modal", crate::app::App::view_behavior_modal, true),
    ("bug-report-modal", crate::app::App::view_bug_report_modal, true),
    ("settings-modal", crate::app::App::view_settings_modal, true),
    ("whats-new-modal", crate::app::App::view_whats_new_modal, true),
    ("update-modal", crate::app::App::view_update_modal, true),
    ("language-picker", crate::app::App::view_language_picker, true),
    ("welcome-modal", crate::app::App::view_welcome_modal, true),
    ("history-modal", crate::app::App::view_history_modal, true),
    ("resume-modal", crate::app::App::view_resume_modal, true),
    ("launch-modal", crate::app::App::view_launch_modal, true),
    ("queue-modal", crate::app::App::view_queue_modal, true),
    ("confirm-modal", crate::app::App::view_confirm_modal, true),
    ("account-modal", crate::app::App::view_account_modal, false),
];

pub const MODAL_CANCEL: &[(&str, &str)] = &[
    ("cancel-modal", "btn-cancel-no"),
    ("shutdown-modal", "btn-shutdown-abort"),
    ("followup-modal", "btn-followup-later"),
    ("close-modal", "btn-close-no"),
    ("remove-source-modal", "btn-remove-source-no"),
    ("history-remove-modal", "btn-history-remove-no"),
    ("emu-revert-modal", "btn-emu-revert-no"),
    ("emu-download-modal", "btn-emu-download-no"),
    ("history-clear-modal", "btn-history-clear-no"),
    ("telemetry-modal", "btn-telemetry-decline"),
    ("behavior-modal", "btn-behavior-decline"),
    ("bug-report-modal", "btn-bug-cancel"),
    ("settings-modal", "btn-settings-cancel"),
    ("whats-new-modal", "btn-whats-new-close"),
    ("update-modal", "btn-update-later"),
    ("welcome-modal", "btn-welcome-close"),
    ("history-modal", "btn-history-close"),
    ("resume-modal", "btn-resume-cancel"),
    ("launch-modal", "btn-launch-cancel"),
    ("queue-modal", "btn-queue-close"),
    ("confirm-modal", "btn-confirm-no"),
    ("account-modal", "btn-account-cancel"),
];

pub const SETTINGS_ROWS: &[(&str, &str, &str, &str, &[&str], &[&str])] = &[
    ("settings-row-3", "settings-group-2", "", "general", &["settings.language", "settings.languageHint"], &[]),
    ("settings-row-4", "settings-group-2", "", "general", &["settings.autoUpdate", "settings.autoUpdateHint"], &[]),
    ("settings-row-5", "settings-group-2", "", "general", &["settings.notificationSound", "settings.notificationSoundHint"], &[]),
    ("settings-row-6", "settings-group-3", "", "downloads", &["settings.nativeDownloaderToggle", "settings.nativeDownloaderHint"], &[]),
    ("settings-row-7", "settings-group-3", "", "downloads", &["settings.speedLimit", "settings.speedLimitHint"], &[]),
    ("settings-row-8", "settings-group-3", "", "downloads", &["settings.folderStyle", "settings.folderStyleHint"], &[]),
    ("settings-row-9", "settings-group-3", "", "downloads", &["settings.cancelKeepFilesToggle", "settings.cancelKeepFilesHint"], &[]),
    ("settings-row-10", "settings-group-4", "settings-title-1", "downloads", &["settings.maxRetries", "settings.maxRetriesHint"], &["settings.groupFineTuning"]),
    ("settings-row-11", "settings-group-4", "settings-title-1", "downloads", &["settings.chunkConcurrency", "settings.chunkConcurrencyHint"], &["settings.groupFineTuning"]),
    ("dd-extra-args-row", "settings-group-4", "settings-title-1", "downloads", &["settings.ddExtraArgs", "settings.ddExtraArgsHint"], &["settings.groupFineTuning"]),
    ("settings-row-12", "settings-group-5", "", "depots", &["settings.autoSelectDepots", "settings.autoSelectDepotsHint"], &[]),
    ("settings-row-13", "settings-group-5", "", "depots", &["settings.latestManifests", "settings.latestManifestsHint"], &[]),
    ("settings-row-14", "settings-group-6", "settings-title-2", "depots", &["settings.autoStartDownload", "settings.autoStartDownloadHint"], &["settings.groupLikeSteam"]),
    ("settings-row-15", "settings-group-6", "settings-title-2", "depots", &["settings.includeDlc", "settings.includeDlcHint"], &["settings.groupLikeSteam"]),
    ("settings-row-16", "settings-group-6", "settings-title-2", "depots", &["settings.targetPlatform", "settings.targetPlatformHint", "settings.auto"], &["settings.groupLikeSteam"]),
    ("settings-row-17", "settings-group-6", "settings-title-2", "depots", &["settings.gameLanguage"], &["settings.groupLikeSteam"]),
    ("settings-row-18", "settings-group-7", "", "sources", &["settings.depotSources", "settings.depotSourcesHint", "settings.add"], &[]),
    ("settings-row-19", "settings-group-7", "", "sources", &["settings.hubcapApiKey", "settings.hubcapApiKeyHint"], &[]),
    ("settings-row-20", "settings-group-7", "", "sources", &["settings.ryuuApiKey", "settings.ryuuApiKeyHint"], &[]),
    ("settings-row-21", "settings-group-8", "settings-title-3", "sources", &["settings.proxy", "settings.proxyHint", "settings.proxyTest"], &["settings.groupNetwork"]),
    ("settings-row-22", "settings-group-8", "settings-title-3", "sources", &["settings.downloadServer", "settings.downloadServerHint", "settings.downloadServerAuto", "settings.downloadServerCustom", "settings.downloadServerCustomHint"], &["settings.groupNetwork"]),
    ("settings-row-23", "settings-group-9", "", "steam", &["settings.steamWebApiKey", "settings.steamWebApiKeyHint"], &[]),
    ("settings-row-24", "settings-group-9", "", "steam", &["settings.steamPath", "settings.steamPathHint", "shortcut.browse"], &[]),
    ("settings-row-25", "account-group", "settings-title-4", "steam", &["account.signedOut", "account.signedOutHint", "account.note", "account.signIn", "account.signOut"], &["account.title"]),
    ("settings-row-26", "settings-group-10", "", "privacy", &["settings.telemetry", "settings.telemetryHint"], &[]),
    ("settings-row-27", "settings-group-10", "", "privacy", &["settings.remoteUpdates", "settings.remoteUpdatesHint"], &[]),
    ("behavior-row", "settings-group-10", "", "privacy", &["settings.telemetryBehavior", "settings.telemetryBehaviorHint"], &[]),
    ("settings-row-28", "settings-group-11", "settings-title-5", "privacy", &["settings.debugInfoChannel", "settings.debugInfoVersion", "settings.debugInfoCommit", "settings.debugInfoBuildDate", "settings.debugInfoProfile", "settings.debugInfoPlatform", "settings.debugInfoDiagnosticId", "settings.copyDebugInfo", "bugReport.title", "settings.debugInfoHint"], &["settings.debugInfo"]),
];

pub const SETTINGS_INPUTS: &[(&str, &str)] = &[
    ("auto-update-toggle", "checkbox"),
    ("notification-sound-toggle", "checkbox"),
    ("native-downloader-toggle", "checkbox"),
    ("speed-limit-toggle", "checkbox"),
    ("speed-limit-input", "number"),
    ("cancel-keep-files-toggle", "checkbox"),
    ("max-retries-input", "number"),
    ("chunk-concurrency-input", "number"),
    ("dd-extra-args-input", "text"),
    ("auto-select-depots-toggle", "checkbox"),
    ("latest-manifests-toggle", "checkbox"),
    ("auto-start-download-toggle", "checkbox"),
    ("include-dlc-toggle", "checkbox"),
    ("sources-add-input", "text"),
    ("hubcap-apikey-input", "text"),
    ("ryuu-apikey-input", "text"),
    ("proxy-input", "text"),
    ("download-server-select", "select"),
    ("download-server-input", "text"),
    ("steam-webapi-key-input", "password"),
    ("steam-path-input", "text"),
    ("telemetry-toggle", "checkbox"),
    ("remote-updates-toggle", "checkbox"),
    ("behavior-toggle", "checkbox"),
];

pub const I18N_IDS: &[&str] = &[
    "dotnet-install-link",
    "btn-sources-empty-add",
    "btn-sources-empty-upload",
    "btn-search-account",
    "btn-auto-select",
    "btn-latest-manifests",
    "btn-select-all",
    "btn-deselect-all",
    "btn-branch-login",
    "btn-queue-add",
    "disk-space-text",
    "btn-pause",
    "btn-report-failure",
    "btn-next-step",
    "btn-browse-exe",
    "shortcut-steam-hint",
    "btn-shortcut-steam-choose",
    "btn-shortcut-skip",
    "btn-create-shortcuts",
    "steam-library-status",
    "btn-steam-browse-exe",
    "btn-steam-skip",
    "btn-steam-add",
    "emu-header",
    "emu-description",
    "emu-release-status",
    "emu-drm-title",
    "emu-drm-description",
    "btn-emu-drm-remove",
    "emu-file-empty",
    "btn-emu-cold-steam",
    "btn-emu-webapi-save",
    "emu-gamedata-media-label",
    "btn-emu-gamedata",
    "btn-emu-controller-file",
    "btn-emu-controller-clear",
    "emu-settings-hint",
    "btn-emu-settings-reset",
    "emu-settings-empty",
    "btn-emu-saves-open",
    "btn-emu-saves-backup",
    "btn-emu-saves-restore",
    "btn-emu-ach-reset",
    "btn-emu-stats-reset",
    "btn-emu-avatar",
    "btn-emu-avatar-clear",
    "emu-asset-value-font",
    "emu-asset-value-achievement",
    "emu-asset-value-friend",
    "btn-emu-broadcasts",
    "emu-lobby-later",
    "btn-emu-lobby",
    "emu-mods-empty",
    "btn-emu-mod-folder",
    "btn-emu-mod-file",
    "emu-bypass-hint",
    "emu-dlc-merge-hint",
    "btn-emu-merge-dlcs",
    "btn-emu-new",
    "btn-emu-revert",
    "btn-emu-save-settings",
    "btn-emu-apply",
    "cancel-modal-title",
    "cancel-modal-body",
    "btn-cancel-no",
    "btn-cancel-yes",
    "shutdown-modal-title",
    "btn-shutdown-abort",
    "btn-shutdown-now",
    "followup-modal-title",
    "btn-followup-later",
    "btn-followup-discard",
    "btn-followup-resume",
    "close-modal-title",
    "btn-close-no",
    "btn-close-yes",
    "remove-source-modal-title",
    "btn-remove-source-no",
    "btn-remove-source-yes",
    "history-remove-modal-title",
    "history-remove-body",
    "btn-history-remove-no",
    "btn-history-remove-yes",
    "emu-revert-modal-title",
    "btn-emu-revert-no",
    "btn-emu-revert-yes",
    "emu-download-modal-title",
    "btn-emu-download-no",
    "btn-emu-download-yes",
    "history-clear-modal-title",
    "history-clear-body",
    "btn-history-clear-no",
    "btn-history-clear-yes",
    "btn-telemetry-decline",
    "btn-telemetry-basic",
    "btn-telemetry-accept",
    "btn-behavior-decline",
    "btn-behavior-accept",
    "btn-bug-cancel",
    "btn-bug-open",
    "settings-no-results",
    "native-downloader-hint",
    "btn-sources-add",
    "btn-proxy-test",
    "btn-steam-path-browse",
    "account-status",
    "account-detail",
    "btn-account-login",
    "btn-account-logout",
    "btn-copy-build-info",
    "btn-settings-report-bug",
    "settings-dirty",
    "btn-settings-cancel",
    "btn-settings-save",
    "btn-whats-new-github",
    "btn-whats-new-close",
    "update-modal-title",
    "update-system-text",
    "update-progress-text",
    "btn-update-skip",
    "btn-update-notes-link",
    "btn-update-later",
    "btn-update-now",
    "language-picker-title",
    "welcome-title",
    "btn-welcome-location",
    "btn-welcome-close",
    "btn-history-clear",
    "btn-history-close",
    "resume-modal-title",
    "btn-resume-cancel",
    "btn-resume-verify",
    "btn-resume-fast",
    "launch-modal-title",
    "btn-launch-cancel",
    "btn-launch-start",
    "queue-modal-title",
    "btn-queue-clear",
    "btn-queue-close",
    "btn-queue-start",
    "account-modal-title",
    "btn-account-cancel",
    "btn-account-retry",
];

pub fn declare(ui: &mut UiState) {
    ui.declare("title-bar", &["title-bar"]);
    ui.declare("title-bar", &["title-bar"]);
    ui.declare("btn-minimize", &["title-bar__btn", "title-bar__btn--minimize"]);
    ui.declare("btn-maximize", &["title-bar__btn", "title-bar__btn--maximize"]);
    ui.declare("btn-close", &["title-bar__btn", "title-bar__btn--close"]);
    ui.declare("app", &["app"]);
    ui.declare("btn-settings", &["btn", "btn--outline", "header__settings-btn"]);
    ui.declare("btn-history", &["btn", "btn--outline", "header__history-btn"]);
    ui.declare("btn-queue", &["btn", "btn--outline", "header__queue-btn"]);
    ui.declare("queue-badge", &["header__badge", "hidden"]);
    ui.declare("btn-report-bug", &["btn", "btn--outline", "header__bug-btn"]);
    ui.declare("btn-theme-toggle", &["btn", "btn--outline", "header__theme-btn"]);
    ui.declare("offline-banner", &["app-banner", "app-banner--warning", "hidden"]);
    ui.declare("remote-notices", &["app-notices"]);
    ui.declare("dotnet-warning", &["dotnet-warning", "hidden"]);
    ui.declare("dotnet-warning", &["dotnet-warning", "hidden"]);
    ui.declare("dotnet-install-link", &["btn", "btn--small", "dotnet-warning__btn"]);
    ui.set_attr("dotnet-install-link", "href", "https://dotnet.microsoft.com/en-us/download/dotnet/thank-you/runtime-desktop-9.0.16-windows-x64-installer");
    ui.declare("dotnet-warning-dismiss", &["dotnet-warning__dismiss"]);
    ui.set_text("dotnet-warning-dismiss", "×");
    ui.declare("steps-indicator", &["steps"]);
    ui.declare("steps-indicator", &["steps"]);
    ui.declare("steps-item-1", &["steps__item", "active"]);
    ui.set_text("steps-number-1", "1");
    ui.declare("steps-item-2", &["steps__item"]);
    ui.set_text("steps-number-2", "2");
    ui.declare("steps-item-3", &["steps__item"]);
    ui.set_text("steps-number-3", "3");
    ui.declare("step4-connector", &["steps__connector", "steps__connector--shortcut", "hidden"]);
    ui.declare("step4-indicator", &["steps__item", "steps__item--shortcut", "hidden"]);
    ui.set_text("steps-number-4", "4");
    ui.declare("step6-connector", &["steps__connector", "steps__connector--steam", "hidden"]);
    ui.declare("step6-indicator", &["steps__item", "steps__item--steam", "hidden"]);
    ui.set_text("steps-number-6", "6");
    ui.declare("step5-connector", &["steps__connector", "steps__connector--emulator"]);
    ui.declare("step5-indicator", &["steps__item", "steps__item--emulator"]);
    ui.set_text("steps-number-5", "5");
    ui.declare("step-upload", &["step", "active"]);
    ui.declare("step-upload", &["step", "active"]);
    ui.declare("tab-upload", &["tab-bar__tab", "active"]);
    ui.declare("tab-search", &["tab-bar__tab"]);
    ui.declare("tab-patch-only", &["tab-bar__tab"]);
    ui.declare("tab-content-upload", &["tab-content", "active"]);
    ui.declare("tab-content-upload", &["tab-content", "active"]);
    ui.declare("drop-zone", &["drop-zone"]);
    ui.declare("file-info", &["file-info", "hidden"]);
    ui.declare("file-name", &["file-info__name"]);
    ui.declare("file-remove", &["file-info__remove"]);
    ui.set_text("file-remove", "×");
    ui.declare("upload-error", &["error-message", "hidden"]);
    ui.declare("upload-choice", &["next-steps", "hidden"]);
    ui.declare("upload-loading", &["loading", "hidden"]);
    ui.declare("tab-content-search", &["tab-content"]);
    ui.declare("tab-content-search", &["tab-content"]);
    ui.declare("sources-empty-state", &["sources-empty", "hidden"]);
    ui.declare("sources-empty-input", &["search-input"]);
    ui.declare_input("sources-empty-input", "text", "");
    ui.declare("btn-sources-empty-add", &["btn", "btn--primary"]);
    ui.declare("sources-empty-error", &["error-message", "hidden"]);
    ui.declare("btn-sources-empty-upload", &["btn", "btn--small", "btn--outline"]);
    ui.declare("search-game-banner", &["game-info-banner", "hidden"]);
    ui.declare("search-game-image", &["game-info-banner__image"]);
    ui.declare("search-game-name", &["game-info-banner__title"]);
    ui.declare("search-game-description", &["game-info-banner__description"]);
    ui.declare("search-input-block", &["search-input-row", "hidden"]);
    ui.declare("search-appid-input", &["search-input"]);
    ui.declare_input("search-appid-input", "text", "");
    ui.declare("search-autocomplete", &["search-autocomplete", "hidden"]);
    ui.declare("btn-search", &["btn", "btn--primary"]);
    ui.declare("search-error", &["error-message", "hidden"]);
    ui.declare("search-next-steps", &["next-steps", "hidden"]);
    ui.declare("search-loading", &["loading", "hidden"]);
    ui.declare("search-results", &["search-results", "hidden"]);
    ui.declare("repo-list", &["repo-list"]);
    ui.declare("search-next-row", &["search-next-row", "hidden"]);
    ui.declare("btn-search-next", &["btn", "btn--primary"]);
    ui.declare("search-account-row", &["search-account-row", "hidden"]);
    ui.declare("btn-search-account", &["btn", "btn--small", "btn--outline"]);
    ui.declare("search-playtest-row", &["search-account-row", "search-playtest-row", "hidden"]);
    ui.declare("manifest-loading", &["loading", "hidden"]);
    ui.declare("tab-content-patch-only", &["tab-content"]);
    ui.declare("tab-content-patch-only", &["tab-content"]);
    ui.declare("patch-only-dir", &["dd-path__input"]);
    ui.declare_input("patch-only-dir", "text", "");
    ui.declare("btn-patch-only-browse", &["btn", "btn--small", "btn--outline", "dd-path__browse-btn"]);
    ui.declare("patch-only-appid", &["search-input"]);
    ui.declare_input("patch-only-appid", "text", "");
    ui.declare("patch-only-autocomplete", &["search-autocomplete", "hidden"]);
    ui.declare("patch-only-error", &["error-message", "hidden"]);
    ui.declare("patch-only-loading", &["loading", "hidden"]);
    ui.declare("btn-patch-only-start", &["btn", "btn--primary"]);
    ui.declare("step-select", &["step", "hidden"]);
    ui.declare("step-select", &["step", "hidden"]);
    ui.declare("game-info-banner", &["game-info-banner", "hidden"]);
    ui.declare("game-header-image", &["game-info-banner__image"]);
    ui.declare("game-name", &["game-info-banner__title"]);
    ui.declare("game-description", &["game-info-banner__description"]);
    ui.declare("game-info-loading", &["loading", "hidden"]);
    ui.declare("denuvo-note", &["denuvo-note", "hidden"]);
    ui.declare("app-id-display", &["gradient-text"]);
    ui.declare("depot-count", &[]);
    ui.declare("btn-auto-select", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.set_disabled("btn-auto-select", true);
    ui.declare("btn-auto-select-dlc", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.set_text("btn-auto-select-dlc", "Add DLC");
    ui.declare("btn-latest-manifests", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-select-all", &["btn", "btn--small"]);
    ui.declare("btn-deselect-all", &["btn", "btn--small", "btn--outline"]);
    ui.declare("latest-manifests-note", &["dd-path__hint", "latest-manifests-note", "hidden"]);
    ui.declare("auto-select-note", &["auto-select-note", "hidden"]);
    ui.declare("auto-select-title", &[]);
    ui.declare("auto-select-detail", &[]);
    ui.declare("btn-auto-start-cancel", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("depotSearch", &[]);
    ui.declare_input("depotSearch", "text", "");
    ui.declare("showSelectedOnly", &[]);
    ui.declare_input("showSelectedOnly", "checkbox", "");
    ui.declare("steam-keys-offer", &["steam-keys-offer", "hidden"]);
    ui.declare("depot-list", &["depot-list"]);
    ui.declare("branch-section", &["settings-section", "branch-section", "hidden"]);
    ui.declare("branch-label", &["settings-section__label"]);
    ui.declare("branch-row", &["branch-section__row"]);
    ui.declare("branch-picker", &["history-sort", "branch-picker"]);
    ui.declare("branch-button", &["history-sort__button", "branch-picker__button"]);
    ui.set_attr("branch-button", "aria-expanded", "false");
    ui.declare("branch-button-label", &["history-sort__label", "branch-picker__label"]);
    ui.set_text("branch-button-label", "public");
    ui.declare("branch-menu", &["history-sort__menu", "history-sort__menu--scroll", "branch-picker__menu", "hidden"]);
    ui.declare("branch-password", &["dd-path__input", "branch-section__password", "hidden"]);
    ui.declare_input("branch-password", "password", "");
    ui.declare("branch-login-row", &["branch-section__login", "hidden"]);
    ui.declare("branch-more", &["dd-path__hint"]);
    ui.declare("btn-branch-login", &["btn", "btn--small", "btn--outline"]);
    ui.declare("use-account-row", &["settings-field__toggle-wrap", "branch-section__account", "hidden"]);
    ui.declare("use-account-toggle", &[]);
    ui.declare_input("use-account-toggle", "checkbox", "");
    ui.declare("branch-hint", &["dd-path__hint"]);
    ui.declare("download-dir", &["dd-path__input"]);
    ui.declare_input("download-dir", "text", "");
    ui.declare("btn-browse-dir", &["btn", "btn--small", "btn--outline", "dd-path__browse-btn"]);
    ui.declare("mh-apikey", &["dd-path__input"]);
    ui.declare_input("mh-apikey", "text", "");
    ui.declare("update-mode-notice", &["update-notice", "hidden"]);
    ui.declare("btn-back", &["btn", "btn--outline"]);
    ui.declare("btn-queue-add", &["btn", "btn--outline"]);
    ui.set_disabled("btn-queue-add", true);
    ui.declare("btn-download", &["btn", "btn--primary"]);
    ui.set_disabled("btn-download", true);
    ui.declare("step-progress", &["step", "hidden"]);
    ui.declare("step-progress", &["step", "hidden"]);
    ui.declare("native-alpha-banner", &["native-alpha-banner", "hidden"]);
    ui.declare("disk-space-info", &["disk-space-info", "hidden"]);
    ui.declare("disk-space-text", &[]);
    ui.declare("progress-header", &["progress-header"]);
    ui.declare("progress-bar-fill", &["progress-bar__fill"]);
    ui.declare("progress-status", &["progress-status"]);
    ui.declare("depot-progress-list", &["depot-progress-list"]);
    ui.declare("depot-progress-fill", &["depot-progress-fill"]);
    ui.declare("depot-progress-text", &["depot-progress-text"]);
    ui.set_text("depot-progress-text", "0%");
    ui.declare("download-speed-info", &["download-speed-info", "hidden"]);
    ui.declare("download-speed", &["download-speed-info__speed"]);
    ui.declare("download-eta", &["download-speed-info__eta"]);
    ui.declare("shutdown-after-toggle", &[]);
    ui.declare_input("shutdown-after-toggle", "checkbox", "");
    ui.declare("terminal", &["terminal"]);
    ui.declare("terminal-output", &["terminal__body"]);
    ui.declare("completion-message", &["completion-message", "hidden"]);
    ui.declare("failure-advice", &["next-steps", "hidden"]);
    ui.declare("feedback-prompt", &["feedback-card", "hidden"]);
    ui.declare("btn-cancel", &["btn", "btn--danger"]);
    ui.declare("btn-pause", &["btn", "btn--outline"]);
    ui.declare("btn-report-failure", &["btn", "btn--outline", "hidden"]);
    ui.declare("btn-next-step", &["btn", "btn--primary", "hidden"]);
    ui.declare("step-shortcut", &["step", "hidden"]);
    ui.declare("step-shortcut", &["step", "hidden"]);
    ui.declare("shortcut-exe-path", &["dd-path__input"]);
    ui.declare_input("shortcut-exe-path", "text", "");
    ui.declare("btn-browse-exe", &["btn", "btn--small", "btn--outline", "dd-path__browse-btn"]);
    ui.declare("shortcut-detected-section", &["shortcut-detected-section", "hidden"]);
    ui.declare("btn-toggle-detected", &["settings-advanced__toggle"]);
    ui.set_text("shortcut-detected-arrow", "▶");
    ui.declare("shortcut-detected-list", &["shortcut-detected-list", "hidden"]);
    ui.declare("shortcut-desktop", &[]);
    ui.declare_input("shortcut-desktop", "checkbox", "");
    ui.set_checked("shortcut-desktop", true);
    ui.declare("shortcut-startmenu", &[]);
    ui.declare_input("shortcut-startmenu", "checkbox", "");
    ui.set_checked("shortcut-startmenu", true);
    ui.declare("shortcut-steam-row", &["shortcut-option", "shortcut-option--full"]);
    ui.declare("btn-shortcut-steam-choose", &["btn", "btn--small", "btn--outline", "shortcut-option__action", "hidden"]);
    ui.declare("shortcut-steam-library", &[]);
    ui.declare_input("shortcut-steam-library", "checkbox", "");
    ui.declare("shortcut-status", &["completion-message", "hidden"]);
    ui.declare("btn-shortcut-start-over", &["btn", "btn--outline"]);
    ui.declare("btn-shortcut-skip", &["btn", "btn--outline"]);
    ui.declare("btn-create-shortcuts", &["btn", "btn--primary"]);
    ui.declare("step-steam-library", &["step", "hidden"]);
    ui.declare("step-steam-library", &["step", "hidden"]);
    ui.declare("steam-library-status", &["emu-release-status"]);
    ui.declare("steam-exe-path", &["dd-path__input"]);
    ui.declare_input("steam-exe-path", "text", "");
    ui.declare("btn-steam-browse-exe", &["btn", "btn--small", "btn--outline", "dd-path__browse-btn"]);
    ui.declare("steam-detected-section", &["shortcut-detected-section", "hidden"]);
    ui.declare("btn-steam-toggle-detected", &["settings-advanced__toggle"]);
    ui.set_text("steam-detected-arrow", "▶");
    ui.declare("steam-detected-list", &["shortcut-detected-list", "hidden"]);
    ui.declare("steam-game-name", &["dd-path__input"]);
    ui.declare_input("steam-game-name", "text", "");
    ui.declare("steam-launch-options", &["dd-path__input"]);
    ui.declare_input("steam-launch-options", "text", "");
    ui.declare("steam-library-result", &["completion-message", "hidden"]);
    ui.declare("btn-steam-skip", &["btn", "btn--outline"]);
    ui.declare("btn-steam-add", &["btn", "btn--primary"]);
    ui.declare("step-emulator", &["step", "hidden"]);
    ui.declare("step-emulator", &["step", "hidden"]);
    ui.declare("emu-release-status", &["emu-release-status"]);
    ui.declare("emu-summary", &["emu-summary"]);
    ui.declare("emu-tab-files", &["emu-nav__tab", "is-active"]);
    ui.declare("emu-badge-files", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-gamedata", &["emu-nav__tab"]);
    ui.declare("emu-badge-gamedata", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-controller", &["emu-nav__tab"]);
    ui.declare("emu-badge-controller", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-emulator", &["emu-nav__tab"]);
    ui.declare("emu-badge-emulator", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-saves", &["emu-nav__tab"]);
    ui.declare("emu-badge-saves", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-profile", &["emu-nav__tab"]);
    ui.declare("emu-badge-profile", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-network", &["emu-nav__tab"]);
    ui.declare("emu-badge-network", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-mods", &["emu-nav__tab"]);
    ui.declare("emu-badge-mods", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-tab-extras", &["emu-nav__tab"]);
    ui.declare("emu-badge-extras", &["emu-nav__badge", "hidden"]);
    ui.declare("emu-panel-files", &["emu-panel", "is-active"]);
    ui.declare("emu-panel-files", &["emu-panel", "is-active"]);
    ui.declare("emu-denuvo-note", &["denuvo-note", "hidden"]);
    ui.declare("emu-drm-section", &["emu-drm-section", "hidden"]);
    ui.declare("emu-drm-list", &["emu-drm-list"]);
    ui.declare("emu-drm-status-wrap", &["emu-drm-status-wrap", "hidden"]);
    ui.declare("btn-emu-drm-copy", &["emu-drm-copy-btn"]);
    ui.declare("emu-drm-status", &["emu-drm-status"]);
    ui.declare("btn-emu-drm-remove", &["btn", "btn--danger"]);
    ui.declare("emu-patch-version", &["emu-patch-version", "hidden"]);
    ui.declare("emu-patch-version-text", &[]);
    ui.declare("btn-emu-update-gbe", &["btn", "btn--small", "btn--primary", "hidden"]);
    ui.declare("emu-file-list", &["emu-file-list"]);
    ui.declare("emu-drm-restore", &["emu-drm-restore", "hidden"]);
    ui.declare("emu-drm-restore-list", &["emu-drm-restore__list"]);
    ui.declare("emu-file-empty", &["emu-file-empty", "hidden"]);
    ui.declare("emu-already-patched", &["emu-already-patched", "hidden"]);
    ui.declare("emu-method-section", &["emu-section", "hidden"]);
    ui.declare("emu-method-replace", &[]);
    ui.declare_input("emu-method-replace", "radio", "emu-method");
    ui.set_checked("emu-method-replace", true);
    ui.declare("emu-method-coldclient", &[]);
    ui.declare_input("emu-method-coldclient", "radio", "emu-method");
    ui.declare("emu-cold-options", &["emu-gamedata__key", "hidden"]);
    ui.declare("emu-cold-exe", &["emu-field__input", "emu-field__select"]);
    ui.declare("emu-cold-steam", &["emu-cold-steam", "hidden"]);
    ui.declare("btn-emu-cold-steam", &["btn", "btn--small", "btn--primary"]);
    ui.declare("emu-variant-section", &["emu-section"]);
    ui.declare("emu-variant-regular", &[]);
    ui.declare_input("emu-variant-regular", "radio", "emu-variant");
    ui.set_checked("emu-variant-regular", true);
    ui.declare("emu-variant-experimental", &[]);
    ui.declare_input("emu-variant-experimental", "radio", "emu-variant");
    ui.declare("emu-panel-gamedata", &["emu-panel"]);
    ui.declare("emu-panel-gamedata", &["emu-panel"]);
    ui.declare("emu-gamedata-section", &["emu-section", "emu-gamedata-section"]);
    ui.declare("emu-gamedata-toggle", &[]);
    ui.declare_input("emu-gamedata-toggle", "checkbox", "");
    ui.set_checked("emu-gamedata-toggle", true);
    ui.declare("emu-webapi-key", &["dd-path__input"]);
    ui.declare_input("emu-webapi-key", "password", "");
    ui.declare("btn-emu-webapi-save", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-gamedata-media-label", &["emu-gamedata__key-label"]);
    ui.declare("emu-gamedata-media", &["segmented"]);
    ui.declare("emu-gamedata-media-off", &["segmented__option", "is-active"]);
    ui.declare("emu-gamedata-media-images", &["segmented__option"]);
    ui.declare("emu-gamedata-media-all", &["segmented__option"]);
    ui.declare("btn-emu-gamedata", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-gamedata-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-controller", &["emu-panel"]);
    ui.declare("emu-panel-controller", &["emu-panel"]);
    ui.declare("emu-controller-section", &["emu-section", "emu-gamedata-section"]);
    ui.declare("emu-controller-toggle", &[]);
    ui.declare_input("emu-controller-toggle", "checkbox", "");
    ui.set_checked("emu-controller-toggle", true);
    ui.declare("emu-controller-status", &["dd-path__hint"]);
    ui.declare("emu-controller-build", &["dd-path__hint", "hidden"]);
    ui.declare("emu-controller-file", &["dd-path__input"]);
    ui.declare_input("emu-controller-file", "text", "");
    ui.declare("btn-emu-controller-file", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-controller-clear", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("emu-controller-type", &["emu-field__input", "emu-field__select"]);
    ui.declare("emu-set-disable_steamoverlaygameid_env_var", &[]);
    ui.declare_input("emu-set-disable_steamoverlaygameid_env_var", "checkbox", "");
    ui.declare("emu-controller-result", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-emulator", &["emu-panel"]);
    ui.declare("emu-panel-emulator", &["emu-panel"]);
    ui.declare("emu-settings-hint", &["emu-settings-hint"]);
    ui.declare("emu-settings-search", &["emu-field__input", "emu-settings__search"]);
    ui.declare_input("emu-settings-search", "search", "");
    ui.declare("emu-settings-changed", &[]);
    ui.declare_input("emu-settings-changed", "checkbox", "");
    ui.declare("btn-emu-settings-reset", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-settings-groups", &["emu-settings__groups"]);
    ui.declare("emu-settings-fields", &["emu-settings__fields"]);
    ui.declare("emu-settings-empty", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-saves", &["emu-panel"]);
    ui.declare("emu-panel-saves", &["emu-panel"]);
    ui.declare("emu-needs-saves", &["emu-extra__needs", "hidden"]);
    ui.declare("emu-saves-list", &["emu-extra-list"]);
    ui.declare("btn-emu-saves-open", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-saves-backup", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-saves-restore", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-saves-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-ach-count", &["dd-path__hint"]);
    ui.declare("emu-ach-list", &["emu-ach-list"]);
    ui.declare("emu-ach-empty", &["dd-path__hint", "hidden"]);
    ui.declare("btn-emu-ach-reset", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-stats-reset", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-ach-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-profile", &["emu-panel"]);
    ui.declare("emu-panel-profile", &["emu-panel"]);
    ui.declare("emu-needs-profile", &["emu-extra__needs", "hidden"]);
    ui.declare("emu-avatar-preview", &["emu-avatar__img"]);
    ui.declare("btn-emu-avatar", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-avatar-clear", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("emu-font-system", &[]);
    ui.declare("emu-asset-clear-font", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("emu-asset-clear-achievement", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("emu-asset-clear-friend", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("emu-profile-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-network", &["emu-panel"]);
    ui.declare("emu-panel-network", &["emu-panel"]);
    ui.declare("emu-needs-network", &["emu-extra__needs", "hidden"]);
    ui.declare("emu-broadcasts", &["emu-field__input", "emu-broadcasts"]);
    ui.declare("btn-emu-broadcasts", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-broadcasts-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-lobby-later", &["dd-path__hint", "hidden"]);
    ui.declare("emu-lobby-section", &["emu-section", "emu-gamedata-section"]);
    ui.declare("btn-emu-lobby", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-lobby-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-mods", &["emu-panel"]);
    ui.declare("emu-panel-mods", &["emu-panel"]);
    ui.declare("emu-needs-mods", &["emu-extra__needs", "hidden"]);
    ui.declare("emu-mods-list", &["emu-extra-list"]);
    ui.declare("emu-mods-empty", &["dd-path__hint"]);
    ui.declare("emu-mod-id", &["dd-path__input"]);
    ui.declare_input("emu-mod-id", "text", "");
    ui.declare("btn-emu-mod-folder", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-emu-mod-file", &["btn", "btn--small", "btn--outline"]);
    ui.declare("emu-mods-status", &["dd-path__hint", "hidden"]);
    ui.declare("emu-panel-extras", &["emu-panel"]);
    ui.declare("emu-panel-extras", &["emu-panel"]);
    ui.declare("emu-bypass-section", &["emu-section", "emu-bypass-section"]);
    ui.declare("emu-bypass-toggle", &[]);
    ui.declare_input("emu-bypass-toggle", "checkbox", "");
    ui.declare("emu-dlc-merge-section", &["emu-section", "emu-dlc-merge-section", "hidden"]);
    ui.declare("emu-dlc-merge-hint", &["emu-bypass-row__hint"]);
    ui.declare("emu-dlc-merge-status", &["dd-path__hint", "hidden"]);
    ui.declare("btn-emu-merge-dlcs", &["btn", "btn--primary", "btn--small"]);
    ui.declare("emu-apply-status", &["completion-message", "hidden"]);
    ui.declare("btn-emu-start-over", &["btn", "btn--outline"]);
    ui.declare("btn-emu-new", &["btn", "btn--outline"]);
    ui.declare("btn-emu-revert", &["btn", "btn--danger", "hidden"]);
    ui.declare("btn-emu-save-settings", &["btn", "btn--outline", "hidden"]);
    ui.declare("btn-emu-apply", &["btn", "btn--primary"]);
    ui.declare("cancel-modal", &["modal", "hidden"]);
    ui.declare("cancel-modal", &["modal", "hidden"]);
    ui.declare("cancel-modal-title", &["modal__title"]);
    ui.declare("cancel-modal-body", &["modal__text"]);
    ui.declare("btn-cancel-no", &["btn", "btn--outline"]);
    ui.declare("btn-cancel-yes", &["btn", "btn--danger"]);
    ui.declare("shutdown-modal", &["modal", "hidden"]);
    ui.declare("shutdown-modal", &["modal", "hidden"]);
    ui.declare("shutdown-modal-title", &["modal__title"]);
    ui.declare("shutdown-modal-body", &["modal__text"]);
    ui.declare("btn-shutdown-abort", &["btn", "btn--outline"]);
    ui.declare("btn-shutdown-now", &["btn", "btn--danger"]);
    ui.declare("followup-modal", &["modal", "hidden"]);
    ui.declare("followup-modal", &["modal", "hidden"]);
    ui.declare("followup-modal-title", &["modal__title"]);
    ui.declare("followup-modal-body", &["modal__text"]);
    ui.declare("btn-followup-later", &["btn", "btn--outline"]);
    ui.declare("btn-followup-discard", &["btn", "btn--outline"]);
    ui.declare("btn-followup-resume", &["btn", "btn--primary"]);
    ui.declare("close-modal", &["modal", "hidden"]);
    ui.declare("close-modal", &["modal", "hidden"]);
    ui.declare("close-modal-title", &["modal__title"]);
    ui.declare("btn-close-no", &["btn", "btn--outline"]);
    ui.declare("btn-close-yes", &["btn", "btn--danger"]);
    ui.declare("remove-source-modal", &["modal", "hidden"]);
    ui.declare("remove-source-modal", &["modal", "hidden"]);
    ui.declare("remove-source-modal-title", &["modal__title"]);
    ui.declare("btn-remove-source-no", &["btn", "btn--outline"]);
    ui.declare("btn-remove-source-yes", &["btn", "btn--danger"]);
    ui.declare("history-remove-modal", &["modal", "hidden"]);
    ui.declare("history-remove-modal", &["modal", "hidden"]);
    ui.declare("history-remove-modal-title", &["modal__title"]);
    ui.declare("btn-history-remove-no", &["btn", "btn--outline"]);
    ui.declare("btn-history-remove-yes", &["btn", "btn--danger"]);
    ui.declare("emu-revert-modal", &["modal", "hidden"]);
    ui.declare("emu-revert-modal", &["modal", "hidden"]);
    ui.declare("emu-revert-modal-title", &["modal__title"]);
    ui.declare("emu-revert-scope", &["modal__text"]);
    ui.declare("btn-emu-revert-no", &["btn", "btn--outline"]);
    ui.declare("btn-emu-revert-yes", &["btn", "btn--danger"]);
    ui.declare("emu-download-modal", &["modal", "hidden"]);
    ui.declare("emu-download-modal", &["modal", "hidden"]);
    ui.declare("emu-download-modal-title", &["modal__title"]);
    ui.declare("emu-download-scope", &["modal__text"]);
    ui.declare("btn-emu-download-no", &["btn", "btn--outline"]);
    ui.declare("btn-emu-download-yes", &["btn", "btn--primary"]);
    ui.declare("history-clear-modal", &["modal", "hidden"]);
    ui.declare("history-clear-modal", &["modal", "hidden"]);
    ui.declare("history-clear-modal-title", &["modal__title"]);
    ui.declare("history-clear-resumable-warning", &["modal__notice", "hidden"]);
    ui.declare("history-clear-resumable-text", &["modal__text"]);
    ui.declare("history-clear-delete-files", &[]);
    ui.declare_input("history-clear-delete-files", "checkbox", "");
    ui.declare("btn-history-clear-no", &["btn", "btn--outline"]);
    ui.declare("btn-history-clear-yes", &["btn", "btn--danger"]);
    ui.declare("telemetry-modal", &["modal", "hidden"]);
    ui.declare("telemetry-modal", &["modal", "hidden"]);
    ui.declare("telemetry-modal-title", &["modal__title"]);
    ui.declare("btn-telemetry-decline", &["btn", "btn--outline"]);
    ui.declare("btn-telemetry-basic", &["btn", "btn--outline"]);
    ui.declare("btn-telemetry-accept", &["btn", "btn--primary"]);
    ui.declare("behavior-modal", &["modal", "hidden"]);
    ui.declare("behavior-modal", &["modal", "hidden"]);
    ui.declare("behavior-modal-title", &["modal__title"]);
    ui.declare("btn-behavior-decline", &["btn", "btn--outline"]);
    ui.declare("btn-behavior-accept", &["btn", "btn--primary"]);
    ui.declare("bug-report-modal", &["modal", "hidden"]);
    ui.declare("bug-report-modal", &["modal", "hidden"]);
    ui.declare("bug-report-title", &["modal__title"]);
    ui.declare("bug-title", &["dd-path__input", "bug-report__input"]);
    ui.declare_input("bug-title", "text", "");
    ui.declare("bug-what", &["dd-path__input", "bug-report__input"]);
    ui.declare("bug-steps", &["dd-path__input", "bug-report__input"]);
    ui.declare("bug-expected", &["dd-path__input", "bug-report__input"]);
    ui.declare("settings-group-1", &["settings-group"]);
    ui.declare("settings-row-1", &["settings-row"]);
    ui.declare("bug-diag-preview", &["settings-row__hint"]);
    ui.declare("bug-include-diag", &[]);
    ui.declare_input("bug-include-diag", "checkbox", "");
    ui.set_checked("bug-include-diag", true);
    ui.declare("settings-row-2", &["settings-row"]);
    ui.declare("bug-include-log", &[]);
    ui.declare_input("bug-include-log", "checkbox", "");
    ui.declare("bug-report-error", &["error-message", "hidden"]);
    ui.declare("btn-bug-cancel", &["btn", "btn--outline"]);
    ui.declare("btn-bug-open", &["btn", "btn--primary"]);
    ui.declare("settings-modal", &["modal", "hidden"]);
    ui.declare("settings-modal", &["modal", "hidden"]);
    ui.declare("settings-modal-title", &["modal__title", "settings__title"]);
    ui.declare("settings-search", &[]);
    ui.declare_input("settings-search", "search", "");
    ui.declare("settings-tab-general", &["settings__tab", "is-active"]);
    ui.declare("settings-tab-downloads", &["settings__tab"]);
    ui.declare("settings-tab-depots", &["settings__tab"]);
    ui.declare("settings-tab-sources", &["settings__tab"]);
    ui.declare("settings-tab-steam", &["settings__tab"]);
    ui.declare("settings-tab-privacy", &["settings__tab"]);
    ui.declare("settings-no-results", &["settings__empty", "hidden"]);
    ui.declare("settings-panel-general", &["settings__panel", "is-active"]);
    ui.declare("settings-group-2", &["settings-group"]);
    ui.declare("settings-row-3", &["settings-row", "settings-row--stack"]);
    ui.declare("settings-language-cards", &["language-cards"]);
    ui.declare("settings-row-4", &["settings-row"]);
    ui.declare("auto-update-toggle", &[]);
    ui.declare_input("auto-update-toggle", "checkbox", "");
    ui.set_checked("auto-update-toggle", true);
    ui.declare("settings-row-5", &["settings-row"]);
    ui.declare("notification-sound-toggle", &[]);
    ui.declare_input("notification-sound-toggle", "checkbox", "");
    ui.set_checked("notification-sound-toggle", true);
    ui.declare("settings-panel-downloads", &["settings__panel"]);
    ui.declare("settings-group-3", &["settings-group"]);
    ui.declare("settings-row-6", &["settings-row"]);
    ui.declare("native-downloader-hint", &["settings-row__hint"]);
    ui.declare("native-downloader-toggle", &[]);
    ui.declare_input("native-downloader-toggle", "checkbox", "");
    ui.set_checked("native-downloader-toggle", true);
    ui.declare("settings-row-7", &["settings-row", "settings-row--wrap"]);
    ui.declare("speed-limit-toggle", &[]);
    ui.declare_input("speed-limit-toggle", "checkbox", "");
    ui.declare("speed-limit-controls", &["speed-limit", "settings-row__extra", "hidden"]);
    ui.declare("speed-limit-input", &["speed-limit__input"]);
    ui.declare_input("speed-limit-input", "number", "");
    ui.declare("speed-unit-mb", &["speed-limit__unit", "is-active"]);
    ui.declare("speed-unit-mbit", &["speed-limit__unit"]);
    ui.declare("settings-row-8", &["settings-row", "settings-row--wrap"]);
    ui.declare("folder-style-options", &["segmented"]);
    ui.declare("folder-style-options-", &["segmented__option", "is-active"]);
    ui.declare("folder-style-options-underscore", &["segmented__option"]);
    ui.declare("folder-style-options-dash", &["segmented__option"]);
    ui.declare("settings-row-9", &["settings-row"]);
    ui.declare("cancel-keep-files-toggle", &[]);
    ui.declare_input("cancel-keep-files-toggle", "checkbox", "");
    ui.declare("settings-title-1", &["settings__group-title"]);
    ui.declare("settings-group-4", &["settings-group"]);
    ui.declare("settings-row-10", &["settings-row"]);
    ui.declare("max-retries-input", &["dd-path__input", "settings-row__number"]);
    ui.declare_input("max-retries-input", "number", "");
    ui.set_value("max-retries-input", "3");
    ui.declare("settings-row-11", &["settings-row"]);
    ui.declare("chunk-concurrency-input", &["dd-path__input", "settings-row__number"]);
    ui.declare_input("chunk-concurrency-input", "number", "");
    ui.set_value("chunk-concurrency-input", "16");
    ui.declare("dd-extra-args-row", &["settings-row", "settings-row--stack"]);
    ui.declare("dd-extra-args-input", &["dd-path__input"]);
    ui.declare_input("dd-extra-args-input", "text", "");
    ui.declare("settings-panel-depots", &["settings__panel"]);
    ui.declare("settings-group-5", &["settings-group"]);
    ui.declare("settings-row-12", &["settings-row"]);
    ui.declare("auto-select-depots-toggle", &[]);
    ui.declare_input("auto-select-depots-toggle", "checkbox", "");
    ui.declare("settings-row-13", &["settings-row"]);
    ui.declare("latest-manifests-toggle", &[]);
    ui.declare_input("latest-manifests-toggle", "checkbox", "");
    ui.declare("auto-select-options", &["auto-select-options"]);
    ui.declare("settings-title-2", &["settings__group-title"]);
    ui.declare("settings-group-6", &["settings-group"]);
    ui.declare("settings-row-14", &["settings-row"]);
    ui.declare("auto-start-download-toggle", &[]);
    ui.declare_input("auto-start-download-toggle", "checkbox", "");
    ui.declare("settings-row-15", &["settings-row"]);
    ui.declare("include-dlc-toggle", &[]);
    ui.declare_input("include-dlc-toggle", "checkbox", "");
    ui.set_checked("include-dlc-toggle", true);
    ui.declare("settings-row-16", &["settings-row", "settings-row--wrap"]);
    ui.declare("target-platform-options", &["segmented"]);
    ui.declare("target-platform-options-", &["segmented__option", "is-active"]);
    ui.declare("target-platform-options-windows", &["segmented__option"]);
    ui.declare("target-platform-options-linux", &["segmented__option"]);
    ui.declare("target-platform-options-macos", &["segmented__option"]);
    ui.declare("settings-row-17", &["settings-row"]);
    ui.declare("game-language", &["history-sort"]);
    ui.declare("game-language-button", &["history-sort__button"]);
    ui.set_attr("game-language-button", "aria-expanded", "false");
    ui.declare("game-language-label", &["history-sort__label"]);
    ui.declare("game-language-menu", &["history-sort__menu", "history-sort__menu--scroll", "hidden"]);
    ui.declare("settings-panel-sources", &["settings__panel"]);
    ui.declare("settings-group-7", &["settings-group"]);
    ui.declare("settings-row-18", &["settings-row", "settings-row--stack"]);
    ui.declare("sources-list", &["sources-list"]);
    ui.declare("sources-add-input", &["dd-path__input"]);
    ui.declare_input("sources-add-input", "text", "");
    ui.declare("btn-sources-add", &["btn", "btn--outline"]);
    ui.declare("sources-add-error", &["error-message", "hidden"]);
    ui.declare("settings-row-19", &["settings-row", "settings-row--stack"]);
    ui.declare("hubcap-apikey-input", &["dd-path__input"]);
    ui.declare_input("hubcap-apikey-input", "text", "");
    ui.declare("settings-row-20", &["settings-row", "settings-row--stack"]);
    ui.declare("ryuu-apikey-input", &["dd-path__input"]);
    ui.declare_input("ryuu-apikey-input", "text", "");
    ui.declare("settings-title-3", &["settings__group-title"]);
    ui.declare("settings-group-8", &["settings-group"]);
    ui.declare("settings-row-21", &["settings-row", "settings-row--stack"]);
    ui.declare("proxy-input", &["dd-path__input"]);
    ui.declare_input("proxy-input", "text", "");
    ui.declare("btn-proxy-test", &["btn", "btn--small", "btn--outline"]);
    ui.declare("proxy-status", &["proxy-status", "hidden"]);
    ui.declare("proxy-error", &["error-message", "hidden"]);
    ui.declare("settings-row-22", &["settings-row", "settings-row--stack"]);
    ui.declare("download-server-select", &[]);
    ui.declare("download-server-custom", &["hidden"]);
    ui.declare("download-server-input", &["dd-path__input"]);
    ui.declare_input("download-server-input", "text", "");
    ui.declare("download-server-error", &["error-message", "hidden"]);
    ui.declare("settings-panel-steam", &["settings__panel"]);
    ui.declare("settings-group-9", &["settings-group"]);
    ui.declare("settings-row-23", &["settings-row", "settings-row--stack"]);
    ui.declare("steam-webapi-key-input", &["dd-path__input"]);
    ui.declare_input("steam-webapi-key-input", "password", "");
    ui.declare("settings-row-24", &["settings-row", "settings-row--stack"]);
    ui.declare("steam-path-input", &["dd-path__input"]);
    ui.declare_input("steam-path-input", "text", "");
    ui.declare("btn-steam-path-browse", &["btn", "btn--small", "btn--outline"]);
    ui.declare("steam-path-status", &["proxy-status", "hidden"]);
    ui.declare("settings-title-4", &["settings__group-title"]);
    ui.declare("account-group", &["settings-group"]);
    ui.declare("settings-row-25", &["settings-row"]);
    ui.declare("account-status", &["settings-row__label"]);
    ui.declare("account-detail", &["settings-row__hint"]);
    ui.declare("btn-account-login", &["btn", "btn--small", "btn--primary"]);
    ui.declare("btn-account-logout", &["btn", "btn--small", "btn--outline", "hidden"]);
    ui.declare("settings-panel-privacy", &["settings__panel"]);
    ui.declare("settings-group-10", &["settings-group"]);
    ui.declare("settings-row-26", &["settings-row"]);
    ui.declare("telemetry-toggle", &[]);
    ui.declare_input("telemetry-toggle", "checkbox", "");
    ui.declare("settings-row-27", &["settings-row"]);
    ui.declare("remote-updates-toggle", &[]);
    ui.declare_input("remote-updates-toggle", "checkbox", "");
    ui.set_checked("remote-updates-toggle", true);
    ui.declare("behavior-row", &["settings-row"]);
    ui.declare("behavior-toggle", &[]);
    ui.declare_input("behavior-toggle", "checkbox", "");
    ui.declare("settings-title-5", &["settings__group-title"]);
    ui.declare("settings-group-11", &["settings-group"]);
    ui.declare("settings-row-28", &["settings-row", "settings-row--stack"]);
    ui.declare("build-info", &["build-info"]);
    ui.declare("build-info-channel", &["build-info__badge"]);
    ui.set_text("build-info-channel", "—");
    ui.declare("build-info-version", &["build-info__val"]);
    ui.set_text("build-info-version", "—");
    ui.declare("build-info-sha", &["build-info__val"]);
    ui.set_text("build-info-sha", "—");
    ui.declare("build-info-date", &["build-info__val"]);
    ui.set_text("build-info-date", "—");
    ui.declare("build-info-profile", &["build-info__val"]);
    ui.set_text("build-info-profile", "—");
    ui.declare("build-info-platform", &["build-info__val"]);
    ui.set_text("build-info-platform", "—");
    ui.declare("build-info-diagnostic", &["build-info__val"]);
    ui.set_text("build-info-diagnostic", "—");
    ui.declare("btn-copy-build-info", &["btn", "btn--outline", "btn--small"]);
    ui.declare("btn-settings-report-bug", &["btn", "btn--outline", "btn--small"]);
    ui.declare("settings-dirty", &["settings__dirty", "hidden"]);
    ui.declare("btn-settings-cancel", &["btn", "btn--outline"]);
    ui.declare("btn-settings-save", &["btn", "btn--primary"]);
    ui.declare("whats-new-modal", &["modal", "hidden"]);
    ui.declare("whats-new-modal", &["modal", "hidden"]);
    ui.declare("whats-new-title", &["update-modal__title"]);
    ui.declare("whats-new-date", &["update-modal__date"]);
    ui.declare("whats-new-notes", &["update-modal__notes"]);
    ui.declare("btn-whats-new-github", &["btn", "btn--small", "update-modal__skip"]);
    ui.declare("btn-whats-new-close", &["btn", "btn--primary"]);
    ui.declare("update-modal", &["modal", "hidden"]);
    ui.declare("update-modal", &["modal", "hidden"]);
    ui.declare("update-modal-title", &["update-modal__title"]);
    ui.declare("update-current", &["update-modal__current"]);
    ui.declare("update-version", &["update-modal__version"]);
    ui.declare("update-date-row", &["update-modal__date"]);
    ui.declare("update-date", &[]);
    ui.declare("update-notes", &["update-modal__notes"]);
    ui.declare("update-system-hint", &["update-modal__system-hint", "hidden"]);
    ui.declare("update-system-text", &[]);
    ui.declare("update-system-cmd-list", &["update-system-cmd-list"]);
    ui.declare("update-progress-wrap", &["update-modal__progress", "hidden"]);
    ui.declare("update-progress-fill", &["progress-bar__fill"]);
    ui.declare("update-progress-text", &["update-modal__progress-text"]);
    ui.declare("update-actions", &["update-modal__actions"]);
    ui.declare("btn-update-skip", &["btn", "btn--small", "update-modal__skip"]);
    ui.declare("btn-update-notes-link", &["btn", "btn--small", "update-modal__skip", "hidden"]);
    ui.declare("btn-update-later", &["btn", "btn--outline"]);
    ui.declare("btn-update-now", &["btn", "btn--primary"]);
    ui.declare("language-picker", &["modal", "hidden"]);
    ui.declare("language-picker", &["modal", "hidden"]);
    ui.declare("language-picker-title", &["modal__title"]);
    ui.declare("language-picker-cards", &["language-cards"]);
    ui.declare("welcome-modal", &["modal", "hidden"]);
    ui.declare("welcome-modal", &["modal", "hidden"]);
    ui.declare("welcome-title", &["modal__title"]);
    ui.declare("welcome-location-path", &["welcome__location-path"]);
    ui.declare("btn-welcome-location", &["btn", "btn--small", "btn--outline"]);
    ui.declare("btn-welcome-close", &["btn", "btn--small"]);
    ui.declare("history-modal", &["modal", "hidden"]);
    ui.declare("history-modal", &["modal", "hidden"]);
    ui.declare("history-modal-title", &["modal__title"]);
    ui.declare("history-banner", &["history-banner", "hidden"]);
    ui.declare("history-toolbar", &["history-toolbar", "hidden"]);
    ui.declare("history-search", &["history-toolbar__search"]);
    ui.declare_input("history-search", "search", "");
    ui.declare("history-sort", &["history-sort"]);
    ui.declare("history-sort-button", &["history-sort__button"]);
    ui.set_attr("history-sort-button", "aria-expanded", "false");
    ui.declare("history-sort-label", &["history-sort__label"]);
    ui.declare("history-sort-menu", &["history-sort__menu", "hidden"]);
    ui.declare("history-sort-newest", &["history-sort__option"]);
    ui.declare("history-sort-oldest", &["history-sort__option"]);
    ui.declare("history-sort-name", &["history-sort__option"]);
    ui.declare("history-sort-size", &["history-sort__option"]);
    ui.declare("history-filters", &["history-toolbar__filters"]);
    ui.declare("history-list", &["history-list"]);
    ui.declare("btn-history-clear", &["btn", "btn--outline", "btn--small", "btn--push-end"]);
    ui.declare("btn-history-close", &["btn", "btn--primary"]);
    ui.declare("resume-modal", &["modal", "hidden"]);
    ui.declare("resume-modal", &["modal", "hidden"]);
    ui.declare("resume-modal-title", &["modal__title"]);
    ui.declare("btn-resume-cancel", &["btn", "btn--outline"]);
    ui.declare("btn-resume-verify", &["btn", "btn--outline"]);
    ui.declare("btn-resume-fast", &["btn", "btn--primary"]);
    ui.declare("launch-modal", &["modal", "hidden"]);
    ui.declare("launch-modal", &["modal", "hidden"]);
    ui.declare("launch-modal-title", &["modal__title"]);
    ui.declare("launch-list", &["launch-list"]);
    ui.declare("btn-launch-cancel", &["btn", "btn--outline"]);
    ui.declare("btn-launch-start", &["btn", "btn--primary"]);
    ui.declare("queue-modal", &["modal", "hidden"]);
    ui.declare("queue-modal", &["modal", "hidden"]);
    ui.declare("queue-modal-title", &["modal__title"]);
    ui.declare("queue-running", &["queue-running", "hidden"]);
    ui.declare("queue-list", &["queue-list"]);
    ui.declare("btn-queue-clear", &["btn", "btn--outline", "btn--small", "btn--push-end"]);
    ui.declare("btn-queue-close", &["btn", "btn--outline"]);
    ui.declare("btn-queue-start", &["btn", "btn--primary"]);
    ui.declare("confirm-modal", &["modal", "hidden"]);
    ui.declare("confirm-modal", &["modal", "hidden"]);
    ui.declare("confirm-modal-title", &["modal__title"]);
    ui.declare("confirm-modal-body", &["modal__text"]);
    ui.declare("btn-confirm-no", &["btn", "btn--outline"]);
    ui.declare("btn-confirm-yes", &["btn", "btn--primary"]);
    ui.declare("app-toast", &["app-toast", "hidden"]);
    ui.declare("account-modal", &["modal", "hidden"]);
    ui.declare("account-modal", &["modal", "hidden"]);
    ui.declare("account-modal-title", &["modal__title"]);
    ui.declare("account-qr", &["account-qr", "account-qr--empty"]);
    ui.declare("account-modal-status", &["account-modal__status"]);
    ui.declare("btn-account-cancel", &["btn", "btn--outline"]);
    ui.declare("btn-account-retry", &["btn", "btn--primary", "hidden"]);
}
