import { Dot } from './Dot';

export const PeerRow = ({ peer, selected, onSelect }: PeerRowProps) => (
	<button
		type='button'
		onClick={onSelect}
		className={`flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left transition ${
			selected
				? 'border-teal-500 bg-teal-50 dark:bg-teal-950/40'
				: 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900'
		}`}
	>
		<Dot on={peer.live} />
		<span className='flex-1 truncate font-medium'>{peer.name}</span>
		{peer.talking ? (
			<span className='rounded-full bg-teal-500 px-2 py-0.5 text-[0.65rem] font-medium tracking-wide text-white uppercase'>
				talking
			</span>
		) : (
			<span className='truncate font-mono text-xs text-slate-500 dark:text-slate-400'>
				{peer.addr}
			</span>
		)}
	</button>
);
