/** REZ mark: a slanted 4:3 frame pulled wide by two blue wedges (same drawing as the app icon). */
export default function RezLogo({ size = 36 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="96 300 832 424" aria-hidden="true" className="shrink-0 overflow-visible">
      <defs>
        <linearGradient id="rez-logo-blue" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#60A5FA" />
          <stop offset="1" stopColor="#2563EB" />
        </linearGradient>
      </defs>
      <g transform="translate(512 512) skewX(-12) translate(-512 -512)">
        <rect x="322" y="362" width="380" height="300" rx="44" fill="none" stroke="#fff" strokeWidth="64" />
        <path d="M262 512 L132 392 L132 632 Z" fill="url(#rez-logo-blue)" stroke="url(#rez-logo-blue)" strokeWidth="28" strokeLinejoin="round" />
        <path d="M762 512 L892 392 L892 632 Z" fill="url(#rez-logo-blue)" stroke="url(#rez-logo-blue)" strokeWidth="28" strokeLinejoin="round" />
      </g>
    </svg>
  );
}
