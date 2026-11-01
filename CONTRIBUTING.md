# Contributing

Thanks for helping improve Prism Linux Control Center. Before opening a change, read the hardware and license notes below and keep existing user configuration and working device paths intact.

## Development

Read [Build from source](docs/building-from-source.md). Run the relevant checks for your change. The project's `AGENTS.md` describes architecture boundaries, hardware safety, localization, and the repository's validation commands.

For GUI text, update English and Simplified Chinese locale files together. For a new hardware path, include device identity limits, protocol evidence, error handling, startup restoration and a clear split between software tests and physical observations.

## Hardware reports

Include the exact model and revision, USB/PCI identity, connection type, operating system and kernel, application version, command result, relevant redacted logs, and the physical outcome. Mark device behavior unverified when only enumeration, IPC success or bus transfer completion is available. Never include passwords, tokens, private keys, raw owner Windows captures or unrelated device configuration.

## Licensing and assets

Only contribute code and artwork that you can license for the intended distribution. Unless agreed otherwise before submission, by submitting an original contribution you license it to the project and recipients under MIT; you retain your copyright. Preserve notices for copied code, fonts, images and template assets. See [license and attribution](docs/licensing.md).

AI tools may assist with development. Contributors remain responsible for reviewing and testing submissions, checking factual and licensing claims, and identifying AI-generated artwork or other material that needs a separate credit or rights review. Do not submit generated work that imitates a protected character, brand asset, or living artist without the necessary rights.

## Scope and review

Keep changes focused, include a regression test that fails on the reported behavior when practical, and describe any untested hardware lifecycle path. Do not submit speculative hardware writes, firmware changes or undocumented register access.
