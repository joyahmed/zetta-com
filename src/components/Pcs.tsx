import { useEffect, useState } from 'react';
import { Dot } from './Dot';

/// Every PC the app knows about, discovered or added manually, in one list.
///
/// These were two separate places before, and neither was complete: an
/// "Advanced" list that could add and remove an address but not correct one, and
/// a "Names" list below it that could rename a PC but not delete it. Fixing a
/// typo meant deleting the PC and typing the whole thing again. One row per PC
/// now owns all three.
export const Pcs = ({ peers, manual, onRename, onEdit, onRemove }: PcsProps) => {
	const [names, setNames] = useState<Record<string, string>>({});
	const [addrs, setAddrs] = useState<Record<string, string>>({});

	// Manual entries have to be merged in rather than read off the roster: the
	// roster is empty while the transport is stopped, and a PC you added should
	// still be there to edit when it is.
	const rows: PcRow[] = [
		...peers.map(p => ({
			addr: p.addr,
			name: p.name,
			manual: p.manual,
			live: p.live
		})),
		...manual
			.filter(a => !peers.some(p => p.addr === a))
			.map(a => ({ addr: a, name: a, manual: true, live: false }))
	];

	// Seeded from the roster, and re-seeded when a name arrives from the network
	// — otherwise a field you have not touched would sit showing an address
	// after the heartbeat that named it. Fields you *have* touched are left
	// alone, so a poll cannot overwrite what you are in the middle of typing.
	useEffect(() => {
		setNames(d => {
			const next = { ...d };
			for (const r of rows) if (!(r.addr in next)) next[r.addr] = r.name;
			return next;
		});
		setAddrs(d => {
			const next = { ...d };
			for (const r of rows) if (!(r.addr in next)) next[r.addr] = r.addr;
			return next;
		});
	}, [peers, manual]);

	/// Empty or unchanged puts the field back rather than sending a rebind that
	/// would change nothing.
	const commitAddr = (from: string) => {
		const to = (addrs[from] ?? '').trim();
		if (!to || to === from) {
			setAddrs(d => ({ ...d, [from]: from }));
			return;
		}
		onEdit(from, to);
	};

	if (rows.length === 0) {
		return (
			<p className='text-sm text-slate-500 dark:text-slate-400'>
				No PCs yet. Add one above, or start the transport and wait for
				discovery.
			</p>
		);
	}

	return (
		<div className='flex flex-col gap-2'>
			{rows.map(r => (
				<div
					key={r.addr}
					className='flex flex-col gap-1.5 rounded-lg border border-slate-200 p-2 dark:border-slate-800'
				>
					<div className='flex items-center gap-2'>
						<Dot {...{ on: r.live }} />
						<input
							value={names[r.addr] ?? r.name}
							onChange={e =>
								setNames(d => ({
									...d,
									[r.addr]: e.currentTarget.value
								}))
							}
							// Saved on blur and on Enter rather than per
							// keystroke: every save restarts the transport, and
							// doing that on each letter would be absurd.
							onBlur={() => onRename(r.addr, names[r.addr] ?? '')}
							onKeyDown={e => {
								if (e.key === 'Enter') e.currentTarget.blur();
							}}
							className='min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm outline-none focus:border-teal-500 dark:border-slate-700 dark:bg-slate-900'
						/>
						{/* Only manual entries can be deleted. A discovered PC
						    would be back in the roster within seconds, so the
						    button would be a lie. */}
						{r.manual && (
							<button
								type='button'
								onClick={() => onRemove(r.addr)}
								aria-label={`Remove ${r.name}`}
								className='shrink-0 rounded-md px-2 py-1 text-xs text-slate-500 transition hover:bg-rose-50 hover:text-rose-600 dark:text-slate-400 dark:hover:bg-rose-950/40'
							>
								Remove
							</button>
						)}
					</div>

					{r.manual ? (
						<input
							value={addrs[r.addr] ?? r.addr}
							onChange={e =>
								setAddrs(d => ({
									...d,
									[r.addr]: e.currentTarget.value
								}))
							}
							onBlur={() => commitAddr(r.addr)}
							onKeyDown={e => {
								if (e.key === 'Enter') e.currentTarget.blur();
							}}
							className='min-w-0 rounded-lg border border-slate-200 bg-white px-3 py-1 font-mono text-xs outline-none focus:border-teal-500 dark:border-slate-800 dark:bg-slate-900'
						/>
					) : (
						<p className='px-1 font-mono text-xs text-slate-400 dark:text-slate-500'>
							{r.addr} · found automatically
						</p>
					)}
				</div>
			))}
		</div>
	);
};
