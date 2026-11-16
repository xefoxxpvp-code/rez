import type { ReactNode } from "react";
import type { GameArt } from "../lib/types";
import AspectFrame from "./AspectFrame";

interface Props {
  art: GameArt | null;
  name: string;
  /** Used only when the game has no artwork at all. */
  fallback: { width: number; height: number; screen: { width: number; height: number } };
  active?: boolean;
  className?: string;
  children?: ReactNode;
}

/**
 * The game's own visuals: Steam hero art under a black veil with the transparent logo on
 * top; otherwise the exe icon over a blurred copy of itself. Art is only darkened, never tinted.
 */
export default function GameArtTile({ art, name, fallback, active = false, className = "", children }: Props) {
  let content: ReactNode;
  if (art?.hero) {
    content = (
      <>
        <img src={art.hero} alt="" className="absolute inset-0 h-full w-full object-cover opacity-80" />
        <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-black/25 to-black/10" />
        {art.logo ? (
          <img src={art.logo} alt={name} className="relative max-h-[62%] max-w-[82%] object-contain drop-shadow-[0_4px_12px_rgba(0,0,0,0.6)]" />
        ) : art.icon ? (
          <img src={art.icon} alt={name} className="relative h-12 w-12 object-contain" />
        ) : null}
      </>
    );
  } else if (art?.icon) {
    content = (
      <>
        <img src={art.icon} alt="" className="absolute inset-0 h-full w-full scale-150 object-cover opacity-35 blur-2xl" />
        <img src={art.icon} alt={name} className="relative h-14 w-14 object-contain drop-shadow-[0_4px_12px_rgba(0,0,0,0.6)]" />
      </>
    );
  } else {
    content = <AspectFrame width={fallback.width} height={fallback.height} screen={fallback.screen} active={active} size={40} />;
  }

  return (
    <div
      className={`relative flex shrink-0 items-center justify-center overflow-hidden rounded-[14px] ${
        active ? "bg-primary/30 ring-2 ring-blue-400/70" : "bg-white/[0.04]"
      } ${className}`}
    >
      {content}
      {children}
    </div>
  );
}
