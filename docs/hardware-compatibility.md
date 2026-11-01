# Hardware compatibility

The daemon discovers devices and advertises capabilities per device. A listed family does not mean that every model, revision, connection mode or feature is verified. Device appearance in discovery alone is not evidence that a command changed hardware.

| Area | Status | Evidence and limits |
|---|---|---|
| Lian Li wired USB controllers | Implemented; selected devices and controls have owner history, with broad model/revision coverage partial | Source includes fan, pump, RGB and LCD drivers for multiple UNI FAN, AIO, Strimer and screen families. Do not generalize one model's results to all listed variants. The exact device and each used feature need to be checked against capabilities and the individual reports. |
| Lian Li wireless TX/RX and wireless devices | Implemented; partial/experimental by model and command | The two reported dongle revisions and device families have code paths. Radio binding/control depends on both endpoints and protocol acknowledgements; historical USB captures do not verify over-air behavior. Wireless LCD streaming by dongle alone is unsupported. |
| Fan and pump control | Implemented; partially verified | Curves, fixed duty and supported motherboard PWM sync are in the daemon. Reports confirm discovery and sync initialization on this host. A reported duty is not a measured fan RPM or proof of safe cooling on every device. BIOS/manual mode and each fan/pump's response still require model-specific validation. |
| Motherboard fan synchronization | Implemented where the device advertises it; partially verified | Linux PWM sources are used for supported Lian Li headers. Missing/stale inputs have documented fallback behavior. This does not enable motherboard ARGB synchronization on unrelated controllers. |
| Motherboard ARGB synchronization | Experimental, controller-specific | The GUI and daemon expose supported device capabilities. No general motherboard ARGB ownership or universal board header support is claimed. G.Skill ENE RAM reports no motherboard or merge sync capability. |
| G.Skill DDR5 RGB through ENE | Physically verified on the owner's two ENE controllers | Owner verified static red/blue, breathing green, Rainbow, off, direct colors and startup Rainbow restoration. Exact DIMM slots were not mapped; equal animation timing is not supported or verified. |
| ASUS Aura USB RGB | Implemented; physical effect response not established by the release evidence reviewed | The host enumerated ASUS Aura `0B05:19AF`, saved RGB settings and source includes an Aura path. Do not infer hardware response from enumeration or daemon startup. Exact device/revision, effects, restart restoration and interactions with ASUS software require additional documented observation. |
| ASUS motherboard coolant sensor | Unsupported on the owner's original ROG STRIX Z790-E GAMING WIFI | The exact `T_SENSOR` input was not mapped to a Linux sensor source. An AUXTIN value is not treated as coolant without a supported mapping. The WIFI II driver table is not reused for this board. |
| LCD devices | Implemented; selected physical paths verified, broad models partially verified | The repository supports USB-connected panels, image/video/template workflows and optional desktop modes by device. Owner history confirms selected TLV2 still-image presentation; other transfers, panel presentation, templates and desktop capture are not equally verified. USB transfer completion is not proof of presentation. |
| Gigabyte AORUS RTX 4080 XTREME WATERFORCE WB RGB | Physically verified after GUI dispatch repair | Exact match only: `10de:2704`, subsystem `1458:40c8`. Static and Rainbow Wave are the supported modes on one combined zone. NVIDIA PortId 1 is resolved dynamically below the GPU; bus number is not fixed. Backend has no LED readback. No other GPU is claimed. |
| Suspend/resume | Deferred | Suspend/resume was intentionally not validated. |

## Verification notes

The owner confirmed that RTX 4080 Static and Rainbow Wave control physically works after the GUI dispatch repair. The exact PCI/subsystem match and dynamic NVIDIA PortId 1 adapter lookup are part of the implementation. The controller has no LED readback, so logs and successful I²C messages are not presented as visual proof.

The owner verified Static Red/Blue, Breathing Green, Rainbow, Off, Direct colors and saved Rainbow restoration on two ENE G.Skill DIMM controllers. This is evidence for those controllers, not all memory RGB hardware.

Other historical phase testing is not included in the public release source snapshot because the raw records contain private host and protocol research details.
