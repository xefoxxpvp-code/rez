interface Props {
  width: number;
  height: number;
  /** Native screen size, drawn as the dashed outline the game is stretched to. */
  screen: { width: number; height: number };
  active?: boolean;
  /** Frame height in px. */
  size?: number;
}

/** The game's aspect ratio drawn inside the screen's shape: what REZ stretches to full screen. */
export default function AspectFrame({ width, height, screen, active = false, size = 26 }: Props) {
  const H = size;
  const W = Math.round((H * screen.width) / screen.height);
  const innerW = Math.min(W, (H * width) / height);
  const x = (W - innerW) / 2;
  return (
    <svg width={W + 2} height={H + 2} viewBox={`-1 -1 ${W + 2} ${H + 2}`} aria-hidden="true" className="shrink-0">
      <rect x={0} y={0} width={W} height={H} rx={3} fill="none" stroke="rgb(255 255 255 / 0.2)" strokeDasharray="3 3" />
      <rect
        x={x}
        y={0}
        width={innerW}
        height={H}
        rx={3}
        fill={active ? "rgb(15 46 181 / 0.3)" : "rgb(255 255 255 / 0.04)"}
        stroke={active ? "#60a5fa" : "rgb(255 255 255 / 0.7)"}
        strokeWidth={1.5}
      />
    </svg>
  );
}
