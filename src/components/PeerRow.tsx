import { Dot } from './Dot';
import { Moon } from './Moon';

export const PeerRow = ({ peer, selected, onSelect, slot }: PeerRowProps) => (
	<button
		type='button'
		onClick={onSelect}
		className={`flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left transition ${
			selected
				? 'border-accent bg-accent-soft'
				: 'border-line bg-surface hover:border-faint'
		}`}
	>
		<Dot {...{ on: peer.live }} />
		{/* The position is the shortcut: Ctrl+Alt+N holds to talk to this
		    machine, Ctrl+Shift+N aims at it and opens the window. Shown so
		    nobody has to learn which row is which. */}
		{slot !== undefined && slot <= 9 && (
			<span className='w-4 shrink-0 text-center font-mono text-xs text-faint'>
				{slot}
			</span>
		)}
		<span className='min-w-0 flex-1 truncate font-medium'>{peer.name}</span>
		{/* Beside the name rather than in place of the address: busy is about
		    the person, and the address is still how you tell two PCs apart. */}
		{peer.live && peer.busy && !peer.talking && (
			<span className='flex shrink-0 items-center gap-1 rounded-full border border-line bg-sunken px-2 py-0.5 text-[0.65rem] font-medium tracking-wide text-muted uppercase'>
				<Moon {...{ size: 10 }} />
				busy
			</span>
		)}
		{peer.talking ? (
			<span className='shrink-0 rounded-full bg-accent px-2 py-0.5 text-[0.65rem] font-medium tracking-wide text-on-accent uppercase'>
				talking
			</span>
		) : (
			<span className='shrink-0 truncate font-mono text-xs text-muted'>
				{peer.addr}
			</span>
		)}
	</button>
);
