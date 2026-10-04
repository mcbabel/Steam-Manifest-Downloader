use crate::theme::ThemeMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Wizard,
    History,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Source,
    Select,
    Progress,
    Shortcut,
    SteamLibrary,
    Emulator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTab {
    Upload,
    Search,
    PatchOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcTarget {
    Search,
    PatchOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowsePurpose {
    UploadFile,
    PatchDir,
    DownloadDir,
    DepotManifest(usize),
    ShortcutExe,
    SteamExe,
    SettingsDownloadDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputId {
    UploadPath,
    Search,
    SourceAdd,
    PatchDir,
    PatchAppId,
    DepotFilter,
    MhKey,
    DownloadDir,
    DepotManifest(usize),
    ShortcutExe,
    SteamExe,
    SteamName,
    SteamLaunch,
    EmuField(usize),
    Setting(usize),
    SettingsSourceAdd,
    BrowserPath,
    HistoryFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListId {
    Repos,
    Depots,
    Log,
    DepotProgress,
    History,
    SettingsSources,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTarget {
    Page,
    List(ListId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingToggle {
    AutoUpdate,
    NativeDownloader,
    CancelKeepFiles,
    Telemetry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Nav(Page),
    Quit,
    ForceQuit,
    Help,
    CloseModal,
    ListSelect(ListId, usize),
    ListActivate(ListId, usize),
    Scroll(ScrollTarget, i32),
    FocusInput(InputId, u16),

    SourceTab(SourceTab),
    Browse(BrowsePurpose),
    LoadUpload,
    SearchSubmit,
    AutocompletePick(AcTarget, usize),
    SelectRepo(usize),
    SearchNext,
    AddSource,
    PatchStart,

    ToggleDepot(usize),
    SelectAll,
    DeselectAll,
    ToggleShowSelected,
    FetchLatest(usize),
    ClearDepotManifest(usize),
    ToggleMask(InputId),
    StartDownload,
    StartDownloadAnyway,
    BackToSource,

    PauseResume,
    AskCancel,
    CancelConfirmed,
    ProgressNext,
    Home,

    PickExe(usize),
    ToggleShortcutDesktop,
    ToggleShortcutStartMenu,
    ToggleShortcutSteam,
    CreateShortcuts,
    ShortcutNext,
    SteamAdd,
    SteamCloseAndAdd,
    SteamNext,

    EmuToggleFile(usize),
    EmuSelectAllFiles,
    EmuVariant(bool),
    EmuToggleBypass,
    EmuSection(usize),
    EmuToggleField(usize),
    EmuApply,
    EmuDownloadConfirmed,
    EmuDownloadCancelled,
    EmuSave,
    AskEmuRevert,
    EmuRevertConfirmed,
    EmuMergeDlc,
    EmuRemoveDrm,
    EmuDone,

    HistoryResume(usize),
    HistoryResumeConfirmed(usize),
    HistoryRedownload(usize),
    HistoryUpdate(usize),
    HistoryRepair(usize),
    HistoryPlay(usize),
    ToggleShutdownAfter,
    ShutdownAbort,
    ShutdownNow,
    FollowupResume,
    FollowupLater,
    HistoryOpenFolder(usize),
    HistoryCopyPath(usize),
    HistoryEditEmu(usize),
    AskHistoryRemove(usize),
    HistoryRemoveConfirmed,
    AskHistoryClear,
    HistoryClearConfirmed,
    HistoryRefresh,

    SettingsTab(usize),
    SettingsToggle(SettingToggle),
    SettingsLanguage(&'static str),
    SettingsTheme(ThemeMode),
    SettingsAddSource,
    SettingsRemoveSource(usize),
    SettingsRemoveSourceConfirmed(usize),
    SettingsSave,
    SettingsRevert,
    CopyBuildInfo,
    CheckUpdates,

    ModalToggleCheck,
    BrowserUp,
    BrowserHome,
    BrowserToggleHidden,
    BrowserChoose,
    TelemetryAnswer(bool),
    LanguageChosen(&'static str),
    UpdateSkip,
    UpdateOpen,
}
