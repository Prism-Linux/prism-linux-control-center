export function linePath(
  values: readonly (number | null)[],
  width = 100,
  height = 40,
  pad = 3,
  domain?: readonly [number, number],
): string {
  const valid = values.filter((value): value is number => value != null && Number.isFinite(value));
  if (!valid.length) return "";
  const min = domain?.[0] ?? Math.min(...valid);
  const max = domain?.[1] ?? Math.max(...valid);
  const span = Math.max(max - min, Math.abs(max) * 0.04, 1);
  const left = pad;
  const right = width - pad;
  const top = pad;
  const bottom = height - pad;
  return valid.map((value, index) => {
    const x = valid.length === 1 ? right : left + index / (valid.length - 1) * (right - left);
    const y = bottom - ((value - min) / span) * (bottom - top);
    return `${index === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
  }).join(" ");
}

export function finitePercent(value: number | null | undefined): number | null {
  return value != null && Number.isFinite(value) && value >= 0 && value <= 100 ? value : null;
}
