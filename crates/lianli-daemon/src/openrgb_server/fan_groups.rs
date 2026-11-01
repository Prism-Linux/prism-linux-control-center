use super::*;
use lianli_media::rgb::sync_layout::Layout;
use lianli_shared::rgb::{RgbRenderFamily, RgbZoneInfo};
use std::collections::HashMap;

const GROUP_ZONE_NAME: &str = "Fans";

/// A wireless fan group exposed as one zone along its sync layout, so
/// OpenRGB effects, which render each zone independently, span every fan.
/// Per-fan segments are not exposed because the Effects plugin renders each
/// selected segment independently too, splitting the effect per fan again.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FanGroup {
    layout: Layout,
    physical_zones: Vec<usize>,
    reversed_fan_leds: Option<usize>,
}

pub(super) type FanGroups = HashMap<String, Arc<FanGroup>>;

pub(super) fn expose(
    caps: Vec<RgbDeviceCapabilities>,
    per_fan_zones: bool,
) -> (Vec<RgbDeviceCapabilities>, FanGroups) {
    let mut groups = FanGroups::new();
    let caps = caps
        .into_iter()
        .map(|mut cap| {
            if per_fan_zones {
                return cap;
            }
            if let Some(group) = FanGroup::plan(&cap) {
                let leds = group.layout.logical_led_count() as u16;
                cap.zones = vec![RgbZoneInfo {
                    name: GROUP_ZONE_NAME.to_owned(),
                    name_ui: None,
                    led_count: leds,
                }];
                cap.total_led_count = leds;
                groups.insert(cap.device_id.clone(), Arc::new(group));
            }
            cap
        })
        .collect();
    (caps, groups)
}

impl FanGroup {
    fn plan(cap: &RgbDeviceCapabilities) -> Option<Self> {
        let profile = cap.render_profile?;
        let fans = usize::from(profile.fan_count);
        if !cap.device_id.starts_with("wireless:")
            || !cap.supports_direct
            || fans == 0
            || cap.zones.len() != fans
        {
            return None;
        }
        let layout = Layout::for_profile(profile)?;
        let physical_zones: Vec<usize> =
            cap.zones.iter().map(|z| usize::from(z.led_count)).collect();
        let per_fan = layout.logical_led_count() / fans;
        // The daemon advertises a sync layout only for devices whose merged
        // lighting it supports, so OpenRGB follows the same rule.
        if cap.sync_led_count != Some(layout.logical_led_count() as u16)
            || layout.physical_led_count() != physical_zones.iter().sum::<usize>()
            || per_fan * fans != layout.logical_led_count()
        {
            return None;
        }
        // The sync layout is in RF wire order; regular uploads reverse the fans
        // of a right-attached SL-INF chain, so undo that here.
        let reversed = profile.right_attach
            && matches!(
                profile.family,
                RgbRenderFamily::SlInf | RgbRenderFamily::SlInfV3
            );
        let reversed_fan_leds = reversed.then(|| layout.physical_led_count() / fans);
        let group = Self {
            layout,
            physical_zones,
            reversed_fan_leds,
        };
        group.keeps_fans_apart(per_fan).then_some(group)
    }

    /// Some sync layouts pack every fan into a different wire format than
    /// regular uploads; those cannot be projected onto per-fan zones.
    fn keeps_fans_apart(&self, per_fan: usize) -> bool {
        let strip: Vec<[u8; 3]> = (1..=self.layout.logical_led_count() as u16)
            .map(|led| {
                let [high, low] = led.to_be_bytes();
                [high, low, 0]
            })
            .collect();
        let Ok(zones) = self.project(&strip) else {
            return false;
        };
        let fans = zones.len();
        zones.iter().all(|(zone, colors)| {
            let zone = usize::from(*zone);
            let fan = if self.reversed_fan_leds.is_some() {
                fans - 1 - zone
            } else {
                zone
            };
            let own = fan * per_fan + 1..=(fan + 1) * per_fan;
            colors
                .iter()
                .map(|[high, low, _]| usize::from(u16::from_be_bytes([*high, *low])))
                .all(|led| led == 0 || own.contains(&led))
        })
    }

