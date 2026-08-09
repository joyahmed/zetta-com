import { useState } from 'react';

/// The escape hatch for adding a PC discovery cannot reach.
///
/// Not the main road: discovery supplies peers on a normal network, and this is
/// for the ones that filter mDNS, and for a PC on another subnet that will never
/// be discovered at all. It only adds — the list below owns renaming, correcting
/// and removing, so there is one place to look for each of those.
export const AddPc = ({ onAdd }: AddPcProps) => {
	const [draft, setDraft] = useState('');

	const submit = async (e: React.FormEvent) => {
		e.preventDefault();
		const addr = draft.trim();
		if (!addr) return;
		await onAdd(addr);
		setDraft('');
	};

	return (
		// Adding restarts the transport, so the change takes effect immediately
		// rather than at the next launch.
		<form className='flex gap-2' onSubmit={submit}>
			<input
				value={draft}
				onChange={e => setDraft(e.currentTarget.value)}
				placeholder='192.168.0.42:9001'
				className='min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm outline-none focus:border-teal-500 dark:border-slate-700 dark:bg-slate-900'
			/>
			<button
				type='submit'
				className='shrink-0 rounded-lg bg-teal-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-teal-500'
			>
				Add
			</button>
		</form>
	);
};
