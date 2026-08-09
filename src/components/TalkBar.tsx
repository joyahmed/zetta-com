/// The one thing that has to be readable from across the room: whether your
/// microphone is open. Everything else on this screen can be squinted at.
export const TalkBar = ({ held, key_, to }: TalkBarProps) => (
	<div
		className={`flex items-center justify-center gap-3 rounded-xl border px-4 py-4 transition ${
			held
				? 'border-teal-500 bg-teal-500 text-white'
				: 'border-slate-200 bg-white text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400'
		}`}
	>
		<span
			className={`size-3 rounded-full ${
				held ? 'animate-pulse bg-white' : 'bg-slate-300 dark:bg-slate-700'
			}`}
		/>
		<span className='text-sm font-medium'>
			{held ? `Talking to ${to}` : `Hold ${key_} to talk to ${to}`}
		</span>
	</div>
);
