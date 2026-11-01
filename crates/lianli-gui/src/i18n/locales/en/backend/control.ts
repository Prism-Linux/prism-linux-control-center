export default {
  serviceAction: {
    checkingState: "Checking service state…",
    checking: "Checking service state and hardware ownership…",
    authorize: "Approve the service authorization prompt to continue…",
    waiting: "Waiting for the service and verifying hardware ownership…",
    started: "Service started. Daemon identity and loaded settings verified.",
    stopped: "Service stopped cleanly. Hardware ownership released.",
    restarted: "Service restarted. New daemon identity and loaded settings verified.",
    unverified:
      "Service completion could not be verified: {detail}. No forced kill or automatic retry was issued. Recheck services and inspect the selected service's journal.",
    interrupted: "Service verification was interrupted. Recheck services before retrying.",
    alreadyRunning: "Another service operation is running",
    dispatchFailed: "The action was not retried. Recheck service state if the request timed out: {error}",
  },
  switch: {
    checkingPrerequisites: "Checking service switch prerequisites…",
    checking: "Checking both service accounts, startup selection and saved settings…",
    oldMediaUnavailable:
      "Old destination media is unavailable. Its settings will be backed up and replaced by the copied settings.",
    preparing: "Preparing settings and decoding copied media under the destination account…",
    stoppingSource: "Pausing competing starts and stopping the current daemon cleanly…",
    publishing: "Publishing prepared settings and media with rollback backups…",
    starting: "Starting and verifying the selected daemon…",
    switched: "Service mode switched. Selected account and loaded configuration have been verified.",
    restoring: "Restoring the previous service mode…",
    nothingToRecover: "No service switch needs recovery.",
    archived: "Completed switch record archived.",
    restored: "Previous service mode restored and verified.",
    interrupted: "Service switching was interrupted. Use Recover switch before trying another mode.",
    failed:
      "Service switch failed. Recheck Installation Health. If recovery is pending, choose Recover switch. See the journal for lianli-control-switch.service or lianli-control-recovery.service.",
    alreadyRunning: "Another service switch is running",
    actionRunning: "Another service action is running",
  },
  route: {
    unsupportedContainer: "Host service access is unavailable in this container",
    hostSpawnMissing:
      "Run distrobox-host-exec true interactively inside this box to install host-spawn, then Recheck. No helper is installed by this check.",
    busHidden: "The host user's session bus is not visible in this box: {error}",
    busForeign: "The visible host session bus does not belong to this user",
    hostToolMissing: "Host /usr/bin/{name} is unavailable. Service checks cannot run safely",
    bridgeFailed:
      "Host command bridge failed: {error}. Verify the host's Flatpak session helper and the box's host session-bus access.",
    ownershipLockUnavailable: "Host ownership lock is unavailable",
    operationLockUnavailable: "Host service operation lock is unavailable",
    startupPaused:
      "Daemon startup is paused for an unfinished service switch. Recover the switch before starting either mode",
    ownerChanged: "Hardware owner changed during process inspection. Recheck services",
  },
  mediaImport: {
    identityMismatch: "Import helper returned a different identity. Inspect storage before retrying",
    workerStopped: "Import worker stopped. Files may have been copied. Inspect storage before retrying.",
  },
  mediaStorage: {
    noReceipt: "No ownership receipt. Retained as unverified storage",
    unpublishedReceipt:
      "Ownership receipt has no published directory. An import may be pending or interrupted. Review metadata and references before removal.",
    invalidReceipt: "Invalid retained ownership receipt: {error}",
    interruptedRemoval: "Interrupted removal. Review remaining files and references before resuming",
  },
};
