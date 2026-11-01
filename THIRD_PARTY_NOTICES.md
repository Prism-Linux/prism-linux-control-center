# Third-party notices

Prism Linux Control Center contains or derives from work by upstream and third-party authors. The project MIT license applies only to original Prism contributions identified in [LICENSE](LICENSE); it does not replace other licenses or notices.

## Source and assets included in this repository

- **Lian Li Linux:** upstream source at <https://github.com/sgtaziz/lian-li-linux>, based on commit `896593b3b84f0d9225471222a319c4c95144c635`. Preserve `sgtaziz`'s notice in [LICENSE](LICENSE) and all existing file-level upstream notices.
- **tinyuz:** source and MIT notice in `vendor/tinyuz/` (`housisong`).
- **HDiffPatch:** source and MIT notice in `vendor/HDiffPatch/` (`housisong`; portions attributed to Yuta Mori).
- **ASUS EC sensors driver:** `drivers/asus-ec-sensors/` retains Eugene Shalygin's copyright and `GPL-2.0-or-later` identifier. Its terms are separate from the userspace MIT license.
- **Hermes-KMS UAPI declarations:** `crates/lianli-display/src/hermes/uapi.rs` follows Hermes-KMS revision `7fbdd011cc74a57c52e1beea0f98c10c5fd5f198` and retains the `GPL-2.0 WITH Linux-syscall-note` terms in `crates/lianli-display/src/hermes/UAPI-LICENSE`.
- **Fonts:** the applicable SIL Open Font License 1.1 text and attribution are beside each bundled font in `templates/assets/`.
- **Template art and layouts:** local `CREDITS.md` files provide contributor, AI image-generation, and font attribution where supplied. Existing upstream notices remain applicable. No broader grant of third-party rights is asserted here.

The Lancool 207 A5 template and RX78 template are excluded because redistribution rights for their material were not established. Their absence is intentional.

## Locked Rust dependencies

The following inventory was produced from `Cargo.lock` and the `license` fields in the corresponding exact-version Cargo manifests after retrieving the locked crates. All **655 registry packages** had a declared license expression; the prior count of missing local metadata was due to crate sources not being downloaded, not evidence of missing or incompatible rights. The inventory covers Cargo registry packages in the lockfile, including target-specific dependencies. The ten workspace crates are project source packages and are not counted as registry dependencies; upstream notices in those crates remain in effect.

License expression counts from those manifests:

| Declared license expression | Packages |
| --- | ---: |
| `(MIT OR Apache-2.0) AND NCSA` | 1 |
| `(MIT OR Apache-2.0) AND Unicode-3.0` | 1 |
| `0BSD OR MIT OR Apache-2.0` | 1 |
| `Apache-2.0` | 10 |
| `Apache-2.0 / MIT` | 1 |
| `Apache-2.0 AND ISC` | 1 |
| `Apache-2.0 AND MIT` | 1 |
| `Apache-2.0 OR BSL-1.0` | 1 |
| `Apache-2.0 OR ISC OR MIT` | 2 |
| `Apache-2.0 OR MIT` | 37 |
| `Apache-2.0 WITH LLVM-exception` | 1 |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 5 |
| `Apache-2.0/MIT` | 6 |
| `BSD-2-Clause` | 3 |
| `BSD-2-Clause OR Apache-2.0 OR MIT` | 2 |
| `BSD-3-Clause` | 8 |
| `BSD-3-Clause AND MIT` | 1 |
| `BSD-3-Clause OR Apache-2.0` | 2 |
| `BSD-3-Clause OR MIT OR Apache-2.0` | 2 |
| `BSD-3-Clause/MIT` | 1 |
| `BSL-1.0` | 2 |
| `CC0-1.0` | 1 |
| `CC0-1.0 OR Apache-2.0` | 1 |
| `CC0-1.0 OR MIT-0 OR Apache-2.0` | 1 |
| `CDLA-Permissive-2.0` | 1 |
| `ISC` | 6 |
| `MIT` | 146 |
| `MIT OR Apache-2.0` | 315 |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | 2 |
| `MIT OR Apache-2.0 OR Zlib` | 7 |
| `MIT OR Zlib OR Apache-2.0` | 1 |
| `MIT/Apache-2.0` | 29 |
| `MPL-2.0` | 5 |
| `Unicode-3.0` | 18 |
| `Unlicense OR MIT` | 7 |
| `Unlicense/MIT` | 2 |
| `WTFPL` | 2 |
| `Zlib` | 3 |
| `Zlib OR Apache-2.0 OR MIT` | 19 |

The five MPL-2.0 packages are `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`. The two `r-efi` versions offer MIT, Apache-2.0, or LGPL-2.1-or-later; the license expression is an alternative, not a finding that the project selected LGPL terms. For each dependency, consult the exact package version's own source and license text when redistributing a built binary.

## Frontend dependencies

The `crates/lianli-gui/package-lock.json` inventory records license metadata for all 139 packages: 116 MIT, 13 `Apache-2.0 OR MIT`, 2 `MIT OR Apache-2.0`, 2 BSD-3-Clause, 1 BSD-2-Clause, 3 ISC, and 2 Apache-2.0. The package lock identifies exact versions. Preserve any notices required by those packages when redistributing built frontend or application artifacts.

## Scope and remaining distribution checks

These inventories describe declared package licenses and the material in this source repository. They do not replace notices shipped by dependencies, legal review, or checks of system libraries linked for a particular distribution and build configuration. Packaging owners must include applicable license texts and comply with the specific terms of the binaries they ship. No distribution rights are claimed for excluded templates, and no independent trademark clearance for the Prism name or icon is asserted.
