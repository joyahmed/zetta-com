import { useEffect, useRef, useState } from 'react';

const time = (ms: number) =>
	new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

export const Messages = ({ messages, onSend, disabled, to }: MessagesProps) => {
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
		<section className='flex flex-col gap-2'>
			<h2 className='text-sm font-medium'>Messages</h2>

			<div className='flex max-h-56 min-h-24 flex-col gap-1 overflow-y-auto rounded-lg border border-slate-200 bg-white p-2 dark:border-slate-800 dark:bg-slate-900'>
				{messages.length === 0 && (
					<p className='m-auto text-sm text-slate-400 dark:text-slate-500'>
						Nothing yet.
					</p>
				)}
				{messages.map(m => (
					<div
						key={m.id}
						className={`flex gap-2 text-sm ${
							m.mine ? 'text-slate-500 dark:text-slate-400' : ''
						}`}
					>
						<span className='shrink-0 font-mono text-xs text-slate-400 dark:text-slate-500'>
							{time(m.at)}
						</span>
						<span className='shrink-0 font-medium'>
							{m.mine ? 'You' : m.from}
						</span>
						<span className='break-words'>{m.text}</span>
					</div>
				))}
				<div ref={end} />
			</div>

			<form className='flex gap-2' onSubmit={submit}>
				<input
					value={draft}
					onChange={e => setDraft(e.currentTarget.value)}
					disabled={disabled}
					placeholder={disabled ? 'Start to send messages' : `Message ${to}`}
					className='flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm outline-none transition placeholder:text-slate-400 focus:border-teal-500 focus:ring-2 focus:ring-teal-500/30 disabled:opacity-55 dark:border-slate-700 dark:bg-slate-900'
				/>
				<button
					type='submit'
					disabled={disabled}
					className='rounded-lg bg-teal-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-teal-500 disabled:opacity-55'
				>
					Send
				</button>
			</form>
		</section>
	);
};
