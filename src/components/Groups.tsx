import { useState } from 'react';

/// Make and change saved groups.
///
/// One group open at a time. Every group expanded at once is a wall of
/// checkboxes, and the question being answered is always about one group.
export const Groups = ({ groups, peers, onSave }: GroupsProps) => {
	const [open, setOpen] = useState<number | null>(null);
	const [name, setName] = useState('');
	const [members, setMembers] = useState<string[]>([]);
	// Why the last save was refused. Shown in the editor, which stays open:
	// closing it threw away the name you typed along with the reason it failed.
	const [error, setError] = useState<string | null>(null);

	const begin = (i: number) => {
		setOpen(i);
		setName(groups[i]?.name ?? '');
		setMembers(groups[i]?.members ?? []);
		setError(null);
	};

	const close = () => {
		setOpen(null);
		setError(null);
	};

	/// Close only once Rust has kept it.
	const attempt = async (next: Group[]) => {
		try {
			await onSave(next);
			close();
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
		}
	};

	const commit = async () => {
		if (open === null) return;
		// The id comes along, so Rust knows this is the same group renamed and
		// the aim stays on it. A new group has none yet; Rust gives it one.
		const g = { id: groups[open]?.id ?? '', name: name.trim(), members };
		const next =
			open === groups.length
				? [...groups, g]
				: groups.map((x, i) => (i === open ? g : x));
		await attempt(next);
	};

	const toggle = (addr: string) =>
		setMembers(m => (m.includes(addr) ? m.filter(a => a !== addr) : [...m, addr]));

	// A member no longer in the roster is still a member — a PC that is off
	// today is still on the team — so it is listed by address under the rest.
	const extra = members.filter(m => !peers.some(p => p.addr === m));

	const editor = (
		<div className='flex flex-col gap-2 rounded-lg border border-line bg-sunken p-2'>
			<label className='flex flex-col gap-1'>
				<span className='text-xs font-medium tracking-wide text-muted uppercase'>
					Name
				</span>
				<input
					value={name}
					onChange={e => {
						setName(e.currentTarget.value);
						setError(null);
					}}
					onKeyDown={e => {
						if (e.key === 'Enter') commit();
						if (e.key === 'Escape') close();
					}}
					aria-invalid={error !== null}
					autoFocus
					placeholder='Devs, Front desk…'
					className={`rounded-lg border bg-surface px-3 py-1.5 text-sm outline-none ${
						error ? 'border-danger' : 'border-line focus:border-accent'
					}`}
				/>
				{error && (
					<span role='alert' className='text-xs text-danger'>
						{error}
					</span>
				)}
			</label>
			<div className='flex flex-col gap-1'>
				{peers.map(p => (
					<label key={p.addr} className='flex cursor-pointer items-center gap-2.5'>
						<input
							type='checkbox'
							checked={members.includes(p.addr)}
							onChange={() => toggle(p.addr)}
							className='size-4 shrink-0 accent-accent'
						/>
						<span className='truncate text-sm text-ink'>{p.name}</span>
					</label>
				))}
				{extra.map(a => (
					<label key={a} className='flex cursor-pointer items-center gap-2.5'>
						<input
							type='checkbox'
							checked
							onChange={() => toggle(a)}
							className='size-4 shrink-0 accent-accent'
						/>
						<span className='truncate font-mono text-xs text-faint'>{a} · not seen now</span>
					</label>
				))}
				{peers.length === 0 && extra.length === 0 && (
					<p className='text-xs text-muted'>No PCs yet. Start, and they appear here.</p>
				)}
			</div>
			<div className='flex gap-2'>
				<button
					type='button'
					onClick={commit}
					disabled={!name.trim()}
					className='rounded-lg bg-accent px-3 py-1.5 text-xs font-medium text-on-accent transition hover:bg-accent-hover disabled:opacity-50'
				>
					Save
				</button>
				<button
					type='button'
					onClick={close}
					className='rounded-lg px-3 py-1.5 text-xs text-muted transition hover:bg-surface'
				>
					Cancel
				</button>
				{open !== null && open < groups.length && (
					<button
						type='button'
						onClick={() => attempt(groups.filter((_, j) => j !== open))}
						className='ml-auto rounded-lg px-3 py-1.5 text-xs text-muted transition hover:bg-danger-soft hover:text-danger'
					>
						Delete
					</button>
				)}
			</div>
		</div>
	);

	return (
		<div className='flex flex-col gap-2'>
			{groups.map((g, i) =>
				open === i ? (
					<div key={g.id}>{editor}</div>
				) : (
					<div
						key={g.id}
						className='flex items-center gap-2 rounded-lg border border-line bg-sunken px-3 py-2'
					>
						<div className='min-w-0 flex-1'>
							<p className='truncate text-sm font-medium'>{g.name}</p>
							<p className='truncate text-xs text-faint'>
								{g.members
									.map(m => peers.find(p => p.addr === m)?.name ?? m)
									.join(', ') || 'Nobody in it yet'}
							</p>
						</div>
						<button
							type='button'
							onClick={() => begin(i)}
							className='shrink-0 rounded-md px-2 py-1 text-xs text-muted transition hover:bg-surface hover:text-ink'
						>
							Edit
						</button>
					</div>
				)
			)}
			{open === groups.length ? (
				editor
			) : (
				<button
					type='button'
					onClick={() => begin(groups.length)}
					className='self-start rounded-lg border border-dashed border-line px-3 py-1.5 text-xs text-muted transition hover:border-faint hover:text-ink'
				>
					New group
				</button>
			)}
			<p className='text-xs text-muted'>
				A group shows next to Everyone under Send to. Members who are offline
				are left out; the rest still get it.
			</p>
		</div>
	);
};
