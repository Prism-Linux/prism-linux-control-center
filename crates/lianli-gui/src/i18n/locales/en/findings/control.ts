const lingeringTitle = "Distrobox system startup";
const moduleTitle = "Optional display module: {module}";
const lockTitle = "Process access to the host ownership lock";
const groupCredentialsTitle = "Numeric group credentials";
const groupMembershipStaleTitle = "Group membership changed after this process started";
const groupMissingTitle = "The process lacks the host lock's numeric group";
const lockRepair =
  "Install the supplied host tmpfiles rule and run sudo systemd-tmpfiles --create lianli.conf. For user service setup, add your user to the host lianli group, log out and back in, then restart the selected service. A system service must use the packaged lianli account/group. Do not replace an active lock.";
const lockRepairDistrobox =
  "Apply the host tmpfiles/group setup, log out and back in, then stop and re-enter the existing box and restart its user service. Compare numeric group IDs visible inside the box. Matching names alone are insufficient. Do not create a private lock or install host rules only inside the box.";
const evdiLibraryTitle = "EVDI userspace library";
const hermesAccessTitle = "Hermes host render access";
const loginStartupLogs =
  "Inspect host logs with journalctl --user -u lianli-session.service -b -n 100 --no-pager. This check does not start or change services.";
const loginStartupRepair = `Inside box '{name}', run: lianli-control distrobox-service-unit --desktop-session --box '{name}'. Review the output and install it as ~/.config/systemd/user/lianli-session.service on the host. On the host run systemctl --user daemon-reload, then systemctl --user enable --now lianli-session.service. Keep the host process and private user runtime views shared with the box. See the Distrobox guide for custom paths. This capture helper does not start the hardware daemon. ${loginStartupLogs}`;
const loginStartupInvalidBoxRepair = `Invalid detected Distrobox name. Follow the Distrobox desktop startup guide. ${loginStartupLogs}`;
const recipeTitle = "Distrobox service controls need the current service recipe";
const hardwareServiceRepair =
  "Check the host service setup and journal. Install the supplied units, then select one hardware mode. Do not start another daemon while an existing owner is running.";
const storageRepair =
  "Check the daemon account's directory access, mount state, free space and quotas in its filesystem namespace. Create a missing state directory with the intended daemon ownership using the installation guide. Repair storage, Recheck, then retry the failed save or media preparation.";

