# ASUS EC support for the original ROG STRIX Z790-E GAMING WIFI

This directory carries the upstream Linux `asus-ec-sensors` hwmon driver with
one focused DMI profile for the original `ROG STRIX Z790-E GAMING WIFI` board.
It exposes only `Water_In` using the Intel 700-series mapping. The profile is
kept separate from the WIFI II profile.

The source is based on Linux upstream `drivers/hwmon/asus-ec-sensors.c` at
commit `abf8c33470f78e2226ee3dcdc69004660020ab47` (retrieved 2026-10-30).
The corresponding change is recorded in
[`original-z790e-wifi.patch`](original-z790e-wifi.patch). The source retains
its upstream GPL-2.0-or-later SPDX notice.

## Mapping and safeguards

- Exact DMI board name: `ROG STRIX Z790-E GAMING WIFI`.
- Exposed hwmon label: `Water_In`.
- EC family mapping: Intel 700 series, bank `0x01`, offset `0x00`, one-byte
  temperature value.
- EC access mutex: `\\_SB.PC00.LPCB.SIO1.MUT0`, found in the captured DSDT.
- No `force_id`, EC writes, firmware changes, or WIFI II mapping reuse.

## Local deployment

The verified host has this driver installed as DKMS package
`asus-ec-sensors/1.0.0.1`, built for its running kernel. The module is selected
at boot by `/etc/modules-load.d/asus-ec-sensors.conf`. DKMS rebuilds it for
future kernels when matching headers are installed. This local driver addition
is separate from the Unified Linux Controller daemon package.

To reproduce on a compatible checkout with administrator access:

```sh
cd drivers/asus-ec-sensors
sudo install -d /usr/src/asus-ec-sensors-1.0.0.1
sudo cp asus-ec-sensors.c Makefile dkms.conf /usr/src/asus-ec-sensors-1.0.0.1/
sudo dkms add -m asus-ec-sensors -v 1.0.0.1
sudo dkms build -m asus-ec-sensors -v 1.0.0.1
sudo dkms install -m asus-ec-sensors -v 1.0.0.1
```

Confirm the exact DMI match before using the module on other hardware. Remove
the DKMS package and module-load file to roll it back.
