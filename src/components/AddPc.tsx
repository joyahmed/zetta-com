import { useState } from 'react';

/// The escape hatch for adding a PC discovery cannot reach.
///
/// Not the main road: discovery supplies peers on a normal network, and this is
/// for the ones that filter mDNS, and for a PC on another subnet that will never
/// be discovered at all.
///
/// It takes a name as well as an address. It used to take only the address, so
/// adding a machine meant typing `192.168.0.42:9001`, closing this, opening
/// Settings and renaming the row that appeared — three steps to do one thing, at
/// the one moment you actually know whose machine it is.
export const AddPc = ({ onAdd, onName }: AddPcProps) => {
	const [addr, setAddr] = useState('');
	const [name, setName] = useState('');
	const [busy, setBusy] = useState(false);

	const submit = async (e: React.FormEvent) => {
		e.preventDefault();
		const entry = addr.trim();
		if (!entry || busy) return;

		setBusy(true);
		try {
			// Added first. If the address will not resolve the add reports it
			// and no label is left behind pointing at a machine that was never
			// taken.
			await onAdd(entry);
			const label = name.trim();
			if (label) await onName(entry, label);
			setAddr('');
			setName('');
		} finally {
			setBusy(false);
		}
	};

	return (
		// Adding restarts the transport, so the change takes effect immediately
		// rather than at the next launch.
		<form className='flex flex-col gap-3' onSubmit={submit}>
			<label className='flex flex-col gap-1'>
				<span className='text-xs font-medium tracking-wide text-muted uppercase'>
					Address
				</span>
				<input
					value={addr}
					onChange={e => setAddr(e.currentTarget.value)}
					placeholder='192.168.0.42:9001'
					autoFocus
					className='rounded-lg border border-line bg-surface px-3 py-2 font-mono text-sm outline-none focus:border-accent'
				/>
			</label>

			<label className='flex flex-col gap-1'>
				<span className='text-xs font-medium tracking-wide text-muted uppercase'>
					Name
				</span>
				<input
					value={name}
					onChange={e => setName(e.currentTarget.value)}
					placeholder='Who is at that machine'
					className='rounded-lg border border-line bg-surface px-3 py-2 text-sm outline-none focus:border-accent'
				/>
				<span className='text-xs text-faint'>
					Optional. Left empty, the PC is listed by its address until
					it announces a name of its own.
				</span>
			</label>

			<button
				type='submit'
				disabled={busy || !addr.trim()}
				className='self-start rounded-lg bg-accent px-4 py-2 text-sm font-medium text-on-accent transition hover:bg-accent-hover disabled:opacity-50'
			>
				{busy ? 'Adding…' : 'Add'}
			</button>
		</form>
	);
};
