interface Props {
  width: number;
  height: number;
  /** Native screen size, drawn as the dashed outline the game is stretched to. */
  screen: { width: number; height: number };
  active?: boolean;
}

/** The game's aspect ratio drawn inside the screen's shape: what REZ stretches to full screen. */
export default function AspectFrame({ width, height, screen, active = false }: Props) {
  const H = 26;
  const W = Math.round((H * screen.width) / screen.height);
  const innerW = Math.min(W, (H * width) / height);
  const x = (W - innerW) / 2;
  return (
    <svg width={W + 2} height={H + 2} viewBox={`-1 -1 ${W + 2} ${H + 2}`} aria-hidden="true" className="shrink-0">
      <rect x={0} y={0} width={W} height={H} rx={2} fill="none" stroke="var(--color-muted)" strokeDasharray="2 2" />
      <rect
        x={x}
        y={0}
        width={innerW}
        height={H}
        rx={2}
        fill={active ? "color-mix(in srgb, var(--color-signal) 22%, transparent)" : "transparent"}
        stroke={active ? "var(--color-signal)" : "var(--color-ink)"}
        strokeWidth={1.5}
      />
    </svg>
  );
}
