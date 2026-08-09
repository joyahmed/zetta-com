import { PeerRow } from './PeerRow';

const Empty = ({ children }: { children: React.ReactNode }) => (
	<p className='rounded-lg border border-dashed border-slate-300 px-3 py-6 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400'>
		{children}
	</p>
);

/// Who you are addressing, not just who exists.
///
/// The roster is the control surface: selecting somebody points voice and text
/// at that machine alone, and "Everyone" puts it back. That is the difference
/// between an intercom and a way of telling one person something.
export const Roster = ({ peers, running, target, onTarget }: RosterProps) => {
	const live = peers.filter(p => p.live).length;

	return (
		<section className='flex flex-col gap-2'>
			<div className='flex items-baseline justify-between'>
				<h2 className='text-sm font-medium'>Send to</h2>
				<span className='text-xs text-slate-500 dark:text-slate-400'>
					{running ? `${live} of ${peers.length} live` : 'not looking'}
				</span>
			</div>

			<button
				type='button'
				onClick={() => onTarget(null)}
				className={`flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left font-medium transition ${
					target === null
						? 'border-teal-500 bg-teal-50 dark:bg-teal-950/40'
						: 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900'
				}`}
			>
				Everyone
				<span className='ml-auto text-xs font-normal text-slate-500 dark:text-slate-400'>
					{live} live
				</span>
			</button>

			{!running && <Empty>Press Start to look for other PCs.</Empty>}

			{/* Discovery only runs while the transport does, so an empty roster
			    here means nobody answered — not that we have not looked yet. */}
			{running && peers.length === 0 && (
				<Empty>
					Nobody found yet. If this network blocks discovery, add an
					address under Advanced.
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
