function gcd(a: number, b: number): number {
  return b === 0 ? a : gcd(b, a % b);
}

/** Aspect ratio label of a resolution, e.g. 1440x1080 → "4:3". 8:5 is shown as the usual "16:10". */
export function aspectLabel(width: number, height: number): string {
  const d = gcd(width, height);
  const [w, h] = [width / d, height / d];
  if (w === 8 && h === 5) return "16:10";
  return `${w}:${h}`;
}

export function formatResolution(width: number, height: number): string {
  return `${width} × ${height}`;
}
