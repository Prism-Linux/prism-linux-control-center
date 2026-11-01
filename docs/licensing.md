# Licensing and attribution

## Project license

The owner has approved the MIT license for original Prism Linux Control Center contributions by Major Riven. The root [LICENSE](../LICENSE) retains the original `sgtaziz` copyright notice and the MIT permission text, and identifies Major Riven's original contributions. This project-level license does not relicense code or assets from Lian Li Linux contributors or other third parties.

Prism is a separately maintained derivative of [Lian Li Linux](https://github.com/sgtaziz/lian-li-linux), based on upstream commit `896593b3b84f0d9225471222a319c4c95144c635`. Existing file-level copyright and license notices remain with their files. See [NOTICE](../NOTICE) and [SOURCE_PROVENANCE.md](../SOURCE_PROVENANCE.md).

## Included third-party material

- `vendor/tinyuz` and `vendor/HDiffPatch` retain their upstream MIT license files.
- `drivers/asus-ec-sensors` retains Eugene Shalygin's copyright and `GPL-2.0-or-later` identifier.
- `crates/lianli-display/src/hermes/uapi.rs` contains public Hermes-KMS UAPI declarations with `GPL-2.0 WITH Linux-syscall-note`; the complete license and exception text is retained in `crates/lianli-display/src/hermes/UAPI-LICENSE`.
- Bundled fonts retain their upstream SIL Open Font License 1.1 texts beside the font files. Template-specific credits are included where supplied; preserve those credits and all source-file notices.
- The Lancool 207 A5 template is omitted because permission to redistribute that contribution was not established. The RX78 artwork/font template remains omitted for the same reason. Do not restore either without documented rights clearance.

See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) for the included notices and the Rust/npm dependency license inventory. System libraries linked for a particular build target are supplied separately and are not covered by that dependency inventory.

## Branding

Prism is not affiliated with or endorsed by Lian Li, ASUS, G.Skill, Gigabyte, or other manufacturers referenced in the project. Their names and marks identify products and belong to their respective owners. The project does not claim rights in those marks. No independent trademark clearance for the Prism name or icon is asserted here.
