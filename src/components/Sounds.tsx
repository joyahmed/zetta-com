import { preview, type SoundName } from '../utils/sounds';

/// One row per sound: a switch, what it is for, and a way to hear it now.
///
/// The Test button matters more than it looks. Without it the only way to set
/// the volume is to wait for somebody to message you, and by then you are
/// listening to the message rather than to the sound.
const ROWS: { key: keyof Omit<Sounds, 'volume'>; label: string; hint: string; test: SoundName }[] = [
	{
		key: 'message',
		label: 'Message arrived',
		hint: 'Only while the window is hidden or behind something.',
		test: 'message'
	},
	{
		key: 'voice',
		label: 'Someone starts talking',
		hint: 'A short blip as their voice begins.',
		test: 'voice'
	},
	{
		key: 'presence',
		label: 'A watched PC goes offline or comes back',
		hint: 'Choose which with the bell beside each PC below.',
		test: 'offline'
	},
	{
		key: 'key',
		label: 'My talk key',
		hint: 'A tick on press and release. On speakers the room hears it too.',
		test: 'keyDown'
	}
];

export const Sounds = ({ sounds, onChoose }: SoundsProps) => (
	<div className='flex flex-col gap-3'>
		{ROWS.map(r => (
			<div key={r.key} className='flex items-start gap-2.5'>
				<label className='flex min-w-0 flex-1 cursor-pointer items-start gap-2.5'>
					<input
						type='checkbox'
						checked={sounds[r.key]}
						onChange={e => onChoose({ [r.key]: e.currentTarget.checked })}
						className='mt-0.5 size-4 shrink-0 accent-accent'
					/>
					<span className='flex min-w-0 flex-col'>
						<span className='text-sm text-ink'>{r.label}</span>
						<span className='text-xs text-muted'>{r.hint}</span>
					</span>
				</label>
				<button
					type='button'
					onClick={() => preview(r.test)}
					aria-label={`Play: ${r.label}`}
					className='shrink-0 rounded-lg border border-line px-2.5 py-1 text-xs text-muted transition hover:border-faint hover:text-ink'
				>
					Test
				</button>
			</div>
		))}

		<label className='flex items-center gap-3'>
			<span className='text-xs font-medium tracking-wide text-muted uppercase'>
				Volume
			</span>
			{/* Saved on release, not on every step of the drag: each save is a
			    write of the whole config file. */}
			<input
				type='range'
				min={0}
				max={100}
				step={5}
				defaultValue={Math.round(sounds.volume * 100)}
				key={sounds.volume}
				onPointerUp={e => onChoose({ volume: Number(e.currentTarget.value) / 100 })}
				onKeyUp={e => onChoose({ volume: Number(e.currentTarget.value) / 100 })}
				className='min-w-0 flex-1 accent-accent'
			/>
		</label>
	</div>
);
