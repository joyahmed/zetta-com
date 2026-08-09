/// Packet counters, shortened.
///
/// Audio is fifty packets a second per peer, so `sent` passes a million within
/// hours and seven digits do not fit a fifth of a 460px panel — the number
/// either overflows its tile or forces every tile wider until the row wraps.
///
/// Nobody reads the seventh digit of a packet count anyway: these are watched
/// for whether they are moving and whether loss is climbing, not for their
/// exact value. One decimal keeps that visible at a glance — 1.2M ticking to
/// 1.3M still reads as motion.
const short = (n: number): string => {
	if (n < 1000) return String(n);
	for (const [limit, suffix] of [
		[1e9, 'G'],
		[1e6, 'M'],
		[1e3, 'k']
	] as const) {
		if (n >= limit) {
			const scaled = n / limit;
			// 9.9k, but 34k — a decimal on a two-digit number is noise, and it
			// is what pushes the string back to five characters.
			return scaled < 10
				? `${scaled.toFixed(1)}${suffix}`
				: `${Math.round(scaled)}${suffix}`;
		}
	}
	return String(n);
};

export const Stat = ({ label, value, warn }: StatProps) => (
	// `sunken`, not white. This tile had a bare `bg-white` with no dark
	// counterpart, so on a dark panel it was five glaring white cards with pale
	// text on them — unreadable, and the only part of the app that never got a
	// dark theme at all.
	<div className='flex min-w-0 flex-col items-center rounded-lg border border-line bg-sunken px-1.5 py-2'>
		{/* Loss and rejects have to be visible without being read: at a glance
		    the panel is either all neutral or it is not. */}
		<span
			title={value === undefined ? undefined : String(value)}
			className={`w-full truncate text-center text-base font-semibold tabular-nums ${
				warn ? 'text-danger' : 'text-ink'
			}`}
		>
			{value === undefined ? '—' : short(value)}
		</span>
		<span className='w-full truncate text-center text-[0.6rem] tracking-wide text-muted uppercase'>
			{label}
		</span>
	</div>
);
