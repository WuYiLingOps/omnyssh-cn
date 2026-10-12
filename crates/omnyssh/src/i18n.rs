//! TUI localization.
//!
//! Every user-facing string this frontend draws lives here as a [`Key`], looked
//! up through [`t`] (plain) or [`tf`] (parameterized). The table is a `match`
//! over a C-like enum rather than a runtime map: no allocation, no lock, and
//! the compiler catches any key that is added but never translated.
//!
//! Two rules decide what does *not* belong here:
//! * **Keys, not prose.** `Enter`, `Esc`, `Ctrl+N`, `:close` and the `sh` /
//!   `files` action chips are input tokens, identical in every language. They
//!   stay literal at their call sites.
//! * **Proper nouns.** `OmnySSH` and the `CPU` / `RAM` / `Disk` metric
//!   abbreviations are drawn as-is in both languages.

/// Re-export so callers can name the language type without reaching into core.
pub use omnyssh_core::i18n::Lang;

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

/// A translatable string. Names are `area_thing` so a key reads like the place
/// it appears — the shared `Help*` / `Status*` prefixes are deliberate, not the
/// accidental repetition `enum_variant_names` looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Key {
    // -- Global / status bar ------------------------------------------------
    /// "Terminal too small — please resize to at least 80×24."
    TerminalTooSmall,
    /// "Dashboard" (nav chip).
    NavDashboard,
    /// "Files" (nav chip).
    NavFiles,
    /// "Snippets" (nav chip).
    NavSnippets,
    /// "Terminal" (nav chip).
    NavTerminal,
    /// "Help" (nav chip).
    NavHelp,
    /// "Quit" (nav chip).
    NavQuit,
    /// "[SEARCH]".
    SearchBadge,
    /// "type to filter".
    SearchTypeToFilter,
    /// "confirm".
    HintConfirm,
    /// "clear".
    HintClear,
    /// "cancel".
    HintCancel,
    /// "close".
    HintClose,
    /// "save".
    HintSave,
    /// "select".
    HintSelect,
    /// "scroll".
    HintScroll,
    /// "run".
    HintRun,
    /// "connect".
    HintConnect,
    /// "navigate".
    HintNavigate,
    /// "toggle".
    HintToggle,
    /// "next field".
    HintNextField,
    /// "prev field".
    HintPrevField,
    /// "next param".
    HintNextParam,
    /// "confirm delete".
    HintConfirmDelete,
    /// "confirm setup".
    HintConfirmSetup,
    /// "Setting up SSH keys…".
    SettingUpKeys,
    /// "Transferring" — the label before the file name in the transfer hint.
    HintTransferring,

    // -- Host form labels --------------------------------------------------
    /// "Name".
    FormName,
    /// "Hostname / IP".
    FormHostname,
    /// "User".
    FormUser,
    /// "Port".
    FormPort,
    /// "Identity File".
    FormIdentityFile,
    /// "Password (login, optional)".
    FormPassword,
    /// "Tags (comma-sep)".
    FormTags,
    /// "Notes".
    FormNotes,
    /// "Monitoring (ssh | tcp | tcp:PORT)".
    FormMonitoring,
    /// "Port forwards (port:host:hostport, ...)".
    FormForwards,
    /// "Start tunnel on launch (y/n)".
    FormStartTunnel,
    /// "Forward SSH agent (y/n)".
    #[allow(dead_code)]
    FormForwardAgent,
    /// "Forward SSH agent (y/n, not on Windows yet)".
    FormForwardAgentWindows,

    // -- Snippet form labels -----------------------------------------------
    /// "Name".
    SnippetFormName,
    /// "Command".
    SnippetFormCommand,
    /// "Description".
    #[allow(dead_code)]
    SnippetFormDescription,
    /// "Tags (comma-sep)".
    SnippetFormTags,
    /// "Parameters (comma-sep)".
    SnippetFormParams,
    /// "Scope (global | host)".
    SnippetFormScope,

    // -- Sort labels -------------------------------------------------------
    /// "name".
    SortName,
    /// "cpu".
    SortCpu,
    /// "ram".
    SortRam,
    /// "status".
    SortStatus,

    // -- Help overlay ------------------------------------------------------
    /// "Help — Keyboard Shortcuts".
    HelpTitle,
    /// "GLOBAL NAVIGATION".
    HelpSectionGlobalNav,
    /// "DASHBOARD".
    HelpSectionDashboard,
    /// "DETAIL VIEW".
    HelpSectionDetailView,
    /// "FILE MANAGER".
    HelpSectionFileManager,
    /// "SNIPPETS".
    HelpSectionSnippets,
    /// "TERMINAL".
    HelpSectionTerminal,
    /// "COPY TEXT".
    HelpSectionCopyText,
    /// "Press Esc or ? to close".
    HelpFooterClose,
    /// "(Terminal screen only)".
    HelpTerminalOnly,
    /// "Open details".
    HelpOpenDetails,
    /// "Add host".
    HelpAddHost,
    /// "Edit host".
    HelpEditHost,
    /// "Delete host".
    HelpDeleteHost,
    /// "Refresh".
    HelpRefresh,
    /// "Sort".
    HelpSort,
    /// "Filter tags".
    HelpFilterTags,
    /// "Group by tag".
    HelpGroupByTag,
    /// "Search".
    HelpSearch,
    /// "Quick exec".
    HelpQuickExec,
    /// "Setup SSH key".
    HelpSetupKey,
    /// "Tunnel on/off".
    HelpTunnelToggle,
    /// "Navigate".
    HelpNavigate,
    /// "Connect".
    HelpConnect,
    /// "Back".
    HelpBack,
    /// "Quick view".
    HelpQuickView,
    /// "Switch panel".
    HelpSwitchPanel,
    /// "Mark file".
    HelpMarkFile,
    /// "Copy".
    HelpCopy,
    /// "Paste".
    HelpPaste,
    /// "New dir".
    HelpNewDir,
    /// "Rename".
    HelpRename,
    /// "Delete".
    HelpDelete,
    /// "Toggle hidden".
    HelpToggleHidden,
    /// "Next drive".
    HelpNextDrive,
    /// "Run snippet".
    HelpRunSnippet,
    /// "New".
    HelpNew,
    /// "Edit".
    HelpEdit,
    /// "Broadcast".
    HelpBroadcast,
    /// "New tab".
    HelpNewTab,
    /// "Close tab".
    HelpCloseTab,
    /// "Next tab".
    HelpNextTab,
    /// "V-split".
    HelpVSplit,
    /// "H-split".
    HelpHSplit,
    /// "Exit".
    HelpExit,
    /// "Select text".
    HelpSelectText,

    // -- Dashboard ---------------------------------------------------------
    /// "DASHBOARD" header.
    DashboardTitle,
    /// Header hint "r:refresh  s:sort  t:tags  g:group  /:search  a:add  x:execute  f:tunnel".
    DashboardHints,
    /// Extra hint appended when the selected host can take a key: "Shift+K:ssh-setup".
    DashboardHintKeySetup,
    /// "No hosts configured." empty state (multiline).
    DashboardEmptyNoHosts,
    /// "No hosts match the current filter." empty state.
    DashboardEmptyNoMatch,
    /// "sort".
    DashboardSort,
    /// "filter".
    DashboardFilter,
    /// "search".
    DashboardSearch,
    /// "Untagged".
    Untagged,

    // -- Server card -------------------------------------------------------
    /// "─── reachable ───" reachability line, no port.
    CardReachReachable,
    /// "─── reachable :{} ───" reachability line with the probed port.
    CardReachReachablePort,
    /// "─── unreachable ───" reachability line, no port.
    CardReachUnreachable,
    /// "─── unreachable :{} ───" reachability line with the probed port.
    CardReachUnreachablePort,
    /// "─── checking ───" reachability line, no port.
    CardReachChecking,
    /// "─── checking :{} ───" reachability line with the probed port.
    CardReachCheckingPort,
    /// "─── offline ───" shown when a host has no current metrics.
    CardOffline,
    /// "Up: {}" uptime label on the disk line.
    CardUptime,
    /// "{} running".
    CardDockerRunning,
    /// "{} stopped".
    CardDockerStopped,
    /// "{} restarting".
    CardDockerRestarting,
    /// "no containers".
    CardDockerNoContainers,
    /// "repl lag {}s".
    CardPgReplLag,
    /// "{} errors/5min".
    CardNginxErrors,
    /// "{}MB used".
    CardRedisMemUsed,
    /// "{} process(es)".
    CardNodeProcesses,
    /// "no processes".
    CardNodeNoProcesses,
    /// "ok" service status.
    CardServiceOk,

    // -- Detail view -------------------------------------------------------
    /// "Terminal too small for detail view. (min 60x20)".
    DetailTooSmall,
    /// "No host selected.".
    DetailNoHostSelected,
    /// "  SERVICES\n\n  No discovery data available. Press 'r' to refresh."
    DetailNoDiscovery,
    /// "Connected" connection status.
    DetailStatusConnected,
    /// "Connecting" connection status.
    DetailStatusConnecting,
    /// "Unknown" connection status.
    DetailStatusUnknown,
    /// "Failed: {}".
    DetailStatusFailed,
    /// "unknown" uptime placeholder.
    DetailUptimeUnknown,
    /// " Host: " header label.
    DetailLabelHost,
    /// "User: " header label.
    DetailLabelUser,
    /// "Up: " header label.
    DetailLabelUp,
    /// "Status: " header label.
    DetailLabelStatus,
    /// " OS: " header label.
    DetailLabelOs,
    /// "(discovery pending)" OS placeholder.
    DetailOsPending,
    /// "   Tunnel: " header label.
    DetailLabelTunnel,
    /// "up" tunnel state.
    DetailTunnelUp,
    /// "connecting" tunnel state.
    DetailTunnelConnecting,
    /// "retrying: {}" tunnel state.
    DetailTunnelRetrying,
    /// "failed: {}" tunnel state.
    DetailTunnelFailed,
    /// "off" tunnel state.
    DetailTunnelOff,
    /// " METRICS" section heading.
    DetailMetricsTitle,
    /// " unavailable" metrics-unavailable note.
    DetailMetricsUnavailable,
    /// " Load: " metric label.
    DetailLabelLoad,
    /// " (no metrics available)".
    DetailNoMetrics,
    /// " TOP PROCESSES" section heading.
    DetailTopProcessesTitle,
    /// "  cpu " per-process CPU label.
    DetailProcCpu,
    /// "   mem " per-process memory label.
    DetailProcMem,
    /// " (process data unavailable)".
    DetailProcUnavailable,
    /// " SERVICES" section heading.
    DetailServicesTitle,
    /// " No services detected".
    DetailNoServices,
    /// "{} running, {} stopped" Docker status.
    DetailDockerRunningStopped,
    /// "{} containers running" Docker status.
    DetailDockerRunning,
    /// "repl lag: {}s".
    DetailPgReplLag,
    /// "active, no replication lag".
    DetailPgNoReplLag,
    /// "active, {} errors/5min".
    DetailNginxErrors,
    /// "active, 0 errors/5min".
    DetailNginxNoErrors,
    /// "active, {}MB mem".
    DetailRedisMem,
    /// "active" status.
    DetailActive,
    /// "{} process(es) running".
    DetailNodeProcesses,
    /// "no processes".
    DetailNodeNoProcesses,
    /// ":Connect" key hint.
    DetailHintConnect,
    /// ":Refresh" key hint.
    DetailHintRefresh,
    /// ":← Dashboard" key hint.
    DetailHintDashboard,
    /// ":Quick view" key hint.
    DetailHintQuickView,
    /// ":Tunnel" key hint.
    DetailHintTunnel,

    // -- File Manager -------------------------------------------------------
    /// "REMOTE  {} — {}" (remote panel title next to a connected host).
    FmRemoteConnected,
    /// "REMOTE  (not connected)" (remote panel title with no host).
    FmRemoteNotConnected,
    /// "LOCAL  {}" (local panel title).
    FmLocal,
    /// "(loading…)" (empty panel that has a directory but no entries yet).
    FmLoading,
    /// "(empty)" (empty directory).
    FmEmpty,
    /// " Preview " (preview zone block title).
    FmPreviewTitle,
    /// "  (no preview — select a text file)".
    FmNoPreview,
    /// " Transfer Progress " (transfer block title).
    FmTransferProgressTitle,
    /// "  {} transferred…" (gauge label when the total size is unknown).
    FmTransferredUnknown,
    /// "  Cancelling…" (transfer status while the cancel is in flight).
    FmCancelling,
    /// " New Directory " (mkdir popup title).
    FmNewDirectory,
    /// " Rename '{}' " (rename popup title).
    FmRenameTitle,
    /// " Connect Remote Panel — Select Host " (host picker title).
    FmHostPickerTitle,
    /// "  No hosts configured. Add a host from the Dashboard (1)."
    FmHostPickerEmpty,
    /// ":navigate  " (host picker hint).
    FmHintNavigate,
    /// ":connect  " (host picker hint).
    FmHintConnect,
    /// ":confirm  " (text input popup hint).
    FmHintConfirm,
    /// ":Navigate" (file manager header hint).
    FmHintNavigateHeader,
    /// ":Switch" (file manager header hint).
    FmHintSwitch,
    /// ":Mark" (file manager header hint).
    FmHintMark,
    /// ":Copy" (file manager header hint).
    FmHintCopy,
    /// ":Paste" (file manager header hint).
    FmHintPaste,
    /// ":MkDir" (file manager header hint).
    FmHintMkDir,
    /// ":Rename" (file manager header hint).
    FmHintRename,
    /// ":Delete" (file manager header hint).
    FmHintDelete,
    /// ":Hidden" (file manager header hint).
    FmHintHidden,
    /// ":Host" (file manager header hint).
    FmHintHost,
    /// ":Drive" (file manager header hint, Windows only).
    FmHintDrive,
    /// " Confirm Delete " (delete-confirm popup title).
    FmConfirmDeleteTitle,
    /// "  Delete the following items?"
    FmDeletePrompt,
    /// "    … and {} more" (truncated delete list).
    FmDeleteMore,
    /// "  This cannot be undone."
    FmDeleteIrreversible,
    /// ":Yes — delete  " (delete-confirm hint).
    FmDeleteYes,
    /// ":No" (delete-confirm hint).
    FmDeleteNo,

    // -- Snippets -----------------------------------------------------------
    /// "Terminal too small for snippets screen."
    SnippetsTooSmall,
    /// "Add Snippet" (form popup title).
    SnippetsAddTitle,
    /// "Edit Snippet" (form popup title).
    SnippetsEditTitle,
    /// " Snippets ({}) " (header title with count).
    SnippetsHeaderTitle,
    /// "[search: {}] " (header search badge).
    SnippetsSearchTag,
    /// "[filter: {}] " (header filter badge).
    SnippetsFilterTag,
    /// "  n:new  e:edit  d:del  Enter:run  b:broadcast  /:search" (header hint bar).
    SnippetsHintBar,
    /// "  No snippets match." (empty state under a filter).
    SnippetsNoMatch,
    /// "  No snippets. Press  n  to create your first snippet." (empty state).
    SnippetsEmpty,

    // -- Popup dialogs -----------------------------------------------------
    /// "Add Host" (host form popup title).
    PopupAddHostTitle,
    /// "Edit Host" (host form popup title).
    PopupEditHostTitle,
    /// "  ↑ {} more" (scrolled-out fields above the form window).
    PopupFormMoreAbove,
    /// "  ↓ {} more" (scrolled-out fields below the form window).
    PopupFormMoreBelow,
    /// " Filter by tag [{}] " (tag filter popup title with an active filter).
    PopupFilterByTagActive,
    /// " Filter by tag " (tag filter popup title with no filter).
    PopupFilterByTag,
    /// "All (clear filter)" (tag filter first row).
    PopupTagAll,
    /// " Confirm Delete " (host delete-confirm popup title).
    PopupConfirmDeleteHostTitle,
    /// "  Delete host '{}'?" (host delete-confirm prompt).
    PopupDeleteHostPrompt,
    /// "  This cannot be undone.  " (host delete-confirm warning).
    PopupDeleteHostWarning,
    /// " Confirm Delete Snippet " (snippet delete-confirm popup title).
    PopupConfirmDeleteSnippetTitle,
    /// "  Delete snippet '{}'?" (snippet delete-confirm prompt).
    PopupDeleteSnippetPrompt,
    /// "  This cannot be undone." (snippet delete-confirm warning).
    PopupDeleteSnippetWarning,
    /// " Parameters — {} " (parameter input popup title).
    PopupParamsTitle,
    /// " Broadcast — Select Hosts " (broadcast host picker title).
    PopupBroadcastTitle,
    /// " Unlock SSH key — {} " (passphrase prompt title).
    PopupUnlockKeyTitle,
    /// "  Passphrase (kept in memory until OmnySSH exits):" (passphrase field label).
    PopupPassphraseLabel,
    /// " SSH login — {} " (password prompt title).
    PopupSshLoginTitle,
    /// "  Password (kept in memory until OmnySSH exits):" (password field label).
    PopupPasswordLabel,
    /// "Remember for future logins" (password prompt checkbox, Ctrl+R).
    PopupRememberPassword,
    /// "  Unlocking…" (passphrase prompt progress note).
    PopupUnlocking,
    /// "Permission denied, please try again." (password prompt retry error).
    PopupPermissionDenied,
    /// "New host key recorded: {}" (password prompt new host key notice).
    PopupNewHostKeyRecorded,
    /// " Quick Execute — {} " (quick-execute popup title).
    PopupQuickExecuteTitle,
    /// " Results " (snippet results popup title).
    PopupResultsTitle,
    /// " {} Running… " (snippet result running status with spinner).
    PopupResultRunning,
    /// " ✓ Done " (snippet result success status).
    PopupResultDone,
    /// " ✗ Error " (snippet result failure status).
    PopupResultError,
    /// "(no output)" (snippet result empty output placeholder).
    PopupResultNoOutput,
    /// "  Waiting for result…" (snippet result hint before output arrives).
    PopupResultWaiting,
    /// "Update now" (update popup button label).
    PopupUpdateNow,
    /// "Remind me later" (update popup button label).
    PopupUpdateRemindLater,
    /// "Skip this version" (update popup button label).
    PopupUpdateSkip,
    /// "Don't check again" (update popup button label).
    PopupUpdateDisable,
    /// " Update Available " (update popup title).
    PopupUpdateAvailableTitle,
    /// "  Version  " (update popup version label).
    PopupUpdateVersionLabel,
    /// "  This update can be downloaded and installed now." (update popup body).
    PopupUpdateSelfUpdate,
    /// "  The archive checksum is verified before installing." (update popup body).
    PopupUpdateChecksum,
    /// "  Update it with your package manager:" (update popup body).
    PopupUpdatePackageManager,
    /// "  Download the new release from:" (update popup body).
    PopupUpdateDownloadFrom,
    /// "  {} Downloading and installing — please wait…" (update popup progress).
    PopupUpdateInstalling,
    /// "  Please wait…" (generic wait note).
    PopupPleaseWait,

    // -- Transient status-bar messages -------------------------------------
    /// "Reloading hosts…".
    StatusReloadingHosts,
    /// "Refreshing metrics…".
    StatusRefreshingMetrics,
    /// "Connected to '{}'".
    StatusConnectedTo,
    /// "Connection failed for '{}': {}".
    StatusConnectionFailed,
    /// "Discovery failed for '{}': {}".
    StatusDiscoveryFailed,
    /// "Tunnel for '{}' is up".
    StatusTunnelUp,
    /// "Tunnel for '{}' failed: {}".
    StatusTunnelFailed,
    /// "Tunnel for '{}' stopped".
    StatusTunnelStopped,
    /// "Starting tunnel for '{}'…".
    StatusTunnelStarting,
    /// "No port forwards set for '{}' — add them with e".
    StatusNoPortForwards,
    /// "✓ SSH key setup complete for '{}'. Key: {}".
    StatusKeySetupComplete,
    /// "⚠ Key login works for '{}', but password login is still on (needs sudo without a password). Key: {}".
    StatusKeySetupPartial,
    /// "✗ Key setup failed for '{}' and password login may be off: {}".
    StatusKeySetupFailedUnsafe,
    /// "✗ Key setup failed for '{}': {}".
    StatusKeySetupFailed,
    /// "⚠ Key setup rolled back for '{}': {}".
    StatusKeySetupRolledBack,
    /// "SSH key needs a passphrase — Ctrl+Q to enter it: {}".
    StatusKeyPassphraseNeeded,
    /// "{} needs a password — Ctrl+Q to enter it".
    StatusPasswordNeeded,
    /// "Unlocked {}".
    StatusUnlocked,
    /// "'{}' already uses key authentication.".
    StatusAlreadyUsesKeyAuth,
    /// "No password set for this host. Add a password first to enable key setup.".
    StatusNoPasswordForKeySetup,
    /// "{} is no longer waiting for a password".
    StatusNoLongerWaitingPassword,
    /// "No host selected.".
    StatusNoHostSelected,
    /// "SFTP: {}".
    StatusSftp,
    /// "Update installed. Restart omny to use the new version.".
    StatusUpdateInstalled,
    /// "Update failed: {}".
    StatusUpdateFailed,
    /// "{} item(s) remaining…".
    StatusItemsRemaining,
    /// "Transfer failed: {}".
    StatusTransferFailed,
    /// "Transfer failed: {} (+{} more)".
    StatusTransferFailedMore,
    /// "The transfer finished before it could be cancelled.".
    StatusTransferFinishedNotCancelled,
    /// "Snippet '{}' failed on '{}': {}".
    StatusSnippetFailed,
    /// "Nothing to copy.".
    StatusNothingToCopy,
    /// "Copied to clipboard. Switch panel and press p to paste.".
    StatusCopiedToClipboard,
    /// "Nothing to delete.".
    StatusNothingToDelete,
    /// "Cancelling…".
    StatusCancelling,
    /// "Need at least 2 tabs to split. Open another tab with Ctrl+T.".
    StatusNeedTwoTabsToSplit,
    /// "No such host.".
    StatusNoSuchHost,
    /// "SSH session closed.".
    StatusSshSessionClosed,
    /// "PTY error: {}".
    StatusPtyError,
    /// "Host added.".
    StatusHostAdded,
    /// "Host '{}' added. Press 'Shift+K' to set up SSH key authentication (recommended).".
    StatusHostAddedWithPassword,
    /// "Host updated.".
    StatusHostUpdated,
    /// "Deleted '{}'.".
    StatusHostDeleted,
    /// "Save failed: {}".
    StatusSaveFailed,
    /// "Error: {}".
    StatusError,
    /// "Snippet added.".
    StatusSnippetAdded,
    /// "Snippet updated.".
    StatusSnippetUpdated,
    /// "Deleted snippet '{}'.".
    StatusSnippetDeleted,
    /// "No hosts selected.".
    StatusNoHostsSelected,
    /// "Host not found.".
    StatusFmHostNotFound,
    /// "Connecting to '{}'…".
    StatusFmConnecting,
    /// "No other drives".
    StatusFmNoOtherDrives,
    /// "Nothing in clipboard.".
    StatusFmNothingInClipboard,
    /// "Cannot paste to the same panel.".
    StatusFmCannotPasteSamePanel,
    /// "Clipboard is empty.".
    StatusFmClipboardEmpty,
    /// "Not connected to a host.".
    StatusFmNotConnected,
    /// "Queued {} items for transfer…".
    StatusFmQueuedItems,
    /// "Wait for the running operation to finish.".
    StatusFmWaitForRunning,

    // -- Form validation errors --------------------------------------------
    /// "Name cannot be empty".
    ErrNameEmpty,
    /// "Hostname / IP cannot be empty".
    ErrHostnameEmpty,
    /// "Port must be a number between 1 and 65535, got '{}'".
    ErrPortInvalid,
    /// "Monitoring must be 'ssh', 'tcp' or 'tcp:PORT', got '{}'".
    ErrMonitoringInvalid,
    /// "Port forward: {}".
    ErrPortForward,
    /// "Port forward: two rules listen on port {}".
    ErrPortForwardDupPort,
    /// "{} must be y or n, got '{}'".
    ErrYesNoInvalid,
    /// "Start tunnel on launch" (the field name used in its y/n error).
    ErrLabelStartTunnel,
    /// "Forward SSH agent" (the field name used in its y/n error).
    ErrLabelForwardAgent,
    /// "Command cannot be empty".
    ErrSnippetCommandEmpty,
    /// "Host cannot be empty when scope is 'host'".
    ErrSnippetHostEmpty,
    /// "Host '{}' not found.".
    StatusHostNotFound,
    /// "Connect failed: {}".
    ErrConnectFailed,
    /// "Command failed: {}".
    ErrCommandFailed,
    /// "Connection failed: {}" (key-setup background task).
    ErrKeySetupConnectionFailed,
    /// "Rolled back." (key-setup background task).
    ErrKeySetupRolledBack,
    /// "Unknown failure." (key-setup background task).
    ErrKeySetupUnknownFailure,

    // -- Snippet form label (new) ------------------------------------------
    /// "Host (if scope=host)".
    SnippetFormHost,

    // -- Terminal end line -------------------------------------------------
    /// The dim line written under a session's last output when it ends.
    TerminalEndLine,

    // -- Language switching ------------------------------------------------
    /// "Language: {}" — shown in the status bar after a switch.
    StatusLanguageSwitched,
}

