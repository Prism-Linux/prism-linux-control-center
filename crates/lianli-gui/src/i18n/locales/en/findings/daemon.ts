export default {
  wiredIdentitySave: {
    title: "Wired identity migration could not be saved",
    remediation:
      "Repair the settings or restore a backup, then save or restart the service. Recheck shows the last load result.",
  },
  wiredIdentity: {
    title: "Wired device settings need attention",
    remediation:
      "Repair the settings or restore a backup, then save or restart the service. Recheck shows the last load result.",
  },
  configurationLoad: {
    title: "Configuration load",
    remediation:
      "Repair the settings or restore a backup, then save or restart the service. Recheck shows the last load result.",
  },
  templateLoad: {
    title: "Custom template load",
    remediation:
      "Repair the settings or restore a backup, then save or restart the service. Recheck shows the last load result.",
  },
  wirelessInitialization: {
    title: "Wireless initialization failed",
    remediation:
      "Check the transmitter and receiver connections, daemon logs and USB permissions. Wireless fans and pumps keep their last speed until the link recovers. The daemon retries and resets unresponsive dongles automatically. Recheck only reads status.",
  },
  wirelessTransmitterUnresponsive: {
    title: "Wireless transmitter stopped responding",
    remediation:
      "Unplug the wireless transmitter and plug it back in. Software resets did not restart it. Wireless fans and pumps keep their last speed until it responds.",
  },
  wirelessReceiverUnresponsive: {
    title: "Wireless receiver stopped responding",
    remediation:
      "Unplug the wireless receiver and plug it back in. Software resets did not restart it. Wireless fans and pumps keep their last speed until it responds.",
  },
  deviceOpenFailed: {
    title: "Device could not open",
    remediation:
      "Check the connection, USB permissions and daemon logs. After repairs, reconnect the device or restart its service if retries have stopped. Recheck reads status only.",
  },
  deviceOpenMore: {
    title: "Additional device open failures",
    remediation: "Inspect daemon logs for the remaining failures.",
  },
  lcdInitializationFailed: {
    title: "LCD initialization failed",
    remediation:
      "Check USB access and daemon logs. Reconnect the LCD or restart the service after repairs. Recheck only reads status.",
  },
  lcdInitializationMore: {
    title: "Additional LCD initialization failures",
    remediation: "Inspect daemon logs for the remaining failures.",
  },
  coolingFallback: {
    title: "Cooling fallback active",
    remediation: "Check temperature sources in Fans and AIO settings, then Recheck.",
  },
  coolingTemperatures: {
    title: "Cooling temperatures",
  },
  openrgbServer: {
    title: "OpenRGB server",
    remediation:
      "Free the configured TCP port or fix access in the daemon's network namespace. Use Retry OpenRGB in Settings to retry saved settings, or edit the port and Save. Recheck only reads status.",
  },
  runtimeHelperUnavailable: {
    title: "Runtime helper unavailable",
    remediation:
      "Install or rebuild the matching lianli-control beside the daemon. Repair unavailable storage or account services, then Recheck.",
  },
};
