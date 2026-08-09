import { useState } from 'react';

/// Machines added by hand.
///
/// The escape hatch, not the main road: discovery supplies peers on a normal
/// network, and this is for the ones that filter mDNS, and for a PC on another
/// subnet that will never be discovered at all.
export const Advanced = ({ manual, onAdd, onRemove }: AdvancedProps) => {
	const [draft, setDraft] = useState('');

	const submit = async (e: React.FormEvent) => {
		e.preventDefault();
		const addr = draft.trim();
		if (!addr) return;
		await onAdd(addr);
		setDraft('');
	};

	return (
		<div className='flex flex-col gap-3'>
			{/* Adding restarts the transport, so the change takes effect
			    immediately rather than at the next launch. */}
			<form className='flex gap-2' onSubmit={submit}>
				<input
					value={draft}
					onChange={e => setDraft(e.currentTarget.value)}
					placeholder='192.168.0.42:9001'
					className='flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm outline-none focus:border-teal-500 dark:border-slate-700 dark:bg-slate-900'
				/>
				<button
					type='submit'
					className='rounded-lg bg-teal-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-teal-500'
				>
					Add
				</button>
			</form>

			{manual.length === 0 ? (
				<p className='text-xs text-slate-500 dark:text-slate-400'>
					Nothing added by hand. Everything in the roster was found by
					discovery.
				</p>
			) : (
				manual.map(addr => (
					<div
						key={addr}
						className='flex items-center gap-2 rounded-lg border border-slate-200 px-3 py-1.5 dark:border-slate-800'
					>
						<span className='flex-1 truncate font-mono text-xs'>
							{addr}
						</span>
						<button
							type='button'
							onClick={() => onRemove(addr)}
							className='text-xs text-slate-500 hover:text-rose-600 dark:text-slate-400'
						>
							remove
						</button>
					</div>
				))
			)}
		</div>
	);
};