impl Key {
    /// Every key, for the tests that prove the table is complete.
    ///
    /// Public so integration tests (and any future tooling) can walk the table;
    /// the UI itself never enumerates keys, so the binary sees no use for it.
    #[allow(dead_code)]
    pub const ALL: &'static [Key] = &[
        Key::TerminalTooSmall,
        Key::NavDashboard,
        Key::NavFiles,
        Key::NavSnippets,
        Key::NavTerminal,
        Key::NavHelp,
        Key::NavQuit,
        Key::SearchBadge,
        Key::SearchTypeToFilter,
        Key::HintConfirm,
        Key::HintClear,
        Key::HintCancel,
        Key::HintClose,
        Key::HintSave,
        Key::HintSelect,
        Key::HintScroll,
        Key::HintRun,
        Key::HintConnect,
        Key::HintNavigate,
        Key::HintToggle,
        Key::HintNextField,
        Key::HintPrevField,
        Key::HintNextParam,
        Key::HintConfirmDelete,
        Key::HintConfirmSetup,
        Key::SettingUpKeys,
        Key::HintTransferring,
        Key::FormName,
        Key::FormHostname,
        Key::FormUser,
        Key::FormPort,
        Key::FormIdentityFile,
        Key::FormPassword,
        Key::FormTags,
        Key::FormNotes,
        Key::FormMonitoring,
        Key::FormForwards,
        Key::FormStartTunnel,
        Key::FormForwardAgent,
        Key::FormForwardAgentWindows,
        Key::SnippetFormName,
        Key::SnippetFormCommand,
        Key::SnippetFormDescription,
        Key::SnippetFormTags,
        Key::SnippetFormParams,
        Key::SnippetFormScope,
        Key::SortName,
        Key::SortCpu,
        Key::SortRam,
        Key::SortStatus,
        Key::HelpTitle,
        Key::HelpSectionGlobalNav,
        Key::HelpSectionDashboard,
        Key::HelpSectionDetailView,
        Key::HelpSectionFileManager,
        Key::HelpSectionSnippets,
        Key::HelpSectionTerminal,
        Key::HelpSectionCopyText,
        Key::HelpFooterClose,
        Key::HelpTerminalOnly,
        Key::HelpOpenDetails,
        Key::HelpAddHost,
        Key::HelpEditHost,
        Key::HelpDeleteHost,
        Key::HelpRefresh,
        Key::HelpSort,
        Key::HelpFilterTags,
        Key::HelpGroupByTag,
        Key::HelpSearch,
        Key::HelpQuickExec,
        Key::HelpSetupKey,
        Key::HelpTunnelToggle,
        Key::HelpNavigate,
        Key::HelpConnect,
        Key::HelpBack,
        Key::HelpQuickView,
        Key::HelpSwitchPanel,
        Key::HelpMarkFile,
        Key::HelpCopy,
        Key::HelpPaste,
        Key::HelpNewDir,
        Key::HelpRename,
        Key::HelpDelete,
        Key::HelpToggleHidden,
        Key::HelpNextDrive,
        Key::HelpRunSnippet,
        Key::HelpNew,
        Key::HelpEdit,
        Key::HelpBroadcast,
        Key::HelpNewTab,
        Key::HelpCloseTab,
        Key::HelpNextTab,
        Key::HelpVSplit,
        Key::HelpHSplit,
        Key::HelpExit,
        Key::HelpSelectText,
        Key::DashboardTitle,
        Key::DashboardHints,
        Key::DashboardHintKeySetup,
        Key::DashboardEmptyNoHosts,
        Key::DashboardEmptyNoMatch,
        Key::DashboardSort,
        Key::DashboardFilter,
        Key::DashboardSearch,
        Key::Untagged,
        Key::CardReachReachable,
        Key::CardReachReachablePort,
        Key::CardReachUnreachable,
        Key::CardReachUnreachablePort,
        Key::CardReachChecking,
        Key::CardReachCheckingPort,
        Key::CardOffline,
        Key::CardUptime,
        Key::CardDockerRunning,
        Key::CardDockerStopped,
        Key::CardDockerRestarting,
        Key::CardDockerNoContainers,
        Key::CardPgReplLag,
        Key::CardNginxErrors,
        Key::CardRedisMemUsed,
        Key::CardNodeProcesses,
        Key::CardNodeNoProcesses,
        Key::CardServiceOk,
        Key::DetailTooSmall,
        Key::DetailNoHostSelected,
        Key::DetailNoDiscovery,
        Key::DetailStatusConnected,
        Key::DetailStatusConnecting,
        Key::DetailStatusUnknown,
        Key::DetailStatusFailed,
        Key::DetailUptimeUnknown,
        Key::DetailLabelHost,
        Key::DetailLabelUser,
        Key::DetailLabelUp,
        Key::DetailLabelStatus,
        Key::DetailLabelOs,
        Key::DetailOsPending,
        Key::DetailLabelTunnel,
        Key::DetailTunnelUp,
        Key::DetailTunnelConnecting,
        Key::DetailTunnelRetrying,
        Key::DetailTunnelFailed,
        Key::DetailTunnelOff,
        Key::DetailMetricsTitle,
        Key::DetailMetricsUnavailable,
        Key::DetailLabelLoad,
        Key::DetailNoMetrics,
        Key::DetailTopProcessesTitle,
        Key::DetailProcCpu,
        Key::DetailProcMem,
        Key::DetailProcUnavailable,
        Key::DetailServicesTitle,
        Key::DetailNoServices,
        Key::DetailDockerRunningStopped,
        Key::DetailDockerRunning,
        Key::DetailPgReplLag,
        Key::DetailPgNoReplLag,
        Key::DetailNginxErrors,
        Key::DetailNginxNoErrors,
        Key::DetailRedisMem,
        Key::DetailActive,
        Key::DetailNodeProcesses,
        Key::DetailNodeNoProcesses,
        Key::DetailHintConnect,
        Key::DetailHintRefresh,
        Key::DetailHintDashboard,
        Key::DetailHintQuickView,
        Key::DetailHintTunnel,
        Key::FmRemoteConnected,
        Key::FmRemoteNotConnected,
        Key::FmLocal,
        Key::FmLoading,
        Key::FmEmpty,
        Key::FmPreviewTitle,
        Key::FmNoPreview,
        Key::FmTransferProgressTitle,
        Key::FmTransferredUnknown,
        Key::FmCancelling,
        Key::FmNewDirectory,
        Key::FmRenameTitle,
        Key::FmHostPickerTitle,
        Key::FmHostPickerEmpty,
        Key::FmHintNavigate,
        Key::FmHintConnect,
        Key::FmHintConfirm,
        Key::FmHintNavigateHeader,
        Key::FmHintSwitch,
        Key::FmHintMark,
        Key::FmHintCopy,
        Key::FmHintPaste,
        Key::FmHintMkDir,
        Key::FmHintRename,
        Key::FmHintDelete,
        Key::FmHintHidden,
        Key::FmHintHost,
        Key::FmHintDrive,
        Key::FmConfirmDeleteTitle,
        Key::FmDeletePrompt,
        Key::FmDeleteMore,
        Key::FmDeleteIrreversible,
        Key::FmDeleteYes,
        Key::FmDeleteNo,
        Key::SnippetsTooSmall,
        Key::SnippetsAddTitle,
        Key::SnippetsEditTitle,
        Key::SnippetsHeaderTitle,
        Key::SnippetsSearchTag,
        Key::SnippetsFilterTag,
        Key::SnippetsHintBar,
        Key::SnippetsNoMatch,
        Key::SnippetsEmpty,
        Key::PopupAddHostTitle,
        Key::PopupEditHostTitle,
        Key::PopupFormMoreAbove,
        Key::PopupFormMoreBelow,
        Key::PopupFilterByTagActive,
        Key::PopupFilterByTag,
        Key::PopupTagAll,
        Key::PopupConfirmDeleteHostTitle,
        Key::PopupDeleteHostPrompt,
        Key::PopupDeleteHostWarning,
        Key::PopupConfirmDeleteSnippetTitle,
        Key::PopupDeleteSnippetPrompt,
        Key::PopupDeleteSnippetWarning,
        Key::PopupParamsTitle,
        Key::PopupBroadcastTitle,
        Key::PopupUnlockKeyTitle,
        Key::PopupPassphraseLabel,
        Key::PopupSshLoginTitle,
        Key::PopupPasswordLabel,
        Key::PopupRememberPassword,
        Key::PopupUnlocking,
        Key::PopupPermissionDenied,
        Key::PopupNewHostKeyRecorded,
        Key::PopupQuickExecuteTitle,
        Key::PopupResultsTitle,
        Key::PopupResultRunning,
        Key::PopupResultDone,
        Key::PopupResultError,
        Key::PopupResultNoOutput,
        Key::PopupResultWaiting,
        Key::PopupUpdateNow,
        Key::PopupUpdateRemindLater,
        Key::PopupUpdateSkip,
        Key::PopupUpdateDisable,
        Key::PopupUpdateAvailableTitle,
        Key::PopupUpdateVersionLabel,
        Key::PopupUpdateSelfUpdate,
        Key::PopupUpdateChecksum,
        Key::PopupUpdatePackageManager,
        Key::PopupUpdateDownloadFrom,
        Key::PopupUpdateInstalling,
        Key::PopupPleaseWait,
        Key::StatusReloadingHosts,
        Key::StatusRefreshingMetrics,
        Key::StatusConnectedTo,
        Key::StatusConnectionFailed,
        Key::StatusDiscoveryFailed,
        Key::StatusTunnelUp,
        Key::StatusTunnelFailed,
        Key::StatusTunnelStopped,
        Key::StatusTunnelStarting,
        Key::StatusNoPortForwards,
        Key::StatusKeySetupComplete,
        Key::StatusKeySetupPartial,
        Key::StatusKeySetupFailedUnsafe,
        Key::StatusKeySetupFailed,
        Key::StatusKeySetupRolledBack,
        Key::StatusKeyPassphraseNeeded,
        Key::StatusPasswordNeeded,
        Key::StatusUnlocked,
        Key::StatusAlreadyUsesKeyAuth,
        Key::StatusNoPasswordForKeySetup,
        Key::StatusNoLongerWaitingPassword,
        Key::StatusNoHostSelected,
        Key::StatusSftp,
        Key::StatusUpdateInstalled,
        Key::StatusUpdateFailed,
        Key::StatusItemsRemaining,
        Key::StatusTransferFailed,
        Key::StatusTransferFailedMore,
        Key::StatusTransferFinishedNotCancelled,
        Key::StatusSnippetFailed,
        Key::StatusNothingToCopy,
        Key::StatusCopiedToClipboard,
        Key::StatusNothingToDelete,
        Key::StatusCancelling,
        Key::StatusNeedTwoTabsToSplit,
        Key::StatusNoSuchHost,
        Key::StatusSshSessionClosed,
        Key::StatusPtyError,
        Key::StatusHostAdded,
        Key::StatusHostAddedWithPassword,
        Key::StatusHostUpdated,
        Key::StatusHostDeleted,
        Key::StatusSaveFailed,
        Key::StatusError,
        Key::StatusSnippetAdded,
        Key::StatusSnippetUpdated,
        Key::StatusSnippetDeleted,
        Key::StatusNoHostsSelected,
        Key::StatusFmHostNotFound,
        Key::StatusFmConnecting,
        Key::StatusFmNoOtherDrives,
        Key::StatusFmNothingInClipboard,
        Key::StatusFmCannotPasteSamePanel,
        Key::StatusFmClipboardEmpty,
        Key::StatusFmNotConnected,
        Key::StatusFmQueuedItems,
        Key::StatusFmWaitForRunning,
        Key::ErrNameEmpty,
        Key::ErrHostnameEmpty,
        Key::ErrPortInvalid,
        Key::ErrMonitoringInvalid,
        Key::ErrPortForward,
        Key::ErrPortForwardDupPort,
        Key::ErrYesNoInvalid,
        Key::ErrLabelStartTunnel,
        Key::ErrLabelForwardAgent,
        Key::ErrSnippetCommandEmpty,
        Key::ErrSnippetHostEmpty,
        Key::StatusHostNotFound,
        Key::ErrConnectFailed,
        Key::ErrCommandFailed,
        Key::ErrKeySetupConnectionFailed,
        Key::ErrKeySetupRolledBack,
        Key::ErrKeySetupUnknownFailure,
        Key::SnippetFormHost,
        Key::TerminalEndLine,
        Key::StatusLanguageSwitched,
    ];
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

