# Known limitations

- Hardware coverage varies by model, firmware and connection mode. Check the [compatibility matrix](hardware-compatibility.md) before use.
- Prerelease update discovery depends on the proposed `sgtaziz/prism-linux-control-center` GitHub repository being created. Existing stable `1.2.4` builds continue using the old stable update endpoint.
- The exact ASUS T_SENSOR input on the original ROG STRIX Z790-E GAMING WIFI is not exposed through a verified Linux sensor mapping.
- Some wireless device commands and LCD paths lack complete physical verification. A USB write or successful API response does not prove a display or LED changed.
- ASUS Aura physical RGB behavior is not established by the release evidence reviewed.
- G.Skill ENE Rainbow animation timing differs between the two verified DIMMs; the controllers do not advertise synchronized RGB timing.
- The RTX 4080 backend supports Static and Rainbow Wave only. Other effects, brightness/speed control, independent zones, hardware save commands and device readback are not supported.
- Suspend/resume validation is deferred.
- The frontend production bundle emits an existing large-chunk advisory.
- No beta distribution package or isolated installation smoke test has been produced in this preparation.
