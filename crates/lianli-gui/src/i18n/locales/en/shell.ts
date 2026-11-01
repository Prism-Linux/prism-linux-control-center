export default {
  header: {
    switchToLight: "Switch to light mode",
    switchToDark: "Switch to dark mode",
    refreshing: "Refreshing…",
    refreshNow: "Refresh now",
    saveFailed: "Save failed: {error}",
    minimizeWindow: "Minimize window",
    toggleMaximizeWindow: "Maximize or restore window",
    closeWindow: "Close window",
  },
  sidebar: {
    connected: "Connected",
    lastSeen: "Last seen: {summary}",
  },
  installationNotice: {
    title: "Check your installation",
    intro: "Some setup checks need attention. You can review these at any time in Installation Health.",
    waitingTitle: "Waiting for daemon",
    waitingBody: "Open Settings to manage services. If the daemon is starting, wait and Recheck.",
    allPass: "These installation checks now pass.",
    later: "Later",
    recheck: "Recheck",
    openGuide: "Open guide",
    reviewChecks: "Review checks",
  },
  updateNotice: {
    title: "Prism Linux Control Center {version} is available",
    releaseNotes: "View release notes",
  },
  wirelessNotice: {
    transmitterTitle: "Wireless transmitter stopped responding",
    transmitterBody:
      "Unplug the wireless transmitter and plug it back in. Software resets did not restart it. Wireless fans and pumps keep their last speed until it responds.",
    receiverTitle: "Wireless receiver stopped responding",
    receiverBody:
      "Unplug the wireless receiver and plug it back in. Software resets did not restart it. Wireless fans and pumps keep their last speed until it responds.",
  },
  compatibilityNotice: {
    title: "Changes disabled",
  },
  editableName: {
    renameDevice: "Rename this device",
    deviceName: "Device name",
  },
  externalLink: {
    openFailedCopied: "Could not open the link: {error}. The link was copied to the clipboard.",
    openFailed: "Could not open the link: {error}. {url}",
  },
  download: {
    timedOut: "Download timed out. Check your connection and retry",
    tooLarge: "Download exceeds {limit} bytes",
    noBody: "Download body is unavailable",
  },
  config: {
    copiedTemplateChanged: "A copied template changed in another window. Reload and review the import before saving.",
    lcdNotConfirmed: "Copied templates were saved, but LCD settings were not confirmed. Retry Save or reload to inspect settings. {error}",
    previousImportPending: "Save or reload the previous copied selection first.",
    reloadFailed: "The configuration was saved, but it could not be reloaded to confirm the result: {error}",
    reloadEmpty: "The configuration save returned no configuration to reload. The result could not be confirmed.",
  },
};
