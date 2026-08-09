import { PeerRow } from './PeerRow';

const Empty = ({ children }: { children: React.ReactNode }) => (
	<p className='rounded-lg border border-dashed border-slate-300 px-3 py-6 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400'>
		{children}
	</p>
);

export const Roster = ({ peers, running, selected, onSelect }: RosterProps) => {
	const live = peers.filter(p => p.live).length;

	return (
		<section className='flex flex-col gap-2'>
			<div className='flex items-baseline justify-between'>
				<h2 className='text-sm font-medium'>On the network</h2>
				<span className='text-xs text-slate-500 dark:text-slate-400'>
					{running ? `${live} of ${peers.length} live` : 'not looking'}
				</span>
			</div>

			{!running && <Empty>Press Start to look for other PCs.</Empty>}

			{/* Discovery only runs while the transport does, so an empty roster
			    here means nobody answered — not that we have not looked yet. */}
			{running && peers.length === 0 && (
				<Empty>
					Nobody found yet. If this network blocks discovery, add an
					address under Advanced.
				</Empty>
			)}

			{peers.map(p => (
				<PeerRow
					key={p.id}
					peer={p}
					selected={selected === p.addr}
					onSelect={() => onSelect(p.addr)}
				/>
			))}
		</section>
	);
};
