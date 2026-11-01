use crate::account::Account;
use crate::reservation::ServiceOperationLock;
use crate::switch_journal::{Journal, Record};
use crate::ui_text::text;
use anyhow::{ensure, Result};
use lianli_shared::installation::InstallationContext;
use lianli_shared::services::ServiceScope;
use lianli_shared::ui_message::UiText;
use std::path::Path;

fn caller_uid(record: &Record) -> u32 {
    if record.intent.source.scope == ServiceScope::User {
        record.intent.source.uid
    } else {
        record.intent.destination.uid
    }
}

pub fn run() -> Result<UiText> {
    ensure!(
        unsafe { libc::geteuid() } == 0
            && InstallationContext::detect() == InstallationContext::Native,
        "Automatic recovery requires the installed native system service"
    );
    let (caller, id) =
        {
            let operation = ServiceOperationLock::acquire(&InstallationContext::Native)?;
            let Some(journal) = Journal::load(&operation)? else {
                return Ok(text(
                    "control.switch.nothingToRecover",
                    "No service switch needs recovery.",
                ));
            };
            let caller = Account::user(caller_uid(journal.record()))?;
            ensure!(caller.name == journal.record().intent.caller_name,
            "The pending switch account changed. Preserve its journal for administrator recovery");
            (caller, journal.record().intent.id.clone())
        };
    crate::switch_job::recover_automatically(caller, &id)
}

pub fn trigger() -> Result<()> {
    if InstallationContext::detect() != InstallationContext::Native
        || ![
            "/etc/systemd/system/lianli-control-recovery.service",
            "/usr/lib/systemd/system/lianli-control-recovery.service",
        ]
        .iter()
        .any(|path| Path::new(path).is_file())
    {
        return Ok(());
    }
    crate::recovery_trigger::trigger()
}
