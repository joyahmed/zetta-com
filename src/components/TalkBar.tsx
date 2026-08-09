/// The one thing that has to be readable from across the room: whether your
/// microphone is open. Everything else on this screen can be squinted at.
export const TalkBar = ({ held, key_, to }: TalkBarProps) => (
	<div
		className={`flex items-center justify-center gap-3 rounded-xl border px-4 py-3.5 transition ${
			held
				? 'border-accent bg-accent text-on-accent'
				: 'border-line bg-surface text-muted'
		}`}
	>
		<span
			className={`size-2.5 rounded-full ${
				held ? 'animate-pulse bg-on-accent' : 'bg-line'
			}`}
		/>
		<span className='truncate text-sm font-medium'>
			{held ? `Talking to ${to}` : `Hold ${key_} to talk to ${to}`}
		</span>
	</div>
);
