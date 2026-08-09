import { PeerRow } from './PeerRow';

const Empty = ({ children }: { children: React.ReactNode }) => (
	<p className='rounded-lg border border-dashed border-line px-3 py-6 text-center text-sm text-muted'>
		{children}
	</p>
);

/// Every PC, with room to read it.
///
/// The home screen aims with a one-row strip; this is the other half of that
/// split — where the address, the live state and the slot number the shortcuts
/// use are all legible at once. Selecting somebody points voice and text at
/// that machine alone, and "Everyone" puts it back.
export const Roster = ({ peers, running, target, onTarget }: RosterProps) => {
	const live = peers.filter(p => p.live).length;

	return (
		<section className='flex flex-col gap-2'>
			<div className='flex items-baseline justify-between'>
				<h2 className='text-xs font-medium tracking-wide text-muted uppercase'>
					Send to
				</h2>
				<span className='text-xs text-faint'>
					{running ? `${live} of ${peers.length} live` : 'not looking'}
				</span>
			</div>

			<button
				type='button'
				onClick={() => onTarget(null)}
				className={`flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left font-medium transition ${
					target === null
						? 'border-accent bg-accent-soft'
						: 'border-line bg-surface hover:border-faint'
				}`}
			>
				Everyone
				<span className='ml-auto text-xs font-normal text-muted'>
					{live} live
				</span>
			</button>

			{!running && <Empty>Press Start to look for other PCs.</Empty>}

			{/* Discovery only runs while the transport does, so an empty roster
			    here means nobody answered — not that we have not looked yet. */}
			{running && peers.length === 0 && (
				<Empty>
					Nobody found yet. If this network blocks discovery, add an
					address with the + button.
				</Empty>
			)}

			{peers.map((p, i) => (
				<PeerRow
					key={p.id}
					{...{
						peer: p,
						selected: target === p.addr,
						onSelect: () => onTarget(p.addr),
						slot: i + 1
					}}
				/>
			))}
		</section>
	);
};
