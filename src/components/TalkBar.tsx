/// The one thing that has to be readable from across the room: whether your
/// microphone is open. Everything else on this screen can be squinted at.
///
/// `nobody` is a deleted group still selected. Said in the danger colour rather
/// than quietly, since pressing the key now reaches no one.
export const TalkBar = ({ held, key_, to, nobody }: TalkBarProps) => (
	<div
		className={`flex items-center justify-center gap-3 rounded-xl border px-4 py-3.5 transition ${
			held
				? 'border-accent bg-accent text-on-accent'
				: nobody
					? 'border-danger bg-danger-soft text-danger'
					: 'border-line bg-surface text-muted'
		}`}
	>
		<span
			className={`size-2.5 shrink-0 rounded-full ${
				held ? 'animate-pulse bg-on-accent' : 'bg-line'
			}`}
		/>
		{/* Wraps rather than truncates when nobody is aimed at: the reason is
		    the part that matters, and it is the part an ellipsis would cut. */}
		<span className={`text-sm font-medium ${nobody ? 'text-center' : 'truncate'}`}>
			{held ? `Talking to ${to}` : `Hold ${key_} to talk to ${to}`}
		</span>
	</div>
);
