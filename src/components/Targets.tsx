import { GROUP } from '../hooks/useTarget';
import { Dot } from './Dot';
import { Moon } from './Moon';

/// Who you are addressing — one row, scrolling sideways.
///
/// This was a vertical list, and it was the thing that would have broken the
/// screen in use: the roster grows with every PC on the network and the message
/// log grows with every word said, and both were competing for the same
/// vertical space in a 460px panel. At ten machines the roster owned the window.
///
/// PCs grow along an axis nothing else uses now, and the log is the only thing
/// on the screen allowed to grow downwards. Kept visible rather than hidden
/// behind a tab, because "can they hear me" is the one question push-to-talk
/// must never make you go and look for.
const Chip = ({
	label,
	live,
	selected,
	onClick,
	title,
	slot,
	busy
}: {
	label: string;
	live: boolean;
	selected: boolean;
	onClick: () => void;
	title?: string;
	/// 1–9 for the machines that have a direct key, absent for the rest. Shown
	/// rather than left to the shortcut list: there are only nine number keys,
	/// so which PCs have one is a fact about the roster you are looking at, and
	/// it changes as machines come and go.
	slot?: number;
	/// On do-not-disturb. A small moon and nothing more: the strip is for
	/// aiming, and the note under the talk bar says what it means.
	busy?: boolean;
}) => (
	<button
		type='button'
		onClick={onClick}
		title={title}
		className={`flex shrink-0 items-center gap-2 rounded-full border px-3 py-1.5 text-sm transition ${
			selected
				? 'border-accent bg-accent-soft font-medium text-ink'
				: 'border-line bg-surface text-muted hover:border-faint hover:text-ink'
		}`}
	>
		<Dot {...{ on: live }} />
		<span className='max-w-32 truncate'>{label}</span>
		{busy && (
			<span className='text-faint' aria-label='busy'>
				<Moon {...{ size: 11 }} />
			</span>
		)}
		{slot !== undefined && (
			<span className='font-mono text-[0.65rem] text-faint'>{slot}</span>
		)}
	</button>
);

export const Targets = ({
	peers,
	groups,
	running,
	target,
	onTarget,
	onSeeAll
}: TargetsProps) => {
	const live = peers.filter(p => p.live).length;

	return (
		<div className='flex flex-col gap-1.5'>
			<div className='flex items-baseline justify-between px-0.5'>
				<h2 className='text-xs font-medium tracking-wide text-muted uppercase'>
					Send to
				</h2>
				<span className='text-xs text-faint'>
					{running ? `${live} of ${peers.length} live` : 'not looking'}
				</span>
			</div>

			{/* Contained rather than full-bleed: running the strip to the window
			    edge sliced whichever chip landed there clean in half against
			    the border.
			    No fade mask. There was one, and it dimmed the leading edge of
			    the first chip — "Everyone" arrived half-faded before anything
			    had been scrolled. A partly visible chip at the right is already
			    understood as "there is more", and needs no help. */}
			<div className='no-scrollbar flex gap-2 overflow-x-auto pb-0.5'>
				{/* Everyone is first and always present: it is the default the
				    app returns to, not one option among the machines. */}
				<Chip
					{...{
						label: 'Everyone',
						live: live > 0,
						selected: target === null,
						onClick: () => onTarget(null)
					}}
				/>

				{/* Groups next, before single PCs: picking a team is a broader
				    choice than picking a person, and the strip reads widest to
				    narrowest. Live when anyone in it is. */}
				{groups.map(g => {
					const members = peers.filter(p => g.members.includes(p.addr));
					const on = members.filter(p => p.live);
					return (
						<Chip
							key={GROUP + g.name}
							{...{
								label: g.name,
								live: on.length > 0,
								selected: target === GROUP + g.name,
								onClick: () => onTarget(GROUP + g.name),
								title: `${on.length} of ${g.members.length} live: ${members.map(p => p.name).join(', ')}`
							}}
						/>
					);
				})}

				{peers.map((p, i) => (
					<Chip
						key={p.id}
						{...{
							label: p.name,
							live: p.live,
							selected: target === p.addr,
							onClick: () => onTarget(p.addr),
							slot: i < 9 ? i + 1 : undefined,
							busy: p.live && p.busy,
							// The position is what Ctrl+Alt+n aims at, so it
							// belongs where you can check it without opening
							// the shortcut list.
							title: `${p.addr}${i < 9 ? ` · Ctrl+${i + 1}` : ''}${p.live && p.busy ? ' · busy (do not disturb)' : ''}`
						}}
					/>
				))}

				{/* Always offered, not only when the strip overflows: the full
				    list is also where liveness and addresses are legible, and a
				    control that appears and disappears is one nobody learns. */}
				<button
					type='button'
					onClick={onSeeAll}
					title='All PCs'
					className='shrink-0 rounded-full border border-dashed border-line px-3 py-1.5 text-sm text-muted transition hover:border-faint hover:text-ink'
				>
					{running && peers.length === 0 ? 'Nobody yet' : 'All'}
				</button>
			</div>
		</div>
	);
};
