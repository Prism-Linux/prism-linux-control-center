import type { SensorInfo, SensorSource } from "@/types";

function isFresh(sampleTime: number | null, now: number): boolean {
  return sampleTime != null && sampleTime <= now && now - sampleTime <= 10_000;
}

export function liveTemperature(
  sensors: SensorInfo[],
  kind: "cpu" | "gpu",
  sampleTime: number | null,
  now = Date.now(),
): number | null {
  if (!isFresh(sampleTime, now)) return null;
  const valid = sensors.filter((sensor) =>
    sensor.unit === "C" && sensor.current_value != null && Number.isFinite(sensor.current_value),
  );
  let candidates: SensorInfo[];
  if (kind === "cpu") {
    const cpuHwmon = valid.filter((sensor) =>
      sensor.source.type === "hwmon" && ["k10temp", "coretemp"].includes(sensor.source.name),
    );
    candidates = cpuHwmon.length ? cpuHwmon : valid.filter((sensor) => {
      const identity = [sensor.display_name, sensor.sensor_name?.device_name, sensor.sensor_name?.sensor_name]
        .filter(Boolean).join(" ").toLowerCase();
      return /\bcpu\b|\bcore\b/.test(identity);
    });
    const packageSensor = candidates.find((sensor) => {
      const label = [sensor.source.type === "hwmon" ? sensor.source.label : "", sensor.sensor_name?.sensor_name ?? ""]
        .join(" ").toLowerCase();
      return /tctl|package id 0|control temp/.test(label);
    });
    return packageSensor?.current_value ?? candidates[0]?.current_value ?? null;
  }

  const nvidia = valid.find((sensor) =>
    sensor.source.type === "nvidia_gpu" && sensor.source.metric === "temp",
  );
  if (nvidia) return nvidia.current_value;
  candidates = valid.filter((sensor) =>
    sensor.source.type === "hwmon" && ["amdgpu", "radeon"].includes(sensor.source.name),
  );
  const edge = candidates.find((sensor) => {
    const label = [sensor.source.type === "hwmon" ? sensor.source.label : "", sensor.sensor_name?.sensor_name ?? ""]
      .join(" ").toLowerCase();
    return label.includes("edge");
  });
  if (edge) return edge.current_value;
  const namedGpu = candidates.find((sensor) =>
    [sensor.display_name, sensor.sensor_name?.device_name].filter(Boolean).join(" ").toLowerCase().includes("gpu"),
  );
  const labeledGpu = valid.find((sensor) =>
    [sensor.display_name, sensor.sensor_name?.device_name, sensor.sensor_name?.sensor_name]
      .filter(Boolean).join(" ").toLowerCase().includes("gpu"),
  );
  return namedGpu?.current_value ?? candidates[0]?.current_value ?? labeledGpu?.current_value ?? null;
}

export function liveMemoryUsage(
  sensors: SensorInfo[],
  sampleTime: number | null,
  now = Date.now(),
): number | null {
  if (!isFresh(sampleTime, now)) return null;
  const sensor = sensors.find((item) => item.source.type === "mem_usage" && item.unit === "PERCENT");
  const value = sensor?.current_value;
  return value != null && Number.isFinite(value) && value >= 0 && value <= 100 ? value : null;
}

export function liveCoolantTemperature(
  sensors: SensorInfo[],
  source: SensorSource | null | undefined,
  sampleTime: number | null,
  now = Date.now(),
): number | null {
  if (!source || !isFresh(sampleTime, now)) return null;
  const sensor = sensors.find((item) => JSON.stringify(item.source) === JSON.stringify(source));
  if (!sensor || sensor.unit !== "C") return null;
  const value = sensor.current_value;
  return value != null && Number.isFinite(value) && value >= -50 && value <= 200 ? value : null;
}

export function meanFanRpm(values: number[]): number | null {
  const valid = values.filter((rpm) => Number.isFinite(rpm) && rpm > 0);
  return valid.length ? Math.round(valid.reduce((sum, rpm) => sum + rpm, 0) / valid.length) : null;
}