/// The translation of `key` in `lang`.
///
/// Every key is covered in every language, so this never returns an empty
/// string; a missing arm is a compile error, not a blank cell.
pub fn t(lang: Lang, key: Key) -> &'static str {
    match lang {
        Lang::En => en(key),
        Lang::ZhHans => zh_hans(key),
    }
}

/// The translation of `key`, with `{}` placeholders filled from `args` in
/// order. Extra args are ignored; a missing one is a compile error at the
/// call site.
pub fn tf(lang: Lang, key: Key, args: &[&str]) -> String {
    let mut template = t(lang, key).to_string();
    for arg in args {
        if let Some(at) = template.find("{}") {
            template.replace_range(at..at + 2, arg);
        }
    }
    template
}

fn en(key: Key) -> &'static str {
    match key {
        Key::TerminalTooSmall => "Terminal too small — please resize to at least 80×24.",
        Key::NavDashboard => "Dashboard",
        Key::NavFiles => "Files",
        Key::NavSnippets => "Snippets",
        Key::NavTerminal => "Terminal",
        Key::NavHelp => "Help",
        Key::NavQuit => "Quit",
        Key::SearchBadge => "[SEARCH]",
        Key::SearchTypeToFilter => "type to filter",
        Key::HintConfirm => "confirm",
        Key::HintClear => "clear",
        Key::HintCancel => "cancel",
        Key::HintClose => "close",
        Key::HintSave => "save",
        Key::HintSelect => "select",
        Key::HintScroll => "scroll",
        Key::HintRun => "run",
        Key::HintConnect => "connect",
        Key::HintNavigate => "navigate",
        Key::HintToggle => "toggle",
        Key::HintNextField => "next field",
        Key::HintPrevField => "prev field",
        Key::HintNextParam => "next param",
        Key::HintConfirmDelete => "confirm delete",
        Key::HintConfirmSetup => "confirm setup",
        Key::SettingUpKeys => "Setting up SSH keys…",
        Key::HintTransferring => "Transferring",
        Key::FormName => "Name",
        Key::FormHostname => "Hostname / IP",
        Key::FormUser => "User",
        Key::FormPort => "Port",
        Key::FormIdentityFile => "Identity File",
        Key::FormPassword => "Password (login, optional)",
        Key::FormTags => "Tags (comma-sep)",
        Key::FormNotes => "Notes",
        Key::FormMonitoring => "Monitoring (ssh | tcp | tcp:PORT)",
        Key::FormForwards => "Port forwards (port:host:hostport, ...)",
        Key::FormStartTunnel => "Start tunnel on launch (y/n)",
        Key::FormForwardAgent => "Forward SSH agent (y/n)",
        Key::FormForwardAgentWindows => "Forward SSH agent (y/n, not on Windows yet)",
        Key::SnippetFormName => "Name",
        Key::SnippetFormCommand => "Command",
        Key::SnippetFormDescription => "Description",
        Key::SnippetFormTags => "Tags (comma-sep)",
        Key::SnippetFormParams => "Parameters (comma-sep)",
        Key::SnippetFormScope => "Scope (global | host)",
        Key::SortName => "name",
        Key::SortCpu => "cpu",
        Key::SortRam => "ram",        Key::SortStatus => "status",
        Key::HelpTitle => " Help — Keyboard Shortcuts ",
        Key::HelpSectionGlobalNav => " GLOBAL NAVIGATION",
        Key::HelpSectionDashboard => " DASHBOARD",
        Key::HelpSectionDetailView => " DETAIL VIEW",
        Key::HelpSectionFileManager => " FILE MANAGER",
        Key::HelpSectionSnippets => " SNIPPETS",
        Key::HelpSectionTerminal => " TERMINAL",
        Key::HelpSectionCopyText => " COPY TEXT",
        Key::HelpFooterClose => " Press Esc or ? to close",
        Key::HelpTerminalOnly => "  (Terminal screen only)",
        Key::HelpOpenDetails => "    Open details",
        Key::HelpAddHost => "        Add host",
        Key::HelpEditHost => "        Edit host",
        Key::HelpDeleteHost => "        Delete host",
        Key::HelpRefresh => "        Refresh",
        Key::HelpSort => "        Sort",
        Key::HelpFilterTags => "        Filter tags",
        Key::HelpGroupByTag => "        Group by tag",
        Key::HelpSearch => "        Search",
        Key::HelpQuickExec => "        Quick exec",
        Key::HelpSetupKey => "        Setup SSH key",
        Key::HelpTunnelToggle => "        Tunnel on/off",
        Key::HelpNavigate => "    Navigate",
        Key::HelpConnect => "    Connect",
        Key::HelpBack => "      Back",
        Key::HelpQuickView => "      Quick view",
        Key::HelpSwitchPanel => "      Switch panel",
        Key::HelpMarkFile => "    Mark file",
        Key::HelpCopy => "        Copy",
        Key::HelpPaste => "        Paste",
        Key::HelpNewDir => "        New dir",
        Key::HelpRename => "        Rename",
        Key::HelpDelete => "        Delete",
        Key::HelpToggleHidden => "        Toggle hidden",
        Key::HelpNextDrive => "        Next drive",
        Key::HelpRunSnippet => "    Run snippet",
        Key::HelpNew => "        New",
        Key::HelpEdit => "        Edit",
        Key::HelpBroadcast => "        Broadcast",
        Key::HelpNewTab => "   New tab",
        Key::HelpCloseTab => "   Close tab",
        Key::HelpNextTab => "   Next tab",
        Key::HelpVSplit => "   V-split",
        Key::HelpHSplit => "   H-split",
        Key::HelpExit => "   Exit",
        Key::HelpSelectText => " Select text",
        Key::DashboardTitle => "Dashboard",
        Key::DashboardHints => {
            "r:refresh  s:sort  t:tags  g:group  /:search  a:add  x:execute  f:tunnel"
        }
        Key::DashboardHintKeySetup => "  Shift+K:ssh-setup",
        Key::DashboardEmptyNoHosts => {
            "\n  No hosts configured.\n\n  Press  a  to add a host, or  r  to reload from ~/.ssh/config."
        }
        Key::DashboardEmptyNoMatch => "\n  No hosts match the current filter.",
        Key::DashboardSort => "sort",
        Key::DashboardFilter => "filter",
        Key::DashboardSearch => "search",
        Key::Untagged => "Untagged",
        Key::CardReachReachable => "─── reachable ───",
        Key::CardReachReachablePort => "─── reachable :{} ───",
        Key::CardReachUnreachable => "─── unreachable ───",
        Key::CardReachUnreachablePort => "─── unreachable :{} ───",
        Key::CardReachChecking => "─── checking ───",
        Key::CardReachCheckingPort => "─── checking :{} ───",
        Key::CardOffline => "─── offline ───",
        Key::CardUptime => "Up: {}",
        Key::CardDockerRunning => "{} running",
        Key::CardDockerStopped => "{} stopped",
        Key::CardDockerRestarting => "{} restarting",
        Key::CardDockerNoContainers => "no containers",
        Key::CardPgReplLag => "repl lag {}s",
        Key::CardNginxErrors => "{} errors/5min",
        Key::CardRedisMemUsed => "{}MB used",
        Key::CardNodeProcesses => "{} process(es)",
        Key::CardNodeNoProcesses => "no processes",
        Key::CardServiceOk => "ok",
        Key::DetailTooSmall => "Terminal too small for detail view. (min 60x20)",
        Key::DetailNoHostSelected => "No host selected.",
        Key::DetailNoDiscovery => {
            "  SERVICES\n\n  No discovery data available. Press 'r' to refresh."
        }
        Key::DetailStatusConnected => "Connected",
        Key::DetailStatusConnecting => "Connecting",
        Key::DetailStatusUnknown => "Unknown",
        Key::DetailStatusFailed => "Failed: {}",
        Key::DetailUptimeUnknown => "unknown",
        Key::DetailLabelHost => " Host: ",
        Key::DetailLabelUser => "User: ",
        Key::DetailLabelUp => "Up: ",
        Key::DetailLabelStatus => "Status: ",
        Key::DetailLabelOs => " OS: ",
        Key::DetailOsPending => "(discovery pending)",
        Key::DetailLabelTunnel => "   Tunnel: ",
        Key::DetailTunnelUp => "up",
        Key::DetailTunnelConnecting => "connecting",
        Key::DetailTunnelRetrying => "retrying: {}",
        Key::DetailTunnelFailed => "failed: {}",
        Key::DetailTunnelOff => "off",
        Key::DetailMetricsTitle => " METRICS",
        Key::DetailMetricsUnavailable => " unavailable",
        Key::DetailLabelLoad => " Load: ",
        Key::DetailNoMetrics => " (no metrics available)",
        Key::DetailTopProcessesTitle => " TOP PROCESSES",
        Key::DetailProcCpu => "  cpu ",
        Key::DetailProcMem => "   mem ",
        Key::DetailProcUnavailable => " (process data unavailable)",
        Key::DetailServicesTitle => " SERVICES",
        Key::DetailNoServices => " No services detected",
        Key::DetailDockerRunningStopped => "{} running, {} stopped",
        Key::DetailDockerRunning => "{} containers running",
        Key::DetailPgReplLag => "repl lag: {}s",
        Key::DetailPgNoReplLag => "active, no replication lag",
        Key::DetailNginxErrors => "active, {} errors/5min",
        Key::DetailNginxNoErrors => "active, 0 errors/5min",
        Key::DetailRedisMem => "active, {}MB mem",
        Key::DetailActive => "active",
        Key::DetailNodeProcesses => "{} process(es) running",
        Key::DetailNodeNoProcesses => "no processes",
        Key::DetailHintConnect => ":Connect",
        Key::DetailHintRefresh => ":Refresh",
        Key::DetailHintDashboard => ":← Dashboard",
        Key::DetailHintQuickView => ":Quick view",
        Key::DetailHintTunnel => ":Tunnel",
        Key::FmRemoteConnected => "REMOTE  {} — {}",
        Key::FmRemoteNotConnected => "REMOTE  (not connected)",
        Key::FmLocal => "LOCAL  {}",
        Key::FmLoading => "(loading…)",
        Key::FmEmpty => "(empty)",
        Key::FmPreviewTitle => " Preview ",
        Key::FmNoPreview => "  (no preview — select a text file)",
        Key::FmTransferProgressTitle => " Transfer Progress ",
        Key::FmTransferredUnknown => "  {} transferred…",
        Key::FmCancelling => "  Cancelling…",
        Key::FmNewDirectory => " New Directory ",
        Key::FmRenameTitle => " Rename '{}' ",
        Key::FmHostPickerTitle => " Connect Remote Panel — Select Host ",
        Key::FmHostPickerEmpty => "  No hosts configured. Add a host from the Dashboard (1).",
        Key::FmHintNavigate => ":navigate  ",
        Key::FmHintConnect => ":connect  ",
        Key::FmHintConfirm => ":confirm  ",
        Key::FmHintNavigateHeader => ":Navigate",
        Key::FmHintSwitch => ":Switch",
        Key::FmHintMark => ":Mark",
        Key::FmHintCopy => ":Copy",
        Key::FmHintPaste => ":Paste",
        Key::FmHintMkDir => ":MkDir",
        Key::FmHintRename => ":Rename",
        Key::FmHintDelete => ":Delete",
        Key::FmHintHidden => ":Hidden",
        Key::FmHintHost => ":Host",
        Key::FmHintDrive => ":Drive",
        Key::FmConfirmDeleteTitle => " Confirm Delete ",
        Key::FmDeletePrompt => "  Delete the following items?",
        Key::FmDeleteMore => "    … and {} more",
        Key::FmDeleteIrreversible => "  This cannot be undone.",
        Key::FmDeleteYes => ":Yes — delete  ",
        Key::FmDeleteNo => ":No",
        Key::SnippetsTooSmall => "Terminal too small for snippets screen.",
        Key::SnippetsAddTitle => "Add Snippet",
        Key::SnippetsEditTitle => "Edit Snippet",
        Key::SnippetsHeaderTitle => " Snippets ({}) ",
        Key::SnippetsSearchTag => "[search: {}] ",
        Key::SnippetsFilterTag => "[filter: {}] ",
        Key::SnippetsHintBar => "  n:new  e:edit  d:del  Enter:run  b:broadcast  /:search",
        Key::SnippetsNoMatch => "  No snippets match.",
        Key::SnippetsEmpty => "  No snippets. Press  n  to create your first snippet.",
        Key::PopupAddHostTitle => "Add Host",
        Key::PopupEditHostTitle => "Edit Host",
        Key::PopupFormMoreAbove => "  ↑ {} more",
        Key::PopupFormMoreBelow => "  ↓ {} more",
        Key::PopupFilterByTagActive => " Filter by tag [{}] ",
        Key::PopupFilterByTag => " Filter by tag ",
        Key::PopupTagAll => "All (clear filter)",
        Key::PopupConfirmDeleteHostTitle => " Confirm Delete ",
        Key::PopupDeleteHostPrompt => "  Delete host '{}'?",
        Key::PopupDeleteHostWarning => "  This cannot be undone.  ",
        Key::PopupConfirmDeleteSnippetTitle => " Confirm Delete Snippet ",
        Key::PopupDeleteSnippetPrompt => "  Delete snippet '{}'?",
        Key::PopupDeleteSnippetWarning => "  This cannot be undone.",
        Key::PopupParamsTitle => " Parameters — {} ",
        Key::PopupBroadcastTitle => " Broadcast — Select Hosts ",
        Key::PopupUnlockKeyTitle => " Unlock SSH key — {} ",
        Key::PopupPassphraseLabel => "  Passphrase (kept in memory until OmnySSH exits):",
        Key::PopupSshLoginTitle => " SSH login — {} ",
        Key::PopupPasswordLabel => "  Password (kept in memory until OmnySSH exits):",
        Key::PopupRememberPassword => "Remember for future logins",
        Key::PopupUnlocking => "  Unlocking…",
        Key::PopupPermissionDenied => "Permission denied, please try again.",
        Key::PopupNewHostKeyRecorded => "New host key recorded: {}",
        Key::PopupQuickExecuteTitle => " Quick Execute — {} ",
        Key::PopupResultsTitle => " Results ",
        Key::PopupResultRunning => " {} Running… ",
        Key::PopupResultDone => " ✓ Done ",
        Key::PopupResultError => " ✗ Error ",
        Key::PopupResultNoOutput => "(no output)",
        Key::PopupResultWaiting => "  Waiting for result…",
        Key::PopupUpdateNow => "Update now",
        Key::PopupUpdateRemindLater => "Remind me later",
        Key::PopupUpdateSkip => "Skip this version",
        Key::PopupUpdateDisable => "Don't check again",
        Key::PopupUpdateAvailableTitle => " Update Available ",
        Key::PopupUpdateVersionLabel => "  Version  ",
        Key::PopupUpdateSelfUpdate => "  This update can be downloaded and installed now.",
        Key::PopupUpdateChecksum => "  The archive checksum is verified before installing.",
        Key::PopupUpdatePackageManager => "  Update it with your package manager:",
        Key::PopupUpdateDownloadFrom => "  Download the new release from:",
        Key::PopupUpdateInstalling => "  {} Downloading and installing — please wait…",
        Key::PopupPleaseWait => "  Please wait…",
        Key::StatusReloadingHosts => "Reloading hosts…",
        Key::StatusRefreshingMetrics => "Refreshing metrics…",
        Key::StatusConnectedTo => "Connected to '{}'",
        Key::StatusConnectionFailed => "Connection failed for '{}': {}",
        Key::StatusDiscoveryFailed => "Discovery failed for '{}': {}",
        Key::StatusTunnelUp => "Tunnel for '{}' is up",
        Key::StatusTunnelFailed => "Tunnel for '{}' failed: {}",
        Key::StatusTunnelStopped => "Tunnel for '{}' stopped",
        Key::StatusTunnelStarting => "Starting tunnel for '{}'…",
        Key::StatusNoPortForwards => "No port forwards set for '{}' — add them with e",
        Key::StatusKeySetupComplete => "✓ SSH key setup complete for '{}'. Key: {}",
        Key::StatusKeySetupPartial => {
            "⚠ Key login works for '{}', but password login is still on (needs sudo without a password). Key: {}"
        }
        Key::StatusKeySetupFailedUnsafe => {
            "✗ Key setup failed for '{}' and password login may be off: {}"
        }
        Key::StatusKeySetupFailed => "✗ Key setup failed for '{}': {}",
        Key::StatusKeySetupRolledBack => "⚠ Key setup rolled back for '{}': {}",
        Key::StatusKeyPassphraseNeeded => {
            "SSH key needs a passphrase — Ctrl+Q to enter it: {}"
        }
        Key::StatusPasswordNeeded => "{} needs a password — Ctrl+Q to enter it",
        Key::StatusUnlocked => "Unlocked {}",
        Key::StatusAlreadyUsesKeyAuth => "'{}' already uses key authentication.",
        Key::StatusNoPasswordForKeySetup => {
            "No password set for this host. Add a password first to enable key setup."
        }
        Key::StatusNoLongerWaitingPassword => "{} is no longer waiting for a password",
        Key::StatusNoHostSelected => "No host selected.",
        Key::StatusSftp => "SFTP: {}",
        Key::StatusUpdateInstalled => {
            "Update installed. Restart omny to use the new version."
        }
        Key::StatusUpdateFailed => "Update failed: {}",
        Key::StatusItemsRemaining => "{} item(s) remaining…",
        Key::StatusTransferFailed => "Transfer failed: {}",
        Key::StatusTransferFailedMore => "Transfer failed: {} (+{} more)",
        Key::StatusTransferFinishedNotCancelled => {
            "The transfer finished before it could be cancelled."
        }
        Key::StatusSnippetFailed => "Snippet '{}' failed on '{}': {}",
        Key::StatusNothingToCopy => "Nothing to copy.",
        Key::StatusCopiedToClipboard => {
            "Copied to clipboard. Switch panel and press p to paste."
        }
        Key::StatusNothingToDelete => "Nothing to delete.",
        Key::StatusCancelling => "Cancelling…",
        Key::StatusNeedTwoTabsToSplit => {
            "Need at least 2 tabs to split. Open another tab with Ctrl+T."
        }
        Key::StatusNoSuchHost => "No such host.",
        Key::StatusSshSessionClosed => "SSH session closed.",
        Key::StatusPtyError => "PTY error: {}",
        Key::StatusHostAdded => "Host added.",
        Key::StatusHostAddedWithPassword => {
            "Host '{}' added. Press 'Shift+K' to set up SSH key authentication (recommended)."
        }
        Key::StatusHostUpdated => "Host updated.",
        Key::StatusHostDeleted => "Deleted '{}'.",
        Key::StatusSaveFailed => "Save failed: {}",
        Key::StatusError => "Error: {}",
        Key::StatusSnippetAdded => "Snippet added.",
        Key::StatusSnippetUpdated => "Snippet updated.",
        Key::StatusSnippetDeleted => "Deleted snippet '{}'.",
        Key::StatusNoHostsSelected => "No hosts selected.",
        Key::StatusFmHostNotFound => "Host not found.",
        Key::StatusFmConnecting => "Connecting to '{}'…",
        Key::StatusFmNoOtherDrives => "No other drives",
        Key::StatusFmNothingInClipboard => "Nothing in clipboard.",
        Key::StatusFmCannotPasteSamePanel => "Cannot paste to the same panel.",
        Key::StatusFmClipboardEmpty => "Clipboard is empty.",
        Key::StatusFmNotConnected => "Not connected to a host.",
        Key::StatusFmQueuedItems => "Queued {} items for transfer…",
        Key::StatusFmWaitForRunning => "Wait for the running operation to finish.",
        Key::ErrNameEmpty => "Name cannot be empty",
        Key::ErrHostnameEmpty => "Hostname / IP cannot be empty",
        Key::ErrPortInvalid => "Port must be a number between 1 and 65535, got '{}'",
        Key::ErrMonitoringInvalid => {
            "Monitoring must be 'ssh', 'tcp' or 'tcp:PORT', got '{}'"
        }
        Key::ErrPortForward => "Port forward: {}",
        Key::ErrPortForwardDupPort => "Port forward: two rules listen on port {}",
        Key::ErrYesNoInvalid => "{} must be y or n, got '{}'",
        Key::ErrLabelStartTunnel => "Start tunnel on launch",
        Key::ErrLabelForwardAgent => "Forward SSH agent",
        Key::ErrSnippetCommandEmpty => "Command cannot be empty",
        Key::ErrSnippetHostEmpty => "Host cannot be empty when scope is 'host'",
        Key::StatusHostNotFound => "Host '{}' not found.",
        Key::ErrConnectFailed => "Connect failed: {}",
        Key::ErrCommandFailed => "Command failed: {}",
        Key::ErrKeySetupConnectionFailed => "Connection failed: {}",
        Key::ErrKeySetupRolledBack => "Rolled back.",
        Key::ErrKeySetupUnknownFailure => "Unknown failure.",
        Key::SnippetFormHost => "Host (if scope=host)",
        Key::TerminalEndLine => {
            "\r\n\x1b[2m[Connection closed. Press Enter to close this tab.]\x1b[0m\
             \x1b[?25l\x1b[?9;1000;1002;1003l"
        }
        Key::StatusLanguageSwitched => "Language: {}",
    }
}

