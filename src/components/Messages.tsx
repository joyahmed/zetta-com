import { useEffect, useRef, useState } from 'react';

// Always 24-hour, whatever the machine is set to. A locale that says "11:54 PM"
// is three characters wider than the column, so the stamp wrapped onto a second
// line and every message in the log became two rows tall. A fixed-width stamp is
// also what makes the column line up at all, which is the only reason to put the
// time in a column rather than inline.
//
// hourCycle rather than hour12: false, because the two are not the same request
// — hour12 selects h24, which renders midnight as "24:00".
const time = (ms: number) =>
	new Date(ms).toLocaleTimeString([], {
		hour: '2-digit',
		minute: '2-digit',
		hourCycle: 'h23'
	});

/// What has been said, and the box you say it in.
///
/// The only thing on the screen allowed to grow. Everything above it — the talk
/// bar, the target strip, the presets — has a fixed height and scrolls in its
/// own axis, so a busy conversation takes space from nothing and the box you
/// type in stays on the bottom edge where a chat window puts it.
export const Messages = ({
	messages,
	onSend,
	disabled,
	to,
	quick
}: MessagesProps) => {
	const [draft, setDraft] = useState('');
	const end = useRef<HTMLDivElement>(null);

	// Follow the newest line. Without this the log grows downward out of sight
	// and looks like nothing is arriving.
	useEffect(() => {
		end.current?.scrollIntoView({ block: 'end' });
	}, [messages.length]);

	const submit = (e: React.FormEvent) => {
		e.preventDefault();
		const text = draft.trim();
		if (!text) return;
		onSend(text);
		setDraft('');
	};

	return (
		<section className='flex min-h-0 flex-1 flex-col gap-2'>
			<div className='flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto rounded-xl border border-line bg-surface p-2'>
				{messages.length === 0 ? (
					<p className='m-auto px-6 text-center text-sm text-faint'>
						{disabled
							? 'Start the transport to send and receive.'
							: 'Nothing said yet.'}
					</p>
				) : (
					messages.map((m, i) => {
						// Consecutive lines from the same person drop the name.
						// A column of "You You You" is noise, and removing it
						// makes a change of speaker the thing that stands out.
						const who = m.mine ? 'You' : m.from;
						const prev = messages[i - 1];
						const same =
							prev && (prev.mine ? 'You' : prev.from) === who;

						return (
							<div
								key={m.id}
								className={`flex items-baseline gap-2 rounded-md px-1.5 py-0.5 text-sm ${
									same ? '' : 'mt-1.5 first:mt-0'
								}`}
							>
								{/* nowrap as well as a width: the width is what
								    aligns the column, and nowrap is what stops any
								    future stamp from silently buying a second row
								    the way the 12-hour one did. */}
								<span className='w-10 shrink-0 font-mono text-xs whitespace-nowrap text-faint tabular-nums'>
									{time(m.at)}
								</span>
								<span
									className={`w-16 shrink-0 truncate text-xs font-medium ${
										same
											? 'text-transparent'
											: m.mine
												? 'text-muted'
												: 'text-accent'
									}`}
								>
									{who}
								</span>
								<span
									className={`min-w-0 wrap-break-word ${
										m.mine ? 'text-muted' : 'text-ink'
									}`}
								>
									{m.text}
								</span>
							</div>
						);
					})
				)}
				<div ref={end} />
			</div>

			{quick}

			<form className='flex gap-2' onSubmit={submit}>
				<input
					value={draft}
					onChange={e => setDraft(e.currentTarget.value)}
					disabled={disabled}
					placeholder={disabled ? 'Start to send messages' : `Message ${to}`}
					className='min-w-0 flex-1 rounded-xl border border-line bg-surface px-3 py-2.5 text-sm text-ink outline-none transition placeholder:text-faint focus:border-accent disabled:opacity-50'
				/>
				<button
					type='submit'
					disabled={disabled}
					className='shrink-0 rounded-xl bg-accent px-4 py-2.5 text-sm font-medium text-on-accent transition hover:bg-accent-hover disabled:opacity-50'
				>
					Send
				</button>
			</form>
		</section>
	);
};
