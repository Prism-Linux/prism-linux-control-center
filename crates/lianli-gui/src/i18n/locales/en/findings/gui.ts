export default {
  hostUnavailable: {
    title: "Host integration is unavailable",
    remediation:
      "Use a native installation or restore Distrobox's /run/host integration. The daemon cannot safely start with a private container lock.",
  },
  desktopStartupFound: {
    title: "Packaged desktop login startup files found",
    remediation:
      "Check lianli-session.service or follow the desktop startup guide for source builds. This helper is needed only for Desktop mode.",
  },
  desktopStartupMissing: {
    title: "Automatic desktop login startup is not established",
    remediation:
      "Check lianli-session.service or follow the desktop startup guide for source builds. This helper is needed only for Desktop mode.",
  },
  usbRules: {
    title: "Packaged USB permission rules",
    remediation:
      "Install the current USB rules on the host, reload udev rules and reapply them. Review intentional custom rules before replacing them. File checks alone do not verify device permissions.",
  },
  usbRulesCustomAccess: {
    title: "Packaged USB permission rules",
    remediation:
      "No rule replacement is required. Keep working custom rules. Check daemon logs for open failures and Recheck after setup changes.",
  },
  hardwareOwnerUnreachable: {
    title: "Hardware owner unreachable",
    remediation:
      "Wait for startup or switching to finish, then Recheck. If this persists, check daemon logs and socket visibility. For a Distrobox daemon, launch the GUI in the same box or expose the host runtime socket.",
  },
  daemonConnectionUnavailable: {
    title: "Daemon connection unavailable",
    remediation:
      "Choose one service in Settings or start your manual daemon, then Recheck. For Distrobox, the GUI and daemon must share a visible runtime socket.",
  },
  runtimeChecksUnavailable: {
    title: "Runtime installation checks unavailable",
    remediation:
      "Install the matching lianli-control beside the GUI and daemon, start the selected service, then Recheck. See the troubleshooting guide for source builds and offline diagnostics.",
  },
  daemonLockIdentity: {
    title: "Connected daemon lock identity",
    remediation:
      "Update all daemon launch routes, stop the old owner cleanly and restart the selected service. Do not delete a live lock file or start another daemon to repair this.",
  },
  ownershipLock: {
    title: "Shared daemon ownership lock",
    remediation:
      "Install the supplied tmpfiles rule on the host and run sudo systemd-tmpfiles --create lianli.conf. Never delete or replace a lock while a daemon is running.",
  },
};
