import { useState } from 'react';

/// The shared passphrase, and the short code that proves everyone typed it.
///
/// One machine generates it, everyone else is given it, and from then on those
/// machines are a room. Without one the app works exactly as it always did, in
/// the clear — which is a real choice on a network you control, and is said
/// plainly here rather than being a quiet default.
///
/// The code is the part that stops this being miserable. A passphrase typed
/// wrongly on one machine is otherwise indistinguishable from a network fault:
/// everything runs, the roster is empty, nothing reports an error, because a
/// packet that does not authenticate is dropped without a reply. Two people
/// comparing four characters settle it in seconds.
export const Room = ({
	passphrase,
	code,
	onGenerate,
	onJoin
}: RoomProps) => {
	const [draft, setDraft] = useState('');
	const [shown, setShown] = useState(false);
	const [copied, setCopied] = useState(false);

	const copy = async () => {
		try {
			await navigator.clipboard.writeText(passphrase);
			setCopied(true);
			setTimeout(() => setCopied(false), 1500);
		} catch {
			// Clipboard refused. The passphrase is on screen behind Show, so
			// there is still a way to get it out.
		}
	};

	if (!passphrase) {
		return (
			<div className='flex flex-col gap-2'>
				<p className='rounded-lg border border-danger bg-danger-soft px-3 py-2 text-xs text-danger'>
					<strong>Not encrypted.</strong> Anyone on this network can
					listen to everything said, and send audio to you. They do not
					need this app and they will not appear in your roster.
				</p>
				<p className='text-xs text-muted'>
					Generate a passphrase on one PC, then set the same one on the
					others. Only machines with it can hear each other.
				</p>
				<div className='flex gap-2'>
					<button
						type='button'
						onClick={onGenerate}
						className='shrink-0 rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-on-accent transition hover:bg-accent-hover'
					>
						Generate
					</button>
					<input
						value={draft}
						onChange={e => setDraft(e.currentTarget.value)}
						onKeyDown={e => {
							if (e.key === 'Enter' && draft.trim()) {
								onJoin(draft.trim());
								setDraft('');
							}
						}}
						placeholder='or paste the one from another PC'
						className='min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 font-mono text-xs outline-none focus:border-accent'
					/>
				</div>
			</div>
		);
	}

	return (
		<div className='flex flex-col gap-2'>
			<div className='flex items-center gap-2 rounded-lg border border-line bg-sunken px-3 py-2'>
				<span className='text-xs text-muted'>Room</span>
				{/* Big, because its whole job is being compared across a desk. */}
				<span className='font-mono text-lg font-semibold tracking-widest text-accent'>
					{code ?? '····'}
				</span>
				<span className='ml-auto text-xs text-faint'>
					same on every PC
				</span>
			</div>

			<div className='flex gap-2'>
				<input
					readOnly
					value={shown ? passphrase : '•'.repeat(24)}
					onFocus={e => e.currentTarget.select()}
					className='min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 font-mono text-xs text-muted outline-none'
				/>
				<button
					type='button'
					onClick={() => setShown(v => !v)}
					className='shrink-0 rounded-lg border border-line px-2 py-1.5 text-xs text-muted transition hover:border-faint hover:text-ink'
				>
					{shown ? 'Hide' : 'Show'}
				</button>
				<button
					type='button'
					onClick={copy}
					className='shrink-0 rounded-lg border border-line px-2 py-1.5 text-xs text-muted transition hover:border-faint hover:text-ink'
				>
					{copied ? 'Copied' : 'Copy'}
				</button>
			</div>

			<div className='flex gap-2'>
				<input
					value={draft}
					onChange={e => setDraft(e.currentTarget.value)}
					onKeyDown={e => {
						if (e.key === 'Enter' && draft.trim()) {
							onJoin(draft.trim());
							setDraft('');
						}
					}}
					placeholder='paste a different passphrase to switch rooms'
					className='min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 font-mono text-xs outline-none focus:border-accent'
				/>
				{/* Leaving is destructive in a quiet way — everything goes back
				    to travelling in the clear — so it is not the loud button. */}
				<button
					type='button'
					onClick={() => onJoin('')}
					className='shrink-0 rounded-lg px-2 py-1.5 text-xs text-muted transition hover:bg-danger-soft hover:text-danger'
				>
					Leave
				</button>
			</div>
		</div>
	);
};