export default {
  lingeringGuidance: {
    title: lingeringTitle,
    remediation:
      "On the host, run sudo loginctl enable-linger {uid}, then Recheck. Lingering allows this account's user services to run before login and after logout. Lian Li Linux does not change it automatically.",
  },
  lingeringAccountUnknown: {
    title: lingeringTitle,
    remediation: "Verify the host system unit's account, then follow the Distrobox lingering guide.",
  },
  mediaTool: {
    title: "{tool} availability",
    remediation:
      "Install FFmpeg with ffprobe and libx264 in the daemon's environment (inside its Distrobox when applicable). Verify the service PATH, then Recheck and save the failed media settings to retry. No GPU encoder is required.",
  },
  moduleUnavailable: {
    title: moduleTitle,
    remediation:
      "Check the host installation using the desktop backend guide. Hyprland needs no optional display module.",
  },
  moduleLoaded: {
    title: moduleTitle,
  },
  moduleLoadFailed: {
    title: moduleTitle,
    remediation:
      "Review the host kernel log, then repair module compatibility or signing. Retry desktop mode after repair.",
  },
  moduleBuildFilesMissing: {
    title: moduleTitle,
    remediation:
      "Matching host kernel build files are missing. Install the kernel development package, then repair the DKMS build using the host package instructions.",
  },
  moduleBuildIncomplete: {
    title: moduleTitle,
    remediation: "Review the host DKMS build log and finish installing the module for the running kernel.",
  },
  moduleInstalled: {
    title: moduleTitle,
    remediation: "If this backend is needed, follow the host driver startup instructions.",
  },
  moduleNotInstalled: {
    title: moduleTitle,
    remediation:
      "Install this host driver only if you need its desktop backend. Hyprland uses native headless output.",
  },
  lockAccess: { title: lockTitle, remediation: lockRepair },
  lockAccessDistrobox: { title: lockTitle, remediation: lockRepairDistrobox },
  groupCredentials: { title: groupCredentialsTitle, remediation: lockRepair },
  groupCredentialsDistrobox: { title: groupCredentialsTitle, remediation: lockRepairDistrobox },
  groupMembershipStale: { title: groupMembershipStaleTitle, remediation: lockRepair },
  groupMembershipStaleDistrobox: { title: groupMembershipStaleTitle, remediation: lockRepairDistrobox },
  groupMissing: { title: groupMissingTitle, remediation: lockRepair },
  groupMissingDistrobox: { title: groupMissingTitle, remediation: lockRepairDistrobox },
  evdiLibraryLoaded: {
    title: evdiLibraryTitle,
  },
  evdiLibraryMissing: {
    title: evdiLibraryTitle,
    remediation:
      "Install libevdi 1.14 or 1.15 where the daemon runs: lian-li-linux-evdi on Fedora, libevdi0 on Debian or Ubuntu, or evdi-dkms on Arch. Then restart the daemon service.",
  },
  evdiLibraryMissingDistrobox: {
    title: evdiLibraryTitle,
    remediation:
      "Install the library inside the box with distrobox-enter --name {name} -- sudo dnf install --setopt=install_weak_deps=False lian-li-linux-evdi, or libevdi0 on a Debian or Ubuntu box, then restart the daemon service. The host library is not visible inside the box.",
  },
  hermesAccessGranted: {
    title: hermesAccessTitle,
  },
  hermesAccess: {
    title: hermesAccessTitle,
    remediation:
      "Install the current 60-lianli.rules on the host, run sudo udevadm control --reload-rules, then reboot. It includes the Hermes host ACL fix. Distrobox-only installation is insufficient. If access still fails, follow the Hermes permissions section in the setup guide.",
  },
  loginStartupUnavailable: {
    title: "Desktop login startup could not be inspected",
    remediation: loginStartupRepair,
  },
  loginStartupNotInstalled: {
    title: "Optional desktop login service is not installed",
    remediation: loginStartupRepair,
  },
  loginStartupDisabled: {
    title: "Optional desktop login startup is disabled",
    remediation: loginStartupRepair,
  },
  loginStartupMissing: {
    title: "Desktop login service is missing or unavailable",
    remediation: loginStartupRepair,
  },
  loginStartupOutdatedRecipe: {
    title: "Desktop login service needs the current recipe",
    remediation: loginStartupRepair,
  },
  loginStartupNotEnabled: {
    title: "Desktop login service is not enabled persistently",
    remediation: loginStartupRepair,
  },
  loginStartupFailed: {
    title: "Desktop login service failed",
    remediation: loginStartupRepair,
  },
  loginStartupConfigured: {
    title: "Desktop login startup is configured",
    remediation: loginStartupRepair,
  },
  loginStartupUnavailableInvalidBox: {
    title: "Desktop login startup could not be inspected",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupNotInstalledInvalidBox: {
    title: "Optional desktop login service is not installed",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupDisabledInvalidBox: {
    title: "Optional desktop login startup is disabled",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupMissingInvalidBox: {
    title: "Desktop login service is missing or unavailable",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupOutdatedRecipeInvalidBox: {
    title: "Desktop login service needs the current recipe",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupNotEnabledInvalidBox: {
    title: "Desktop login service is not enabled persistently",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupFailedInvalidBox: {
    title: "Desktop login service failed",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupConfiguredInvalidBox: {
    title: "Desktop login startup is configured",
    remediation: loginStartupInvalidBoxRepair,
  },
  switchPrerequisites: {
    title: "Service switching needs host support",
    remediation:
      "Install the matching host control helper, systemd-run and Polkit. For Distrobox, use Set up host support in Settings. Run a desktop authentication agent, then Recheck.",
  },
  serviceSelection: {
    title: "Host service selection is unverified",
    remediation:
      "Resolve any unfinished service switch before starting a daemon. For invalid or hidden selection records, follow the Service modes guide and preserve recovery files. Do not create a private container selection file.",
  },
  distroboxRecipe: {
    title: recipeTitle,
    remediation:
      "Inside box '{name}', run: lianli-control distrobox-service-unit --box '{name}'. It prints the host user unit without installing or starting it. For custom paths, pass --distrobox-enter with the host executable path and --binaries with the guest binary directory. Review the output, stop any older daemon cleanly, then install it as ~/.config/systemd/user/lianli-daemon.service on the host and run systemctl --user daemon-reload there. Follow the Distrobox guide before enabling it.",
  },
  distroboxRecipeInvalidBox: {
    title: recipeTitle,
    remediation:
      "Invalid detected Distrobox name. Follow the Distrobox guide to select an existing box and update the host user unit.",
  },
  distroboxRecipeUnsupported: {
    title: recipeTitle,
    remediation:
      "Follow the Distrobox guide to update the host user unit, then reload the user manager. Keep both application binaries installed inside the same box. Stop an older daemon cleanly before changing its unit.",
  },
  operationLock: {
    title: "Service controls need the current host setup",
    remediation:
      "Install the current packaging/tmpfiles.d/lianli.conf on the host and run sudo systemd-tmpfiles --create lianli.conf, then Recheck.",
  },
  ownershipKernel: {
    title: "Kernel lock ownership is unverified",
    remediation:
      "Restore the shared host lock and host bridge before switching modes. A PID written in the file does not prove current ownership.",
  },
  ownershipProcess: {
    title: "Hardware owner's service is unverified",
    remediation:
      "Recheck after startup or shutdown finishes. Service switching requires host process and service cgroup access. Do not stop a PID based on its number alone.",
  },
  userService: { title: "User hardware service", remediation: hardwareServiceRepair },
  systemService: { title: "System hardware service", remediation: hardwareServiceRepair },
  globalUserUnverified: {
    title: "Global user startup is unverified",
    remediation: "Check global user-service enablement on the host before selecting system mode.",
  },
  serviceConflict: {
    title: "Competing hardware service selections",
    remediation:
      "Choose one hardware service mode. Resolve global user enablement and manually launched owners before switching. Do not stop another user's daemon.",
  },
  globalUserEnabled: {
    title: "Hardware service enabled for every user",
    remediation:
      "Ask the administrator to disable the global default and select startup only for the intended user or the system service.",
  },
  mediaTempStorage: { title: "Temporary media storage", remediation: storageRepair },
  stateStorage: { title: "State directory storage", remediation: storageRepair },
  usbNodeAccess: {
    title: "USB/HID node permissions",
    remediation:
      "Apply host USB rules and group changes, then restart the login or service. For Distrobox, stop and re-enter the box. Missing hidraw nodes need kernel or device-visibility repair. Check daemon logs for open failures.",
  },
};
