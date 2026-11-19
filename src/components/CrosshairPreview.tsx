import { useCrosshairPreview } from "../lib/crosshair";
import type { Crosshair } from "../lib/types";

interface Props {
  crosshair: Crosshair | null;
  /** Tailwind size classes of the scene. */
  className?: string;
  debounceMs?: number;
}

/**
 * The crosshair over a dim game-like scene. The image is drawn by the Rust renderer at 2x and
 * shown at its natural pixel size halved, so thumbnails stay crisp.
 */
export default function CrosshairPreview({ crosshair, className = "h-16 w-16", debounceMs = 0 }: Props) {
  const url = useCrosshairPreview(crosshair, debounceMs);
  return (
    <div
      className={`relative flex shrink-0 items-center justify-center overflow-hidden rounded-[14px] ${className}`}
      style={{
        background:
          "radial-gradient(circle at 50% 45%, #3a4150 0%, #20242c 55%, #121418 100%), linear-gradient(#000, #000)",
      }}
    >
      <span aria-hidden="true" className="absolute inset-x-0 top-[58%] h-px bg-white/5" />
      {url ? <img src={url} alt="" className="relative max-h-full max-w-full [image-rendering:pixelated]" /> : null}
    </div>
  );
}
