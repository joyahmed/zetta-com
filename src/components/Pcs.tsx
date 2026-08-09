import { useState } from 'react';
import { Dot } from './Dot';

/// Every PC the app knows about, discovered or added manually.
///
/// A list you read, not a form you fill in. The first version of this made every
/// row a live text box that saved on blur, which meant the list looked like a
/// wall of identical fields, a stray click anywhere rebound the transport, and a
/// background poll could land in the middle of a name you were still typing.
///
/// Editing is deliberate now: one row at a time, with Save and Cancel, and the
/// draft held outside the row so a poll cannot reach it.
export const Pcs = ({
	peers,
	manual,
	onRename,
	onEdit,
	onRemove,
	onReorder
}: PcsProps) => {
	const [editing, setEditing] = useState<string | null>(null);
	const [name, setName] = useState('');
	const [addr, setAddr] = useState('');

	// Manual entries have to be merged in rather than read off the roster: the
	// roster is empty while the transport is stopped, and a PC you added should
	// still be there to edit when it is.
	const rows: PcRow[] = [
		...peers.map((p, i) => ({
			addr: p.addr,
			name: p.name,
			manual: p.manual,
			live: p.live,
			slot: i < 9 ? i + 1 : undefined
		})),
		...manual
			.filter(a => !peers.some(p => p.addr === a))
			.map(a => ({ addr: a, name: a, manual: true, live: false }))
	];

	/// Send the whole list in its new order, not just the pair that swapped.
	///
	/// Rust orders by position in this list and puts anything absent from it
	/// afterwards by name, so sending a partial list would silently drop every
	/// machine not mentioned to the bottom.
	const move = (i: number, by: number) => {
		const next = rows.map(r => r.addr);
		const to = i + by;
		if (to < 0 || to >= next.length) return;
		[next[i], next[to]] = [next[to], next[i]];
		onReorder(next);
	};

	const begin = (r: PcRow) => {
		setEditing(r.addr);
		setName(r.name);
		setAddr(r.addr);
	};

	/// Only what actually changed is sent. Each of these restarts the transport,
	/// so saving a row you opened and thought better of should cost nothing.
	const save = async (r: PcRow) => {
		const nextName = name.trim();
		const nextAddr = addr.trim();
		setEditing(null);
		if (nextName !== r.name) await onRename(r.addr, nextName);
		if (r.manual && nextAddr && nextAddr !== r.addr) {
			await onEdit(r.addr, nextAddr);
		}
	};

	if (rows.length === 0) {
		return (
			<p className='text-sm text-muted'>
				No PCs yet. Add one above, or start the transport and wait for
				discovery.
			</p>
		);
	}

	return (
		// Recessed against the panel it sits on. An outlined box with
		// transparent rows reads as an empty frame rather than as a list.
		<div className='overflow-hidden rounded-lg border border-line bg-sunken'>
			{rows.map((r, i) =>
				editing === r.addr ? (
					<div
						key={r.addr}
						className='flex flex-col gap-2 border-b border-line-soft bg-sunken p-2 last:border-0'
					>
						{/* Labelled, because two bare boxes one above the other
						    give no way to tell which is the name and which is
						    the address. The name is not monospaced and the
						    address is, which says the same thing again. */}
						<label className='flex flex-col gap-1'>
							<span className='text-xs font-medium tracking-wide text-muted uppercase'>
								Name
							</span>
							<input
								value={name}
								onChange={e => setName(e.currentTarget.value)}
								onKeyDown={e => {
									if (e.key === 'Enter') save(r);
									if (e.key === 'Escape') setEditing(null);
								}}
								autoFocus
								placeholder='A name you will recognise'
								className='rounded-lg border border-line bg-surface px-3 py-1.5 text-sm outline-none focus:border-accent'
							/>
						</label>

						{/* Only a manual address is editable. A discovered one
						    comes from the advertisement and would be overwritten
						    by the next announcement anyway. */}
						{r.manual ? (
							<label className='flex flex-col gap-1'>
								<span className='text-xs font-medium tracking-wide text-muted uppercase'>
									Address
								</span>
								<input
									value={addr}
									onChange={e => setAddr(e.currentTarget.value)}
									onKeyDown={e => {
										if (e.key === 'Enter') save(r);
										if (e.key === 'Escape') setEditing(null);
									}}
									placeholder='192.168.0.42:9001'
									className='rounded-lg border border-line bg-surface px-3 py-1.5 font-mono text-xs outline-none focus:border-accent'
								/>
							</label>
						) : (
							<p className='font-mono text-xs text-faint'>
								{r.addr} · found automatically, so the address is
								not yours to change
							</p>
						)}

						<div className='flex gap-2'>
							<button
								type='button'
								onClick={() => save(r)}
								className='rounded-lg bg-accent px-3 py-1.5 text-xs font-medium text-on-accent transition hover:bg-accent-hover'
							>
								Save
							</button>
							<button
								type='button'
								onClick={() => setEditing(null)}
								className='rounded-lg px-3 py-1.5 text-xs text-muted transition hover:bg-sunken'
							>
								Cancel
							</button>
							{r.manual && (
								<button
									type='button'
									onClick={() => {
										setEditing(null);
										onRemove(r.addr);
									}}
									className='ml-auto rounded-lg px-3 py-1.5 text-xs text-muted transition hover:bg-danger-soft hover:text-danger'
								>
									Remove
								</button>
							)}
						</div>
					</div>
				) : (
					<div
						key={r.addr}
						className='group flex items-center gap-2 border-b border-line-soft px-3 py-2 last:border-0'
					>
						{/* Up and down rather than dragging. The list is short,
						    the target is a whole row, and a drag that has to be
						    started, aimed and released is far easier to get
						    wrong with a trackpad than two arrows are. */}
						<div className='flex shrink-0 flex-col'>
							<button
								type='button'
								onClick={() => move(i, -1)}
								disabled={i === 0}
								aria-label={`Move ${r.name} up`}
								className='px-1 text-xs leading-none text-faint transition hover:text-ink disabled:opacity-25'
							>
								▲
							</button>
							<button
								type='button'
								onClick={() => move(i, 1)}
								disabled={i === rows.length - 1}
								aria-label={`Move ${r.name} down`}
								className='px-1 text-xs leading-none text-faint transition hover:text-ink disabled:opacity-25'
							>
								▼
							</button>
						</div>
						<Dot {...{ on: r.live }} />
						<div className='min-w-0 flex-1'>
							<p className='flex items-baseline gap-1.5 truncate text-sm font-medium'>
								{r.name}
								{/* Which Ctrl+n reaches this machine, shown
								    where you come to think about the machine. */}
								{r.slot !== undefined && (
									<span
										title={`Ctrl+${r.slot} to talk, Ctrl+Shift+${r.slot} to message`}
										className='font-mono text-[0.65rem] font-normal text-faint'
									>
										{r.slot}
									</span>
								)}
							</p>
							<p className='truncate font-mono text-xs text-faint'>
								{r.addr}
								{r.manual ? '' : ' · found automatically'}
							</p>
						</div>
						<button
							type='button'
							onClick={() => begin(r)}
							className='shrink-0 rounded-md px-2 py-1 text-xs text-muted transition hover:bg-sunken hover:text-ink'
						>
							Edit
						</button>
					</div>
				)
			)}
		</div>
	);
};
