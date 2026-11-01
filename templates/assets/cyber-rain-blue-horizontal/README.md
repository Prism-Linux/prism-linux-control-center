# Cyber Rain / Blue

Blue/cyan variant of Cyber Rain / Mustard for the Universal Screen 8.8-inch in horizontal 1920×480 orientation. Includes CPU/GPU temperature and analog load gauges, download/upload gauges, RAM usage/capacity, free storage, NVMe temperature and a seconds clock.

Install through Browse Online Templates and confirm the detected sensors. CPU/GPU/RAM and network widgets retain portable sensor_category hints; catalog installation replaces the neutral sources with the chosen sensors and clears those hints. A category left unavailable displays N/A or an empty gauge until assigned in the editor. Do not copy the portable template directly into the installed template list without binding its sensors.

Network gauges use decimal MB/s on the selected default-route interface, with a fixed 0–125 MB/s range (1 Gbit/s equivalent). Numeric readings can exceed the gauge maximum. Update bindings after switching interfaces or VPNs.

RAM capacity reads kernel-visible MemTotal in GiB, rounded to a whole number, with read-only awk. FREE reads available GiB on the daemon account's home filesystem using df/awk. These require standard Linux tools in the daemon environment. Both readouts update every 10 seconds; command sampling is controlled independently by the daemon.

SSD temperature uses the nvme Composite hwmon sensor without a machine-specific PCI path. If unavailable, or if multiple drives are present, choose the intended sensor in the editor. Its readout updates every five seconds.

The Blue layout and live sensor readings were confirmed by @marvinduranr on a Universal Screen 8.8-inch, horizontal 1920×480, with Lian Li Linux 1.2.4 in a Fedora 44 Distrobox on Bazzite and an AMD RX 9070 XT. The tested local installation required removing portable category markers after assigning its real sources; catalog installation performs that binding step. This catalog submission additionally inherits Mustard's automatic RAM-capacity readout. Full installation of Blue from the upstream catalog remains untested until published.

The native-renderer preview uses simulated values and is stored as JPEG below the browser's 1 MiB preview limit. The background is static; gauges and values update live. See CREDITS.md for artwork and font licenses.