    /// Maps colours along the group onto the device's per-fan zones.
    pub fn project(&self, colors: &[[u8; 3]]) -> anyhow::Result<Vec<(u8, Vec<[u8; 3]>)>> {
        let mut physical = self.layout.project_frame(colors, 0..colors.len(), false)?;
        if let Some(leds) = self.reversed_fan_leds {
            physical = physical
                .chunks_exact(leds)
                .rev()
                .flatten()
                .copied()
                .collect();
        }
        let mut start = 0;
        Ok(self
            .physical_zones
            .iter()
            .enumerate()
            .map(|(zone, &count)| {
                let colors = physical[start..start + count].to_vec();
                start += count;
                (zone as u8, colors)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lianli_shared::rgb::RgbRenderProfile;

    fn fans(
        family: RgbRenderFamily,
        fans: u8,
        leds_per_fan: u16,
        right: bool,
    ) -> RgbDeviceCapabilities {
        let zones: Vec<_> = (1..=fans)
            .map(|fan| serde_json::json!({"name": format!("Fan {fan}"), "led_count": leds_per_fan}))
            .collect();
        let mut cap: RgbDeviceCapabilities = serde_json::from_value(serde_json::json!({
            "device_id": "wireless:aa:bb:cc:dd:ee:ff",
            "device_name": "Fans",
            "supported_modes": ["Direct"],
            "zones": zones,
            "total_led_count": u16::from(fans) * leds_per_fan,
            "supports_direct": true,
            "supports_mb_rgb_sync": false,
            "supported_scopes": []
        }))
        .unwrap();
        let profile = RgbRenderProfile {
            family,
            fan_count: fans,
            led_count: u16::from(fans) * leds_per_fan,
            right_attach: right,
        };
        cap.render_profile = Some(profile);
        cap.sync_led_count =
            Layout::for_profile(profile).map(|layout| layout.logical_led_count() as u16);
        cap
    }

    #[test]
    fn every_fan_family_maps_each_part_of_the_strip_onto_its_own_fan() {
        for (family, leds_per_fan) in [
            (RgbRenderFamily::Tl, 26),
            (RgbRenderFamily::Sl, 40),
            (RgbRenderFamily::SlInf, 44),
            (RgbRenderFamily::Cl, 24),
        ] {
            let group = FanGroup::plan(&fans(family, 3, leds_per_fan, false))
                .unwrap_or_else(|| panic!("{family:?} has no fan group"));
            let logical = group.layout.logical_led_count();
            let per_fan = logical / 3;
            let colors: Vec<[u8; 3]> = (1..=logical as u8).map(|led| [led; 3]).collect();
            for (zone, physical) in group.project(&colors).unwrap() {
                let own = usize::from(zone) * per_fan..(usize::from(zone) + 1) * per_fan;
                assert_eq!(physical.len(), usize::from(leds_per_fan), "{family:?}");
                let lit: Vec<usize> = physical
                    .iter()
                    .filter(|color| color[0] != 0)
                    .map(|color| usize::from(color[0]) - 1)
                    .collect();
                assert!(!lit.is_empty(), "{family:?}");
                assert!(
                    lit.iter().all(|led| own.contains(led)),
                    "{family:?} zone {zone} shows another fan's colours"
                );
            }
        }
    }

    #[test]
    fn devices_without_merged_lighting_keep_per_fan_zones() {
        let mut cap = fans(RgbRenderFamily::Tl, 3, 26, false);
        cap.sync_led_count = None;
        assert!(FanGroup::plan(&cap).is_none());
        // SL V4 sync frames pack each fan's LEDs differently from regular uploads.
        assert!(FanGroup::plan(&fans(RgbRenderFamily::SlV4, 3, 52, false)).is_none());
    }

    #[test]
    fn tl_group_becomes_one_zone_along_the_group() {
        let (caps, groups) = expose(vec![fans(RgbRenderFamily::Tl, 3, 26, false)], false);
        assert_eq!(caps[0].zones.len(), 1);
        assert_eq!(caps[0].zones[0].led_count, 39);
        assert_eq!(caps[0].total_led_count, 39);
        let group = &groups["wireless:aa:bb:cc:dd:ee:ff"];

        let colors: Vec<[u8; 3]> = (0..39u8).map(|led| [led; 3]).collect();
        let zones = group.project(&colors).unwrap();
        assert_eq!(zones.len(), 3);
        for (fan, (zone, physical)) in zones.iter().enumerate() {
            assert_eq!(usize::from(*zone), fan);
            assert_eq!(physical.len(), 26);
            let first = (fan * 13) as u8;
            assert_eq!(physical[0], [first; 3]);
            assert_eq!(physical[12], [first + 12; 3]);
            assert_eq!(physical[13], [first; 3]);
        }
    }

    #[test]
    fn per_fan_setting_and_unsupported_devices_keep_their_zones() {
        let (caps, groups) = expose(vec![fans(RgbRenderFamily::Tl, 3, 26, false)], true);
        assert_eq!(caps[0].zones.len(), 3);
        assert!(groups.is_empty());

        let mut wired = fans(RgbRenderFamily::Tl, 3, 26, false);
        wired.device_id = "hid:0416:7372:test".into();
        let mut no_layout = fans(RgbRenderFamily::Tl, 3, 20, false);
        no_layout.device_id = "wireless:11:22:33:44:55:66".into();
        let (caps, groups) = expose(vec![wired, no_layout], false);
        assert!(caps.iter().all(|cap| cap.zones.len() == 3));
        assert!(groups.is_empty());
    }

    #[test]
    fn group_colours_reach_every_fan_and_openrgb_sees_one_unsegmented_zone() {
        let (caps, groups) = expose(vec![fans(RgbRenderFamily::Tl, 3, 26, false)], false);
        let mut rgb = RgbController::new(HashMap::new(), None);
        let id = &caps[0].device_id;
        let state =
            super::super::legacy::states(&caps, &Default::default(), &groups, &rgb)[id].clone();
        let buffer = Mutex::new(DirectColorBuffer::new());
        let colors = (0..39u8).map(|led| [led; 3]).collect();
        state
            .lock()
            .apply_colors(&Command::Colors(colors), &buffer)
            .unwrap();
        let zones = buffer.lock().take_all().remove(id).unwrap();
        assert_eq!(zones.len(), 3);
        assert_eq!(zones[&2].len(), 26);
        assert_eq!(zones[&2][0], [26; 3]);
        assert_eq!(zones[&2][25], [38; 3]);

        let description = state.lock().description(6);
        let mut zone = 1u16.to_le_bytes().to_vec();
        write_string(&mut zone, "Fans");
        zone.extend_from_slice(&ZONE_TYPE_LINEAR.to_le_bytes());
        for _ in 0..3 {
            zone.extend_from_slice(&39u32.to_le_bytes());
        }
        // No matrix and no segments: the Effects plugin would render each
        // segment separately and split the effect per fan again.
        zone.extend_from_slice(&[0; 4]);
        assert!(description.windows(zone.len()).any(|window| window == zone));
        rgb.stop();
    }

    #[test]
    fn right_attached_sl_inf_fans_are_reordered_for_regular_uploads() {
        let left = FanGroup::plan(&fans(RgbRenderFamily::SlInf, 2, 44, false)).unwrap();
        let right = FanGroup::plan(&fans(RgbRenderFamily::SlInf, 2, 44, true)).unwrap();
        let colors: Vec<[u8; 3]> = (0..16u8).map(|led| [led; 3]).collect();
        let wire = right.layout.project_frame(&colors, 0..16, false).unwrap();
        let regular = right.project(&colors).unwrap();
        assert_eq!(regular[0].1, wire[44..]);
        assert_eq!(regular[1].1, wire[..44]);
        assert_eq!(
            left.project(&colors).unwrap()[0].1,
            left.layout.project_frame(&colors, 0..16, false).unwrap()[..44]
        );
    }
}
