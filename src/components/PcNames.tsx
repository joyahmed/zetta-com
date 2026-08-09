import { useEffect, useState } from 'react';

/// Give a machine the name you actually use for it.
///
/// A PC name is what the machine calls itself — DEVS002 — and that is rarely
/// what you call the person sitting at it. Renaming here wins over both the
/// discovered name and the one carried in heartbeats, since it is the only one
/// somebody chose on purpose. Clearing it brings the discovered name back.
export const PcNames = ({ peers, onRename }: PcNamesProps) => {
	const [drafts, setDrafts] = useState<Record<string, string>>({});

	// Seeded from the roster, and re-seeded when a name arrives from the
	// network — otherwise a field you have not touched would sit showing an
	// address after the heartbeat that named it.
	useEffect(() => {
		setDrafts(d => {
			const next = { ...d };
			for (const p of peers) if (!(p.addr in next)) next[p.addr] = p.name;
			return next;
		});
	}, [peers]);

	if (peers.length === 0) {
		return (
			<p className='text-sm text-slate-500 dark:text-slate-400'>
				No PCs yet. Add one above, or start the transport and wait for
				discovery.
			</p>
		);
	}

	return (
		<div className='flex flex-col gap-2'>
			{peers.map(p => (
				<div key={p.id} className='flex items-center gap-2'>
					<input
						value={drafts[p.addr] ?? p.name}
						onChange={e =>
							setDrafts(d => ({
								...d,
								[p.addr]: e.currentTarget.value
							}))
						}
						// Saved on blur and on Enter rather than per keystroke:
						// every save restarts the transport, and doing that on
						// each letter would be absurd.
						onBlur={() => onRename(p.addr, drafts[p.addr] ?? '')}
						onKeyDown={e => {
							if (e.key === 'Enter') e.currentTarget.blur();
						}}
						className='flex-1 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm outline-none focus:border-teal-500 dark:border-slate-700 dark:bg-slate-900'
					/>
					<span className='w-40 shrink-0 truncate font-mono text-xs text-slate-400 dark:text-slate-500'>
						{p.addr}
					</span>
				</div>
			))}
		</div>
	);
};
