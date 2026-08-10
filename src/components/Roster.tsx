import { useVersion } from '../hooks/useVersion';
import { mismatched } from '../utils/version';
import { PeerRow } from './PeerRow';

const Empty = ({ children }: { children: React.ReactNode }) => (
	<p className='rounded-lg border border-dashed border-line px-3 py-6 text-center text-sm text-muted'>
		{children}
	</p>
);

/// Says when the network is running more than one build of the app.
///
/// Not an update notice — nothing here knows a release exists, and the app
/// never asks the internet. This only appears once somebody has already
/// updated a machine by hand, and it exists because the alternative was
/// watching a PC on an older build go silent with nothing anywhere saying why.
/// A wire format change is invisible from the outside: everything runs, the
/// counters move, and the other machine simply is not there.
const Builds = ({ odd, mine }: BuildsProps) => (
	<p className='rounded-lg border border-line bg-sunken px-3 py-2 text-xs text-muted'>
		<span className='text-ink'>Different builds on this network.</span>{' '}
		{odd.map(p => `${p.name} ${p.version}`).join(', ')} — you are on {mine}.
		Machines on different builds may stop hearing each other.
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
	const mine = useVersion();
	const odd = mismatched(peers, mine);

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

			{odd.length > 0 && <Builds {...{ odd, mine }} />}

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