fn zh_hans(key: Key) -> &'static str {
    match key {
        Key::TerminalTooSmall => "终端窗口过小 — 请将窗口调整到至少 80×24。",
        Key::NavDashboard => "仪表盘",
        Key::NavFiles => "文件",
        Key::NavSnippets => "代码片段",
        Key::NavTerminal => "终端",
        Key::NavHelp => "帮助",
        Key::NavQuit => "退出",
        Key::SearchBadge => "[搜索]",
        Key::SearchTypeToFilter => "输入以筛选",
        Key::HintConfirm => "确认",
        Key::HintClear => "清除",
        Key::HintCancel => "取消",
        Key::HintClose => "关闭",
        Key::HintSave => "保存",
        Key::HintSelect => "选择",
        Key::HintScroll => "滚动",
        Key::HintRun => "运行",
        Key::HintConnect => "连接",
        Key::HintNavigate => "导航",
        Key::HintToggle => "切换",
        Key::HintNextField => "下一字段",
        Key::HintPrevField => "上一字段",
        Key::HintNextParam => "下一参数",
        Key::HintConfirmDelete => "确认删除",
        Key::HintConfirmSetup => "确认设置",
        Key::SettingUpKeys => "正在设置 SSH 密钥…",
        Key::HintTransferring => "正在传输",
        Key::FormName => "名称",
        Key::FormHostname => "主机名 / IP",
        Key::FormUser => "用户",
        Key::FormPort => "端口",
        Key::FormIdentityFile => "身份文件",
        Key::FormPassword => "密码（登录用，可选）",
        Key::FormTags => "标签（逗号分隔）",
        Key::FormNotes => "备注",
        Key::FormMonitoring => "监控方式（ssh | tcp | tcp:端口）",
        Key::FormForwards => "端口转发（端口:主机:目标端口, ...）",
        Key::FormStartTunnel => "启动时开启隧道（y/n）",
        Key::FormForwardAgent => "转发 SSH agent（y/n）",
        Key::FormForwardAgentWindows => "转发 SSH agent（y/n，Windows 暂不支持）",
        Key::SnippetFormName => "名称",
        Key::SnippetFormCommand => "命令",
        Key::SnippetFormDescription => "描述",
        Key::SnippetFormTags => "标签（逗号分隔）",
        Key::SnippetFormParams => "参数（逗号分隔）",
        Key::SnippetFormScope => "作用范围（global | host）",
        Key::SortName => "名称",
        Key::SortCpu => "cpu",
        Key::SortRam => "内存",
        Key::SortStatus => "状态",
        Key::HelpTitle => " 帮助 — 键盘快捷键 ",
        Key::HelpSectionGlobalNav => " 全局导航",
        Key::HelpSectionDashboard => " 仪表盘",
        Key::HelpSectionDetailView => " 详情视图",
        Key::HelpSectionFileManager => " 文件管理器",
        Key::HelpSectionSnippets => " 代码片段",
        Key::HelpSectionTerminal => " 终端",
        Key::HelpSectionCopyText => " 复制文本",
        Key::HelpFooterClose => " 按 Esc 或 ? 关闭",
        Key::HelpTerminalOnly => "  （仅终端界面）",
        Key::HelpOpenDetails => "    打开详情",
        Key::HelpAddHost => "        添加主机",
        Key::HelpEditHost => "        编辑主机",
        Key::HelpDeleteHost => "        删除主机",
        Key::HelpRefresh => "        刷新",
        Key::HelpSort => "        排序",
        Key::HelpFilterTags => "        按标签筛选",
        Key::HelpGroupByTag => "        按标签分组",
        Key::HelpSearch => "        搜索",
        Key::HelpQuickExec => "        快速执行",
        Key::HelpSetupKey => "        设置 SSH 密钥",
        Key::HelpTunnelToggle => "        隧道开/关",
        Key::HelpNavigate => "    导航",
        Key::HelpConnect => "    连接",
        Key::HelpBack => "      返回",
        Key::HelpQuickView => "      快速查看",
        Key::HelpSwitchPanel => "      切换面板",
        Key::HelpMarkFile => "    标记文件",
        Key::HelpCopy => "        复制",
        Key::HelpPaste => "        粘贴",
        Key::HelpNewDir => "        新建目录",
        Key::HelpRename => "        重命名",
        Key::HelpDelete => "        删除",
        Key::HelpToggleHidden => "        显示/隐藏隐藏文件",
        Key::HelpNextDrive => "        下一个驱动器",
        Key::HelpRunSnippet => "    运行片段",
        Key::HelpNew => "        新建",
        Key::HelpEdit => "        编辑",
        Key::HelpBroadcast => "        广播",
        Key::HelpNewTab => "   新建标签页",
        Key::HelpCloseTab => "   关闭标签页",
        Key::HelpNextTab => "   下一个标签页",
        Key::HelpVSplit => "   垂直分屏",
        Key::HelpHSplit => "   水平分屏",
        Key::HelpExit => "   退出",
        Key::HelpSelectText => " 选择文本",
        Key::DashboardTitle => "仪表盘",
        Key::DashboardHints => "r:刷新  s:排序  t:标签  g:分组  /:搜索  a:添加  x:执行  f:隧道",
        Key::DashboardHintKeySetup => "  Shift+K:设置密钥",
        Key::DashboardEmptyNoHosts => {
            "\n  尚未配置任何主机。\n\n  按  a  添加主机，或按  r  从 ~/.ssh/config 重新加载。"
        }
        Key::DashboardEmptyNoMatch => "\n  没有主机匹配当前筛选条件。",
        Key::DashboardSort => "排序",
        Key::DashboardFilter => "筛选",
        Key::DashboardSearch => "搜索",
        Key::Untagged => "未分组",
        Key::CardReachReachable => "─── 可达 ───",
        Key::CardReachReachablePort => "─── 可达 :{} ───",
        Key::CardReachUnreachable => "─── 不可达 ───",
        Key::CardReachUnreachablePort => "─── 不可达 :{} ───",
        Key::CardReachChecking => "─── 检测中 ───",
        Key::CardReachCheckingPort => "─── 检测中 :{} ───",
        Key::CardOffline => "─── 离线 ───",
        Key::CardUptime => "运行: {}",
        Key::CardDockerRunning => "{} 个运行中",
        Key::CardDockerStopped => "{} 个已停止",
        Key::CardDockerRestarting => "{} 个重启中",
        Key::CardDockerNoContainers => "无容器",
        Key::CardPgReplLag => "复制延迟 {}s",
        Key::CardNginxErrors => "{} 个错误/5分钟",
        Key::CardRedisMemUsed => "已用 {}MB",
        Key::CardNodeProcesses => "{} 个进程",
        Key::CardNodeNoProcesses => "无进程",
        Key::CardServiceOk => "正常",
        Key::DetailTooSmall => "终端窗口过小，无法显示详情视图。（最小 60x20）",
        Key::DetailNoHostSelected => "未选择主机。",
        Key::DetailNoDiscovery => "  服务\n\n  暂无发现数据。按 'r' 刷新。",
        Key::DetailStatusConnected => "已连接",
        Key::DetailStatusConnecting => "连接中",
        Key::DetailStatusUnknown => "未知",
        Key::DetailStatusFailed => "失败：{}",
        Key::DetailUptimeUnknown => "未知",
        Key::DetailLabelHost => " 主机: ",
        Key::DetailLabelUser => "用户: ",
        Key::DetailLabelUp => "运行: ",
        Key::DetailLabelStatus => "状态: ",
        Key::DetailLabelOs => " 系统: ",
        Key::DetailOsPending => "（发现中）",
        Key::DetailLabelTunnel => "   隧道: ",
        Key::DetailTunnelUp => "已开启",
        Key::DetailTunnelConnecting => "连接中",
        Key::DetailTunnelRetrying => "重试中：{}",
        Key::DetailTunnelFailed => "失败：{}",
        Key::DetailTunnelOff => "已关闭",
        Key::DetailMetricsTitle => " 指标",
        Key::DetailMetricsUnavailable => " 暂不可用",
        Key::DetailLabelLoad => " 负载: ",
        Key::DetailNoMetrics => " （暂无可用指标）",
        Key::DetailTopProcessesTitle => " 占用最高的进程",
        Key::DetailProcCpu => "  CPU ",
        Key::DetailProcMem => "   内存 ",
        Key::DetailProcUnavailable => " （暂无进程数据）",
        Key::DetailServicesTitle => " 服务",
        Key::DetailNoServices => " 未检测到服务",
        Key::DetailDockerRunningStopped => "{} 个运行中，{} 个已停止",
        Key::DetailDockerRunning => "{} 个容器运行中",
        Key::DetailPgReplLag => "复制延迟：{}s",
        Key::DetailPgNoReplLag => "运行中，无复制延迟",
        Key::DetailNginxErrors => "运行中，{} 个错误/5分钟",
        Key::DetailNginxNoErrors => "运行中，0 个错误/5分钟",
        Key::DetailRedisMem => "运行中，占用 {}MB 内存",
        Key::DetailActive => "运行中",
        Key::DetailNodeProcesses => "{} 个进程运行中",
        Key::DetailNodeNoProcesses => "无进程",
        Key::DetailHintConnect => ":连接",
        Key::DetailHintRefresh => ":刷新",
        Key::DetailHintDashboard => ":← 仪表盘",
        Key::DetailHintQuickView => ":快速查看",
        Key::DetailHintTunnel => ":隧道",
        Key::FmRemoteConnected => "远程  {} — {}",
        Key::FmRemoteNotConnected => "远程  （未连接）",
        Key::FmLocal => "本地  {}",
        Key::FmLoading => "（加载中…）",
        Key::FmEmpty => "（空）",
        Key::FmPreviewTitle => " 预览 ",
        Key::FmNoPreview => "  （无预览 — 请选择一个文本文件）",
        Key::FmTransferProgressTitle => " 传输进度 ",
        Key::FmTransferredUnknown => "  已传输 {}…",
        Key::FmCancelling => "  正在取消…",
        Key::FmNewDirectory => " 新建目录 ",
        Key::FmRenameTitle => " 重命名 '{}' ",
        Key::FmHostPickerTitle => " 连接远程面板 — 选择主机 ",
        Key::FmHostPickerEmpty => "  尚未配置任何主机。请从仪表盘 (1) 添加主机。",
        Key::FmHintNavigate => ":导航  ",
        Key::FmHintConnect => ":连接  ",
        Key::FmHintConfirm => ":确认  ",
        Key::FmHintNavigateHeader => ":导航",
        Key::FmHintSwitch => ":切换",
        Key::FmHintMark => ":标记",
        Key::FmHintCopy => ":复制",
        Key::FmHintPaste => ":粘贴",
        Key::FmHintMkDir => ":新建目录",
        Key::FmHintRename => ":重命名",
        Key::FmHintDelete => ":删除",
        Key::FmHintHidden => ":隐藏文件",
        Key::FmHintHost => ":主机",
        Key::FmHintDrive => ":驱动器",
        Key::FmConfirmDeleteTitle => " 确认删除 ",
        Key::FmDeletePrompt => "  要删除以下项目吗？",
        Key::FmDeleteMore => "    … 还有 {} 项",
        Key::FmDeleteIrreversible => "  此操作无法撤销。",
        Key::FmDeleteYes => ":是 — 删除  ",
        Key::FmDeleteNo => ":否",
        Key::SnippetsTooSmall => "终端窗口过小，无法显示代码片段界面。",
        Key::SnippetsAddTitle => "添加代码片段",
        Key::SnippetsEditTitle => "编辑代码片段",
        Key::SnippetsHeaderTitle => " 代码片段 ({}) ",
        Key::SnippetsSearchTag => "[搜索: {}] ",
        Key::SnippetsFilterTag => "[筛选: {}] ",
        Key::SnippetsHintBar => "  n:新建  e:编辑  d:删除  Enter:运行  b:广播  /:搜索",
        Key::SnippetsNoMatch => "  没有匹配的代码片段。",
        Key::SnippetsEmpty => "  暂无代码片段。按  n  创建你的第一个代码片段。",
        Key::PopupAddHostTitle => "添加主机",
        Key::PopupEditHostTitle => "编辑主机",
        Key::PopupFormMoreAbove => "  ↑ 还有 {} 项",
        Key::PopupFormMoreBelow => "  ↓ 还有 {} 项",
        Key::PopupFilterByTagActive => " 按标签筛选 [{}] ",
        Key::PopupFilterByTag => " 按标签筛选 ",
        Key::PopupTagAll => "全部（清除筛选）",
        Key::PopupConfirmDeleteHostTitle => " 确认删除 ",
        Key::PopupDeleteHostPrompt => "  要删除主机 '{}' 吗？",
        Key::PopupDeleteHostWarning => "  此操作无法撤销。  ",
        Key::PopupConfirmDeleteSnippetTitle => " 确认删除代码片段 ",
        Key::PopupDeleteSnippetPrompt => "  要删除代码片段 '{}' 吗？",
        Key::PopupDeleteSnippetWarning => "  此操作无法撤销。",
        Key::PopupParamsTitle => " 参数 — {} ",
        Key::PopupBroadcastTitle => " 广播 — 选择主机 ",
        Key::PopupUnlockKeyTitle => " 解锁 SSH 密钥 — {} ",
        Key::PopupPassphraseLabel => "  密码短语（保留在内存中，直到 OmnySSH 退出）：",
        Key::PopupSshLoginTitle => " SSH 登录 — {} ",
        Key::PopupPasswordLabel => "  密码（保留在内存中，直到 OmnySSH 退出）：",
        Key::PopupRememberPassword => "记住密码，下次免输入",
        Key::PopupUnlocking => "  正在解锁…",
        Key::PopupPermissionDenied => "权限被拒绝，请重试。",
        Key::PopupNewHostKeyRecorded => "已记录新的主机密钥：{}",
        Key::PopupQuickExecuteTitle => " 快速执行 — {} ",
        Key::PopupResultsTitle => " 结果 ",
        Key::PopupResultRunning => " {} 运行中… ",
        Key::PopupResultDone => " ✓ 完成 ",
        Key::PopupResultError => " ✗ 错误 ",
        Key::PopupResultNoOutput => "（无输出）",
        Key::PopupResultWaiting => "  正在等待结果…",
        Key::PopupUpdateNow => "立即更新",
        Key::PopupUpdateRemindLater => "稍后提醒",
        Key::PopupUpdateSkip => "跳过此版本",
        Key::PopupUpdateDisable => "不再检查",
        Key::PopupUpdateAvailableTitle => " 有可用更新 ",
        Key::PopupUpdateVersionLabel => "  版本  ",
        Key::PopupUpdateSelfUpdate => "  此更新现在即可下载并安装。",
        Key::PopupUpdateChecksum => "  安装前会校验压缩包的校验和。",
        Key::PopupUpdatePackageManager => "  请使用包管理器更新：",
        Key::PopupUpdateDownloadFrom => "  请从以下地址下载新版本：",
        Key::PopupUpdateInstalling => "  {} 正在下载并安装 — 请稍候…",
        Key::PopupPleaseWait => "  请稍候…",
        Key::StatusReloadingHosts => "正在重新加载主机…",
        Key::StatusRefreshingMetrics => "正在刷新指标…",
        Key::StatusConnectedTo => "已连接到 '{}'",
        Key::StatusConnectionFailed => "'{}' 连接失败：{}",
        Key::StatusDiscoveryFailed => "'{}' 服务发现失败：{}",
        Key::StatusTunnelUp => "'{}' 的隧道已开启",
        Key::StatusTunnelFailed => "'{}' 的隧道失败：{}",
        Key::StatusTunnelStopped => "'{}' 的隧道已停止",
        Key::StatusTunnelStarting => "正在为 '{}' 启动隧道…",
        Key::StatusNoPortForwards => "'{}' 未设置端口转发 — 按 e 添加",
        Key::StatusKeySetupComplete => "✓ '{}' 的 SSH 密钥设置完成。密钥：{}",
        Key::StatusKeySetupPartial => {
            "⚠ '{}' 的密钥登录可用，但密码登录仍开启（需要免密码 sudo）。密钥：{}"
        }
        Key::StatusKeySetupFailedUnsafe => "✗ '{}' 的密钥设置失败，且密码登录可能已关闭：{}",
        Key::StatusKeySetupFailed => "✗ '{}' 的密钥设置失败：{}",
        Key::StatusKeySetupRolledBack => "⚠ '{}' 的密钥设置已回滚：{}",
        Key::StatusKeyPassphraseNeeded => "SSH 密钥需要密码短语 — 按 Ctrl+Q 输入：{}",
        Key::StatusPasswordNeeded => "{} 需要密码 — 按 Ctrl+Q 输入",
        Key::StatusUnlocked => "已解锁 {}",
        Key::StatusAlreadyUsesKeyAuth => "'{}' 已使用密钥认证。",
        Key::StatusNoPasswordForKeySetup => "此主机未设置密码。请先添加密码以启用密钥设置。",
        Key::StatusNoLongerWaitingPassword => "{} 已不再等待输入密码",
        Key::StatusNoHostSelected => "未选择主机。",
        Key::StatusSftp => "SFTP：{}",
        Key::StatusUpdateInstalled => "更新已安装。重启 omny 以使用新版本。",
        Key::StatusUpdateFailed => "更新失败：{}",
        Key::StatusItemsRemaining => "还剩 {} 项…",
        Key::StatusTransferFailed => "传输失败：{}",
        Key::StatusTransferFailedMore => "传输失败：{}（另有 {} 项失败）",
        Key::StatusTransferFinishedNotCancelled => "传输在取消之前已完成。",
        Key::StatusSnippetFailed => "代码片段 '{}' 在 '{}' 上失败：{}",
        Key::StatusNothingToCopy => "没有可复制的内容。",
        Key::StatusCopiedToClipboard => "已复制到剪贴板。切换面板后按 p 粘贴。",
        Key::StatusNothingToDelete => "没有可删除的内容。",
        Key::StatusCancelling => "正在取消…",
        Key::StatusNeedTwoTabsToSplit => "至少需要 2 个标签页才能分屏。按 Ctrl+T 新建标签页。",
        Key::StatusNoSuchHost => "主机不存在。",
        Key::StatusSshSessionClosed => "SSH 会话已关闭。",
        Key::StatusPtyError => "PTY 错误：{}",
        Key::StatusHostAdded => "主机已添加。",
        Key::StatusHostAddedWithPassword => {
            "主机 '{}' 已添加。按 'Shift+K' 设置 SSH 密钥认证（推荐）。"
        }
        Key::StatusHostUpdated => "主机已更新。",
        Key::StatusHostDeleted => "已删除 '{}'。",
        Key::StatusSaveFailed => "保存失败：{}",
        Key::StatusError => "错误：{}",
        Key::StatusSnippetAdded => "代码片段已添加。",
        Key::StatusSnippetUpdated => "代码片段已更新。",
        Key::StatusSnippetDeleted => "已删除代码片段 '{}'。",
        Key::StatusNoHostsSelected => "未选择任何主机。",
        Key::StatusFmHostNotFound => "主机不存在。",
        Key::StatusFmConnecting => "正在连接到 '{}'…",
        Key::StatusFmNoOtherDrives => "没有其他驱动器",
        Key::StatusFmNothingInClipboard => "剪贴板为空。",
        Key::StatusFmCannotPasteSamePanel => "无法粘贴到同一个面板。",
        Key::StatusFmClipboardEmpty => "剪贴板为空。",
        Key::StatusFmNotConnected => "未连接到主机。",
        Key::StatusFmQueuedItems => "已排队 {} 项等待传输…",
        Key::StatusFmWaitForRunning => "请等待当前操作完成。",
        Key::ErrNameEmpty => "名称不能为空",
        Key::ErrHostnameEmpty => "主机名 / IP 不能为空",
        Key::ErrPortInvalid => "端口必须是 1 到 65535 之间的数字，收到 '{}'",
        Key::ErrMonitoringInvalid => "监控方式必须是 'ssh'、'tcp' 或 'tcp:PORT'，收到 '{}'",
        Key::ErrPortForward => "端口转发：{}",
        Key::ErrPortForwardDupPort => "端口转发：有两条规则监听端口 {}",
        Key::ErrYesNoInvalid => "{} 必须是 y 或 n，收到 '{}'",
        Key::ErrLabelStartTunnel => "启动时开启隧道",
        Key::ErrLabelForwardAgent => "转发 SSH agent",
        Key::ErrSnippetCommandEmpty => "命令不能为空",
        Key::ErrSnippetHostEmpty => "作用范围为 'host' 时主机不能为空",
        Key::StatusHostNotFound => "找不到主机 '{}'。",
        Key::ErrConnectFailed => "连接失败：{}",
        Key::ErrCommandFailed => "命令执行失败：{}",
        Key::ErrKeySetupConnectionFailed => "连接失败：{}",
        Key::ErrKeySetupRolledBack => "已回滚。",
        Key::ErrKeySetupUnknownFailure => "未知错误。",
        Key::SnippetFormHost => "主机（作用范围为 host 时）",
        Key::TerminalEndLine => {
            "\r\n\x1b[2m[连接已关闭。按回车关闭此标签页。]\x1b[0m\
             \x1b[?25l\x1b[?9;1000;1002;1003l"
        }
        Key::StatusLanguageSwitched => "语言：{}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_translated_in_every_language() {
        for key in Key::ALL {
            for lang in Lang::all() {
                assert!(
                    !t(*lang, *key).trim().is_empty(),
                    "{:?} is empty for {:?}",
                    key,
                    lang
                );
            }
        }
    }

    #[test]
    fn no_key_leaks_its_variant_name() {
        // A `TODO`-style arm that forgot its text would surface as the Debug
        // name; catch that here rather than in a screenshot. Two keys carry a
        // real English string that happens to spell their variant name, so they
        // are listed explicitly rather than skipped wholesale.
        let spells_its_name = [Key::Untagged, Key::HelpNewTab, Key::HelpCloseTab];
        for key in Key::ALL {
            for lang in Lang::all() {
                let text = t(*lang, *key);
                let leaks = text.starts_with("Key::")
                    || (text.contains(&format!("{key:?}")) && !spells_its_name.contains(key));
                assert!(!leaks, "key {:?} leaked its variant name: {text:?}", key);
            }
        }
    }

    #[test]
    fn english_and_chinese_differ_for_real_prose() {
        // Guard against a copy-paste arm that was never translated. Keys whose
        // text is a proper noun or an abbreviation are identical on purpose.
        let same_on_purpose = [Key::SortCpu, Key::SortRam];
        for key in Key::ALL {
            if same_on_purpose.contains(key) {
                continue;
            }
            assert_ne!(
                t(Lang::En, *key),
                t(Lang::ZhHans, *key),
                "{:?} is identical in both languages",
                key
            );
        }
    }

    #[test]
    fn tf_fills_placeholders_in_order() {
        // Placeholder-free keys pass through untouched.
        assert_eq!(tf(Lang::En, Key::NavDashboard, &[]), "Dashboard");
        // A key with no `{}` ignores its args rather than panicking.
        assert_eq!(tf(Lang::En, Key::NavFiles, &["x"]), "Files");
    }
}
